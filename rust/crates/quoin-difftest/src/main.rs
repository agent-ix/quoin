// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-difftest` — the retained TypeScript and `quoin-core`, one request,
//! one verdict.
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
//! It is small on purpose. It covers the operations that exist; when Stage 1
//! lands `quoin-quire` it gains cases, not a framework.
//!
//! ```text
//! quoin-difftest --core rust/target/debug/quoin-core \
//!                --ts  scripts/core-reference.mjs
//! ```

use std::io::Write as _;
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

/// The Stage-0 case set: every exit status `core.ping` can reach.
///
/// A harness that only exercises the success path proves the two
/// implementations agree about success, which is not the interesting claim.
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
    // ── assurance.requirement_of (quoin#384) ──
    //
    // The first cases comparing a REAL retained capability. The TypeScript
    // side calls `src/assurance/graph.ts`'s own `requirementOf`; it does not
    // reimplement it. Everything above this line compares two implementations
    // of a purpose-built ping.
    Case {
        name: "assurance/ac-suffix",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001-AC-3"}"#),
    },
    Case {
        name: "assurance/metric-suffix",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"NFR-010-M-2"}"#),
    },
    Case {
        name: "assurance/unknown-suffix-is-still-a-prefix-match",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-042-QQQ-9"}"#),
    },
    Case {
        name: "assurance/bare-requirement-id",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001"}"#),
    },
    Case {
        name: "assurance/unrecognised-returns-unchanged",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"not-an-id"}"#),
    },
    Case {
        name: "assurance/empty-id",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":""}"#),
    },
    Case {
        name: "assurance/non-ascii-falls-to-unchanged",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"é-001-AC-1"}"#),
    },
    Case {
        name: "assurance/invalid-unknown-field",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001","extra":1}"#),
    },
    Case {
        name: "assurance/invalid-wrong-type",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":7}"#),
    },
    Case {
        name: "invalid/unknown-op",
        op: "evidence.record",
        request: Request::Literal("{}"),
    },
];

/// What one side produced.
struct Observed {
    stdout: String,
    diagnostics: Vec<(String, Vec<String>)>,
    status: i32,
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(core) = flag(&args, "--core") else {
        return usage("--core <path to quoin-core>");
    };
    let Some(ts) = flag(&args, "--ts") else {
        return usage("--ts <path to the TypeScript entry point>");
    };
    let node = flag(&args, "--node").unwrap_or_else(|| "node".to_owned());

    let mut failures = 0_usize;
    for case in CASES {
        let request = case.request.text();
        let rust = observe(Command::new(&core).arg(case.op), &request);
        let typescript = observe(Command::new(&node).arg(&ts).arg(case.op), &request);
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

    println!("\n{} case(s), {failures} difference(s)", CASES.len());
    if failures == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
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
/// byte diff from this harness.
fn canonicalise(stdout: &str) -> Result<String, String> {
    if stdout.trim().is_empty() {
        return Ok(String::new());
    }
    let value: serde_json::Value = serde_json::from_str(stdout).map_err(|e| e.to_string())?;
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
    eprintln!("usage: quoin-difftest --core <path> --ts <path> [--node <node>]");
    std::process::ExitCode::from(2)
}
