// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-difftest` — retained command contracts and `quoin-core`, one
//! candidate revision, one verdict.
//!
//! FR-101 says a capability is retired only after old and new implementations
//! passed at ONE candidate revision. This is the thing that checks that. It
//! feeds the same JSON request to both entry points and compares three facts:
//!
//! 1. **canonical stdout** — byte identity after key sorting;
//! 2. **exit status** — the taxonomy in [`quoin_core::protocol::Outcome`];
//! 3. **normalised diagnostic shape** — the ordered list of `(code, context
//!    keys)`. Deliberately NOT the message text: quoin#373 records that
//!    verdicts are contractual and error text is not, and comparing prose
//!    would make every reworded sentence a false parity failure.
//!
//! The protocol corpus retains the Stage 0 `core.ping` checks. The command
//! corpus, supplied as data beside the retained command fixtures, is the
//! non-vacuous FR-101 evidence: it invokes `bin/quoin.js` and the native
//! `quoin` executable over the same files and compares their terminal
//! contracts. Keeping the command descriptions out of this Rust source means
//! an implementation table cannot silently become the definition of what has
//! to be compared (quoin#424).
//!
//! ```text
//! quoin-difftest --core rust/target/debug/quoin-core \
//!                --ts scripts/core-reference.mjs \
//!                --native-cli rust/target/debug/quoin \
//!                --ts-cli bin/quoin.js \
//!                --command-cases rust/crates/quoin-difftest/fixtures/command-cases.json
//! ```

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use quoin_core::protocol::Outcome;

/// One request, run against both sides.
struct Case {
    name: &'static str,
    op: &'static str,
    request: Request,
}

/// How a case's stdin is produced.
///
/// A bound-breaching request is described rather than written out: a 4 KiB
/// literal in this table would be unreadable, and the interesting fact about
/// it is its size.
enum Request {
    /// Exactly these bytes.
    Literal(&'static str),
    /// `{"echo": "x" * n}`.
    EchoOfBytes(usize),
}

impl Request {
    fn text(&self) -> String {
        match *self {
            Self::Literal(text) => text.to_owned(),
            Self::EchoOfBytes(n) => format!(r#"{{"echo":"{}"}}"#, "x".repeat(n)),
        }
    }
}

/// The case set: every exit status `core.ping` can reach, plus an unknown
/// operation.
///
/// A harness that only exercises the success path proves the two
/// implementations agree about success, which is not the interesting claim.
///
/// **`core.ping` is the only operation left here, and that is the cutover
/// (quoin#445/#447), not an erosion of the gate.** A differential harness
/// needs two implementations of one contract. `src/assurance/` and
/// `src/completeness/` are gone, so for those domains there is no second
/// implementation to compare against, and FR-101-AC-5 forbids standing a
/// non-Rust oracle back up to manufacture one. `core.ping` keeps its place
/// because it never had a retained TypeScript capability to lose — Stage 0
/// wrote both sides on purpose, which is why it is the one operation a
/// difference between them still means something for.
///
/// What the 111 assurance cases asserted did not go with them: they are
/// replayed through the real binary from the frozen golden corpus in
/// `rust/crates/quoin-assurance/tests/golden/`, by
/// `rust/crates/quoin-core/tests/tc_447_assurance_boundary.rs`.
const CASES: &[Case] = &[
    Case {
        name: "ok/empty",
        op: "core.ping",
        request: Request::Literal("{}"),
    },
    Case {
        name: "ok/no-stdin",
        op: "core.ping",
        request: Request::Literal(""),
    },
    Case {
        name: "ok/echo",
        op: "core.ping",
        request: Request::Literal(r#"{"echo":"corr-7"}"#),
    },
    Case {
        name: "ok/echo-at-the-limit",
        op: "core.ping",
        request: Request::EchoOfBytes(4096),
    },
    Case {
        name: "ok/protocol-agrees",
        op: "core.ping",
        request: Request::Literal(r#"{"expect_protocol":1}"#),
    },
    Case {
        name: "partial/protocol-skew",
        op: "core.ping",
        request: Request::Literal(r#"{"expect_protocol":99}"#),
    },
    Case {
        name: "refused/echo-over-the-limit",
        op: "core.ping",
        request: Request::EchoOfBytes(4097),
    },
    Case {
        name: "invalid/unknown-field",
        op: "core.ping",
        request: Request::Literal(r#"{"eco":1}"#),
    },
    Case {
        name: "invalid/wrong-type",
        op: "core.ping",
        request: Request::Literal(r#"{"echo":7}"#),
    },
    Case {
        name: "invalid/malformed",
        op: "core.ping",
        request: Request::Literal("{oops"),
    },
    Case {
        name: "invalid/non-object",
        op: "core.ping",
        request: Request::Literal("[1,2]"),
    },
    Case {
        // A spelling no stage will ever claim. A not-yet-ported operation here
        // stops being an unknown op the moment its stage lands — quoin#458
        // broke this case with `evidence.record` — and what the case is for is
        // the unknown-op path, not the inventory of what exists today.
        name: "invalid/unknown-op",
        op: "core.no_such_operation",
        request: Request::Literal("{}"),
    },
];

/// What one side produced.
struct Observed {
    stdout: String,
    diagnostics: Vec<(String, Vec<String>)>,
    status: i32,
}

/// One real retained command contract loaded from the external corpus.
struct CommandCase {
    name: String,
    capability: String,
    args: Vec<String>,
}

/// What one command-line implementation emitted, without protocol rewriting.
struct CommandObserved {
    stdout: String,
    stderr: String,
    status: i32,
}

const COMMAND_CORPUS_SCHEMA_VERSION: u64 = 1;
const MIN_REAL_CAPABILITIES: usize = 3;
const MIN_REAL_COMMAND_CASES: usize = 6;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(core) = flag(&args, "--core") else {
        return usage("--core <path to quoin-core>");
    };
    let Some(ts) = flag(&args, "--ts") else {
        return usage("--ts <path to the TypeScript entry point>");
    };
    let Some(native_cli) = flag(&args, "--native-cli") else {
        return usage("--native-cli <path to native quoin>");
    };
    let Some(ts_cli) = flag(&args, "--ts-cli") else {
        return usage("--ts-cli <path to bin/quoin.js>");
    };
    let Some(command_cases_path) = flag(&args, "--command-cases") else {
        return usage("--command-cases <path to retained command corpus>");
    };
    let node = flag(&args, "--node").unwrap_or_else(|| "node".to_owned());

    let mut failures = run_protocol_cases(&core, &ts, &node);
    let command_cases = match load_command_cases(Path::new(&command_cases_path)) {
        Ok(cases) => cases,
        Err(message) => {
            println!("  INCONCLUSIVE command corpus: {message}");
            return std::process::ExitCode::from(2);
        }
    };
    let capabilities = command_cases
        .iter()
        .map(|case| case.capability.as_str())
        .collect::<BTreeSet<_>>();
    if capabilities.len() < MIN_REAL_CAPABILITIES || command_cases.len() < MIN_REAL_COMMAND_CASES {
        println!(
            "  INCONCLUSIVE command corpus: {} real capability/capabilities and {} case(s); minimum is {MIN_REAL_CAPABILITIES} and {MIN_REAL_COMMAND_CASES}",
            capabilities.len(),
            command_cases.len(),
        );
        return std::process::ExitCode::from(2);
    }
    let Some(repo_root) = repository_root(Path::new(&command_cases_path)) else {
        println!(
            "  INCONCLUSIVE command corpus: cannot locate the candidate repository root from {command_cases_path}"
        );
        return std::process::ExitCode::from(2);
    };
    failures += run_command_cases(
        &command_cases,
        &native_cli,
        &ts_cli,
        &core,
        &node,
        &repo_root,
    );

    println!(
        "\n{} protocol case(s); {} real command capability/capabilities across {} case(s); {failures} difference(s)",
        CASES.len(),
        capabilities.len(),
        command_cases.len(),
    );
    if failures == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

/// Locate the worktree owning a checked-in command corpus.
///
/// Cases contain `${REPO_ROOT}` rather than a developer's absolute checkout
/// path. Looking upward from the corpus rather than using the process CWD keeps
/// the comparison at the same candidate even when Cargo launched it from
/// `rust/` (quoin#424).
fn repository_root(corpus: &Path) -> Option<std::path::PathBuf> {
    corpus
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
}

fn run_protocol_cases(core: &str, ts: &str, node: &str) -> usize {
    let mut failures = 0_usize;
    for case in CASES {
        let request = case.request.text();
        let rust = observe(Command::new(core).arg(case.op), &request);
        let typescript = observe(Command::new(node).arg(ts).arg(case.op), &request);
        match (rust, typescript) {
            (Ok(rust), Ok(typescript)) => {
                let differences = compare(&rust, &typescript);
                if differences.is_empty() {
                    println!("  ok   {} [{}]", case.name, describe(rust.status));
                } else {
                    failures += 1;
                    println!("  DIFF {}", case.name);
                    for difference in differences {
                        println!("         {difference}");
                    }
                }
            }
            (rust, typescript) => {
                failures += 1;
                println!("  ERR  {}", case.name);
                for (side, result) in [("quoin-core", rust), ("typescript", typescript)] {
                    if let Err(message) = result {
                        println!("         {side} could not be run: {message}");
                    }
                }
            }
        }
    }
    failures
}

/// Load the retained command corpus and reject a malformed or empty shape.
fn load_command_cases(path: &Path) -> Result<Vec<CommandCase>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let document: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let root = document
        .as_object()
        .ok_or_else(|| "command corpus must be a JSON object".to_owned())?;
    let schema_version = root
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "command corpus has no integer schema_version".to_owned())?;
    if schema_version != COMMAND_CORPUS_SCHEMA_VERSION {
        return Err(format!(
            "command corpus schema_version {schema_version} is not {COMMAND_CORPUS_SCHEMA_VERSION}"
        ));
    }
    let cases = root
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "command corpus has no cases array".to_owned())?;
    cases
        .iter()
        .enumerate()
        .map(|(index, value)| command_case(value, index))
        .collect()
}

fn command_case(value: &serde_json::Value, index: usize) -> Result<CommandCase, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("command corpus case {index} is not an object"))?;
    let text = |field: &str| {
        object
            .get(field)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("command corpus case {index} has no non-empty {field}"))
    };
    let args = object
        .get("args")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("command corpus case {index} has no args array"))?
        .iter()
        .enumerate()
        .map(|(argument_index, argument)| {
            argument.as_str().map(str::to_owned).ok_or_else(|| {
                format!("command corpus case {index} arg {argument_index} is not a string")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if args.is_empty() {
        return Err(format!(
            "command corpus case {index} has no command arguments"
        ));
    }
    Ok(CommandCase {
        name: text("name")?,
        capability: text("capability")?,
        args,
    })
}

fn run_command_cases(
    cases: &[CommandCase],
    native_cli: &str,
    ts_cli: &str,
    core: &str,
    node: &str,
    repo_root: &Path,
) -> usize {
    let root = repo_root.to_string_lossy();
    let mut failures = 0_usize;
    for case in cases {
        let arguments = case
            .args
            .iter()
            .map(|argument| argument.replace("${REPO_ROOT}", &root))
            .collect::<Vec<_>>();
        let native = observe_command(Command::new(native_cli).args(&arguments));
        let retained = observe_command(
            Command::new(node)
                .arg(ts_cli)
                .args(&arguments)
                .env("QUOIN_CORE", core),
        );
        match (native, retained) {
            (Ok(native), Ok(retained)) => {
                let differences = compare_command(&native, &retained);
                if differences.is_empty() {
                    println!("  ok   {} [{}]", case.name, describe(native.status));
                } else {
                    failures += 1;
                    println!("  DIFF {} ({})", case.name, case.capability);
                    for difference in differences {
                        println!("         {difference}");
                    }
                }
            }
            (native, retained) => {
                failures += 1;
                println!("  ERR  {} ({})", case.name, case.capability);
                for (side, result) in [("native", native), ("retained TypeScript", retained)] {
                    if let Err(message) = result {
                        println!("         {side} could not be run: {message}");
                    }
                }
            }
        }
    }
    failures
}

/// Compare terminal command contracts without reformatting either stream.
///
/// Unlike the IPC protocol comparison below, command stdout can be Markdown or
/// human text. There is consequently no JSON re-encoding and no diagnostic
/// message elision here: bytes and stream placement are the retained command
/// contract. Build-version/platform fields are deliberately absent from this
/// deterministic corpus rather than normalised after the fact.
fn compare_command(native: &CommandObserved, retained: &CommandObserved) -> Vec<String> {
    let mut out = Vec::new();
    if native.status != retained.status {
        out.push(format!(
            "exit status: native {} ({}), retained TypeScript {} ({})",
            native.status,
            describe(native.status),
            retained.status,
            describe(retained.status)
        ));
    }
    if native.stdout != retained.stdout {
        out.push(format!(
            "stdout:\n           native              {}\n           retained TypeScript {}",
            show(&native.stdout),
            show(&retained.stdout)
        ));
    }
    if native.stderr != retained.stderr {
        out.push(format!(
            "stderr:\n           native              {}\n           retained TypeScript {}",
            show(&native.stderr),
            show(&retained.stderr)
        ));
    }
    out
}

fn observe_command(command: &mut Command) -> Result<CommandObserved, String> {
    let output = command.output().map_err(|error| error.to_string())?;
    Ok(CommandObserved {
        stdout: String::from_utf8(output.stdout).map_err(|error| error.to_string())?,
        stderr: String::from_utf8(output.stderr).map_err(|error| error.to_string())?,
        status: output
            .status
            .code()
            .ok_or("terminated by a signal, not a status")?,
    })
}

fn compare(rust: &Observed, typescript: &Observed) -> Vec<String> {
    let mut out = Vec::new();
    if rust.status != typescript.status {
        out.push(format!(
            "exit status: quoin-core {} ({}), typescript {} ({})",
            rust.status,
            describe(rust.status),
            typescript.status,
            describe(typescript.status)
        ));
    }
    if rust.stdout != typescript.stdout {
        out.push(format!(
            "canonical stdout:\n           quoin-core  {}\n           typescript  {}",
            show(&rust.stdout),
            show(&typescript.stdout)
        ));
    }
    if rust.diagnostics != typescript.diagnostics {
        out.push(format!(
            "diagnostic shape: quoin-core {:?}, typescript {:?}",
            rust.diagnostics, typescript.diagnostics
        ));
    }
    out
}

/// Run one side and reduce it to the three comparable facts.
fn observe(command: &mut Command, request: &str) -> Result<Observed, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .as_mut()
        .ok_or("stdin was not piped")?
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let stderr = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
    Ok(Observed {
        stdout: canonicalise(&stdout)?,
        diagnostics: normalise_diagnostics(&stderr)?,
        status: output
            .status
            .code()
            .ok_or("terminated by a signal, not a status")?,
    })
}

/// Re-canonicalise whatever a side wrote, so the comparison is about content.
///
/// Both sides are REQUIRED to emit canonical JSON, and the test suites on each
/// side assert that separately. Parsing here means a formatting difference is
/// reported by those tests, by name, instead of arriving as an unreadable
/// byte diff from this harness. The one erased field is `core_version`: it is
/// a build-version marker, not a protocol decision, and `quoin-core` and the
/// retained reference intentionally derive it from independent build metadata.
fn canonicalise(stdout: &str) -> Result<String, String> {
    if stdout.trim().is_empty() {
        return Ok(String::new());
    }
    let mut value: serde_json::Value = serde_json::from_str(stdout).map_err(|e| e.to_string())?;
    if let Some(object) = value.as_object_mut() {
        object.remove("core_version");
    }
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

/// `(code, sorted context keys)` per diagnostic, in order. Messages dropped.
fn normalise_diagnostics(stderr: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    if stderr.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(stderr).map_err(|e| format!("stderr is not a JSON array: {e}"))?;
    entries
        .iter()
        .map(|entry| {
            let code = entry
                .get("code")
                .and_then(serde_json::Value::as_str)
                .ok_or("diagnostic has no code")?;
            let context = entry
                .get("context")
                .and_then(serde_json::Value::as_object)
                .map(|map| map.keys().cloned().collect())
                .unwrap_or_default();
            Ok((code.to_owned(), context))
        })
        .collect()
}

fn describe(status: i32) -> String {
    u8::try_from(status)
        .ok()
        .and_then(Outcome::from_code)
        .map_or_else(
            || format!("status {status} is outside the taxonomy"),
            |o| format!("{o:?}"),
        )
}

fn show(text: &str) -> &str {
    if text.is_empty() {
        "<no payload>"
    } else {
        text
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn usage(missing: &str) -> std::process::ExitCode {
    eprintln!("quoin-difftest: missing {missing}");
    eprintln!(
        "usage: quoin-difftest --core <path> --ts <path> --native-cli <path> --ts-cli <path> --command-cases <path> [--node <node>]"
    );
    std::process::ExitCode::from(2)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a corpus assertion must panic to show which retained command contract disappeared"
)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::{
        MIN_REAL_CAPABILITIES, MIN_REAL_COMMAND_CASES, load_command_cases, repository_root,
    };

    /// Trace: FR-101
    /// Provenance: quoin#424
    #[test]
    fn tc_424_command_corpus_is_non_vacuous_and_names_three_real_graph_routes() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/command-cases.json");
        let cases = load_command_cases(&path).expect("the checked-in command corpus parses");
        let capabilities = cases
            .iter()
            .map(|case| case.capability.as_str())
            .collect::<BTreeSet<_>>();
        assert!(
            capabilities.len() >= MIN_REAL_CAPABILITIES,
            "the FR-101 corpus must compare real command capabilities"
        );
        assert!(
            cases.len() >= MIN_REAL_COMMAND_CASES,
            "the FR-101 corpus must have both human and JSON contracts"
        );
        for route in ["graph.change-impact", "graph.churn", "graph.fan-out"] {
            assert!(
                capabilities.contains(route),
                "the first real command slice must retain {route}"
            );
        }
        assert_eq!(
            repository_root(&path),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(3)
                .map(PathBuf::from),
            "the corpus resolves its own candidate root instead of the caller's CWD"
        );
    }
}
