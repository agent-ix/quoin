// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for measurement collection publication (quoin#373, Stage 8).

mod tamper;

use std::io::Read as _;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use serde_json::Value;

use crate::core_runtime::invoke;

const DESCRIPTION: &str = "Measurement collections connect raw producer output to active\nMeasurementPlans. Use `quoin measurement record` to persist one complete producer\ninvocation; use `quoin report` for current state, comparisons, and series.";

/// The `measurement` command and its retained subcommands.
pub(crate) fn command() -> Command {
    Command::new("measurement")
        .about("Record and inspect versioned QA measurements")
        .subcommand(
            Command::new("record")
                .arg(repo())
                .arg(Arg::new("input").long("input").required(true))
                .arg(
                    Arg::new("digest-from-file")
                        .long("digest-from-file")
                        .value_name("FIELD=PATH")
                        .action(ArgAction::Append)
                        .help(
                            "Compute a verificationStack digest from a local file instead of \
                             pasting one by hand. FIELD is lockDigest, executableDigest, \
                             configDigest, or artifacts.<name>; repeatable. When the input \
                             already names a digest for FIELD, it must match the file's \
                             computed digest or the record is refused. A named \
                             verificationStack.artifacts entry that already resolves to a real \
                             file under --repo is truth-checked automatically without this \
                             flag; it is most useful for lockDigest, executableDigest and \
                             configDigest, which carry no path of their own to check \
                             automatically, and for an artifact reachable at a path other than \
                             its own name.",
                        ),
                ),
        )
        .subcommand(
            Command::new("verify")
                .about("Decide a plan's verdict from its stored collections, independent of the producer")
                .arg(repo())
                .arg(
                    Arg::new("plan")
                        .long("plan")
                        .required(true)
                        .value_name("MP-ID")
                        .help("The MeasurementPlan to verify."),
                )
                .arg(
                    Arg::new("claimed")
                        .long("claimed")
                        .value_parser(["accept", "reject", "inconclusive"])
                        .help(
                            "A verdict someone claims for the plan. The checker rejects when \
                             its own verdict differs.",
                        ),
                ),
        )
        .subcommand(producer("intervention"))
        .subcommand(producer("operational-release"))
}

fn repo() -> Arg {
    Arg::new("repo").long("repo").default_value(".")
}

fn producer(name: &'static str) -> Command {
    Command::new(name)
        .arg(repo())
        .arg(Arg::new("definition").long("definition").required(true))
}

/// Execute a parsed measurement command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let Some((name, arguments)) = matches.subcommand() else {
        return Ok(Response::ok(serde_json::json!({ "rendered": DESCRIPTION })));
    };
    match name {
        "record" => record(arguments),
        "verify" => verify(arguments),
        "intervention" => produce(
            arguments,
            "definition",
            "measurement.produce_agent_eval_intervention",
        ),
        "operational-release" => produce(
            arguments,
            "definition",
            "measurement.produce_github_release_operational",
        ),
        _ => Err("an unknown measurement command reached dispatch".to_owned()),
    }
}

fn record(arguments: &ArgMatches) -> Result<Response, String> {
    let input = required(arguments, "input")?;
    let mut record = json_input(&input, "measurement input")?;
    if let Some(specs) = arguments.get_many::<String>("digest-from-file") {
        for spec in specs {
            apply_digest_from_file(&mut record, spec)?;
        }
    }
    publish(
        "measurement.record",
        &serde_json::json!({ "repo": required(arguments, "repo")?, "record": record }),
    )
}

/// `quoin measurement verify` (FR-108, PLAT-961): the independent verdict
/// checker. Prints the verdict document; exits 1 when it is not `accept`.
///
/// PLAT-985 adds four git-derived tamper facts alongside the intake order,
/// gathered in [`tamper`] for the reason stated there: only this binary has
/// git, and the checker itself stays pure.
fn verify(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let plan = required(arguments, "plan")?;
    let order = intake_order(&repo)?;
    let (deleted, edited) = tamper::deleted_and_edited(&repo)?;
    let deleted: Vec<serde_json::Value> = deleted
        .into_iter()
        .map(|collection| {
            serde_json::json!({ "id": collection.id, "plan_ids": collection.plan_ids })
        })
        .collect();
    let request = serde_json::json!({
        "repo": repo,
        "plan": plan,
        "claimed": arguments.get_one::<String>("claimed"),
        "intake_order": order.groups,
        "order_source": order.source,
        "deleted": deleted,
        "edited_collections": edited,
        "apparatus_forged": tamper::forged_apparatus(&repo, &plan),
        "definition_changed_without_version_bump": tamper::definition_changed(&repo, &plan),
    });
    let mut response = invoke("measurement.verify", &request)?;
    if response.outcome.carries_payload() {
        let rendered = quoin_core::protocol::canonical_json(&response.payload)
            .map_err(|error| error.to_string())?;
        response.payload = serde_json::json!({ "rendered": rendered, "warning": order.warning });
    }
    Ok(response)
}

/// The intake order handed to the checker, where it came from, and what the
/// operator should be told about it.
struct IntakeOrder {
    groups: Vec<Vec<String>>,
    source: &'static str,
    warning: Option<String>,
}

/// Collection ids grouped by the first-parent git commit that added each file
/// under the measurement store, earliest first (FR-108-AC-2).
///
/// The store records no intake order, and a collection's own `timestamp` is
/// the producer's to choose. The commit that added the file on the
/// first-parent line is the most producer-independent order available:
/// changing it means rewriting published history, and following first
/// parents only means a side branch's own commit dates cannot reorder
/// collections merged together. `--no-renames` makes a moved file count as
/// added where it now lives.
///
/// A directory that is not a git work tree has no order, and says so. A
/// shallow clone's history is truncated, so its first-add commits are not
/// the real ones: the order is dropped and the checker reports
/// `order_unattested`. Any other git failure is an error, not a silent
/// absence of order.
fn intake_order(repo: &str) -> Result<IntakeOrder, String> {
    let git = |arguments: &[&str]| {
        std::process::Command::new("git")
            .args(["-C", repo])
            .args(arguments)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    };
    let inside = git(&["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Ok(IntakeOrder {
            groups: Vec::new(),
            source: "none",
            warning: Some(format!(
                "no intake order: {repo} is not a git work tree ({})",
                String::from_utf8_lossy(&inside.stderr).trim()
            )),
        });
    }
    let shallow = git(&["rev-parse", "--is-shallow-repository"])?;
    if String::from_utf8_lossy(&shallow.stdout).trim() == "true" {
        return Ok(IntakeOrder {
            groups: Vec::new(),
            source: "git-shallow",
            warning: Some(format!(
                "no intake order: {repo} is a shallow clone, so the commit that first added \
                 each collection is not known; fetch full history to attest the order"
            )),
        });
    }
    let log = git(&[
        "log",
        "--first-parent",
        "--reverse",
        "--topo-order",
        "--no-renames",
        "--diff-filter=A",
        "--relative",
        "--format=commit %H",
        "--name-only",
        "--",
        MEASUREMENTS_DIRECTORY,
    ])?;
    if !log.status.success() {
        return Err(format!(
            "git log over {MEASUREMENTS_DIRECTORY} failed: {}",
            String::from_utf8_lossy(&log.stderr).trim()
        ));
    }
    let mut groups: Vec<Vec<String>> = Vec::new();
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if line.starts_with("commit ") {
            groups.push(Vec::new());
        } else if let Some(id) = line
            .strip_prefix(MEASUREMENTS_DIRECTORY)
            .and_then(|rest| rest.strip_prefix('/'))
            .and_then(|name| name.strip_suffix(".json"))
            .filter(|id| !id.contains('/'))
            && let Some(group) = groups.last_mut()
        {
            group.push(id.to_owned());
        }
    }
    Ok(IntakeOrder {
        groups,
        source: "git-first-parent-add",
        warning: None,
    })
}

/// Where `quoin-measurement` publishes collections, relative to the
/// repository root (`quoin_measurement::store::measurements_root`).
const MEASUREMENTS_DIRECTORY: &str = "spec/evidence/measurements";

/// Apply one `--digest-from-file FIELD=PATH` (PLAT-931): digest the named
/// file and either fill FIELD in (it was absent or `null`) or, when the
/// caller also pasted a digest by hand, refuse before this record is
/// submitted if the two disagree. Hand-pasted digests were previously
/// checked for shape alone and never against the bytes they claim to name.
///
/// This is a convenience on top of, not a substitute for, the server-side
/// check: `quoin_measurement::store::write_measurement_collection` truth-checks
/// every `verificationStack.artifacts` entry whose name already resolves to a
/// real file under `--repo`, whether or not this flag is ever passed. This
/// flag exists for the fields that check cannot reach — `lockDigest`,
/// `executableDigest` and `configDigest` carry no path of their own — and for
/// an artifact reachable at a path other than the name it is stored under.
fn apply_digest_from_file(record: &mut Value, spec: &str) -> Result<(), String> {
    let Some((field, path)) = spec.split_once('=') else {
        return Err(format!(
            "--digest-from-file must be FIELD=PATH, got `{spec}`"
        ));
    };
    if field.is_empty() || path.is_empty() {
        return Err(format!(
            "--digest-from-file must be FIELD=PATH, got `{spec}`"
        ));
    }
    let digest = quoin_store::digest_file_sha256(Path::new(path))
        .map_err(|error| format!("cannot digest {field} from {path}: {error}"))?
        .to_stored();
    let field_path = digest_field_path(field)?;
    let slot = digest_slot(record, &field_path)?;
    match slot {
        Value::Null => *slot = Value::String(digest),
        Value::String(existing) if *existing == digest => {}
        Value::String(existing) => {
            return Err(format!(
                "{field} does not match {path}: the record says {existing}, the file digests \
                 to {digest}"
            ));
        }
        other => {
            return Err(format!(
                "{field} must be a string or absent; the record has {other}"
            ));
        }
    }
    Ok(())
}

/// The `record` JSON path `--digest-from-file`'s FIELD names.
fn digest_field_path(field: &str) -> Result<Vec<&str>, String> {
    match field {
        "configDigest" => Ok(vec!["configDigest"]),
        "lockDigest" => Ok(vec!["verificationStack", "lockDigest"]),
        "executableDigest" => Ok(vec!["verificationStack", "executableDigest"]),
        _ => field
            .strip_prefix("artifacts.")
            .filter(|name| !name.is_empty())
            .map_or_else(
                || {
                    Err(format!(
                        "--digest-from-file: unknown field `{field}` (expected lockDigest, \
                         executableDigest, configDigest, or artifacts.<name>)"
                    ))
                },
                |name| Ok(vec!["verificationStack", "artifacts", name]),
            ),
    }
}

/// Walk `path` inside `record`, creating any missing object along the way,
/// and return the leaf slot a digest is read from or written to.
///
/// # Errors
///
/// A segment along `path` whose value already exists and is not an object.
fn digest_slot<'a>(record: &'a mut Value, path: &[&str]) -> Result<&'a mut Value, String> {
    let Some((last, parents)) = path.split_last() else {
        return Err("--digest-from-file: empty field path".to_owned());
    };
    let mut cursor = record;
    for segment in parents {
        if cursor.is_null() {
            *cursor = Value::Object(serde_json::Map::new());
        }
        let Value::Object(object) = cursor else {
            return Err(format!(
                "--digest-from-file: `{segment}`'s parent is not an object"
            ));
        };
        cursor = object.entry((*segment).to_owned()).or_insert(Value::Null);
    }
    if cursor.is_null() {
        *cursor = Value::Object(serde_json::Map::new());
    }
    let Value::Object(object) = cursor else {
        return Err(format!(
            "--digest-from-file: `{last}`'s parent is not an object"
        ));
    };
    Ok(object.entry((*last).to_owned()).or_insert(Value::Null))
}

fn produce(arguments: &ArgMatches, field: &str, operation: &str) -> Result<Response, String> {
    let definition = json_input(&required(arguments, field)?, "measurement definition")?;
    publish(
        operation,
        &serde_json::json!({ "repo": required(arguments, "repo")?, "definition": definition }),
    )
}

fn publish(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    let mut response = invoke(operation, request)?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let path = response
        .payload
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{operation} returned no path"))?;
    response.payload = serde_json::json!({ "rendered": path });
    Ok(response)
}

fn json_input(path: &str, label: &str) -> Result<serde_json::Value, String> {
    let text = if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("cannot read {label} from stdin: {error}"))?;
        text
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {label}: {error}"))?
    };
    serde_json::from_str(&text).map_err(|error| format!("{label} is not JSON: {error}"))
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("{name} is required"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions report failures")]
mod tests {
    use super::{apply_digest_from_file, command, digest_field_path};

    /// Trace: FR-061, FR-102
    #[test]
    fn tc_373_measurement_preserves_the_retained_grammar() {
        assert!(command().try_get_matches_from(["measurement"]).is_ok());
        assert!(
            command()
                .try_get_matches_from(["measurement", "record", "--input", "record.json"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["measurement", "verify", "--plan", "MP-1"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "measurement",
                    "intervention",
                    "--definition",
                    "producer.json"
                ])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "measurement",
                    "operational-release",
                    "--definition",
                    "release.json"
                ])
                .is_ok()
        );
    }

    /// `--digest-from-file` is repeatable and parses alongside `record`'s
    /// existing flags.
    ///
    /// Trace: FR-102
    /// Provenance: PLAT-931
    #[test]
    fn tc_931_record_accepts_repeated_digest_from_file_flags() {
        let matches = command()
            .try_get_matches_from([
                "measurement",
                "record",
                "--input",
                "record.json",
                "--digest-from-file",
                "lockDigest=Cargo.lock",
                "--digest-from-file",
                "artifacts.config=config.json",
            ])
            .expect("two --digest-from-file flags parse");
        let (_, record) = matches.subcommand().expect("record ran");
        let specs: Vec<&String> = record
            .get_many::<String>("digest-from-file")
            .expect("both flags are present")
            .collect();
        assert_eq!(
            specs,
            ["lockDigest=Cargo.lock", "artifacts.config=config.json"]
        );
    }

    /// Every field `--digest-from-file` documents resolves; anything else is
    /// named and refused rather than silently ignored.
    ///
    /// Trace: FR-102
    /// Provenance: PLAT-931
    #[test]
    fn tc_931_digest_field_path_names_the_documented_surfaces() {
        assert_eq!(
            digest_field_path("configDigest").expect("configDigest resolves"),
            ["configDigest"]
        );
        assert_eq!(
            digest_field_path("lockDigest").expect("lockDigest resolves"),
            ["verificationStack", "lockDigest"]
        );
        assert_eq!(
            digest_field_path("executableDigest").expect("executableDigest resolves"),
            ["verificationStack", "executableDigest"]
        );
        assert_eq!(
            digest_field_path("artifacts.config").expect("artifacts.<name> resolves"),
            ["verificationStack", "artifacts", "config"]
        );
        assert!(digest_field_path("artifacts.").is_err());
        assert!(digest_field_path("bogus").is_err());
    }

    /// An absent digest is filled in from the file, so a caller need not paste
    /// one by hand at all.
    ///
    /// Trace: FR-102
    /// Provenance: PLAT-931
    #[test]
    fn tc_931_apply_digest_from_file_fills_an_absent_digest() {
        // The literal sha256 of `b"artifact bytes"`, not the re-derivation
        // `digest_file_sha256` would compute — asserting the function under
        // test's own output back at itself would prove nothing.
        let expected = "sha256:4659fc0570122b0e0aa14f4ff7c261b1fe51795a01ba79963f462ebf40d7520d";
        let file = tempfile::NamedTempFile::new().expect("a temporary file");
        std::fs::write(file.path(), b"artifact bytes").expect("the file is writable");

        let mut record = serde_json::json!({ "collectionId": "run-001" });
        apply_digest_from_file(
            &mut record,
            &format!("lockDigest={}", file.path().display()),
        )
        .expect("an absent digest is filled in");
        assert_eq!(
            record
                .pointer("/verificationStack/lockDigest")
                .and_then(serde_json::Value::as_str),
            Some(expected)
        );
    }

    /// A hand-pasted digest that agrees with the file's real bytes is left
    /// alone.
    ///
    /// Trace: FR-102
    /// Provenance: PLAT-931
    #[test]
    fn tc_931_apply_digest_from_file_accepts_a_matching_hand_pasted_digest() {
        // The literal sha256 of `b"artifact bytes"` — see the sibling test
        // above for why this is not `digest_file_sha256`'s own output.
        let expected = "sha256:4659fc0570122b0e0aa14f4ff7c261b1fe51795a01ba79963f462ebf40d7520d";
        let file = tempfile::NamedTempFile::new().expect("a temporary file");
        std::fs::write(file.path(), b"artifact bytes").expect("the file is writable");

        let mut record = serde_json::json!({ "configDigest": expected });
        apply_digest_from_file(
            &mut record,
            &format!("configDigest={}", file.path().display()),
        )
        .expect("a digest matching the file's real bytes is accepted");
        assert_eq!(
            record
                .pointer("/configDigest")
                .and_then(serde_json::Value::as_str),
            Some(expected)
        );
    }

    /// A hand-pasted digest that disagrees with the file's real bytes is
    /// refused before the record is ever submitted — closing the gap where a
    /// wrong-but-well-formed digest previously passed silently.
    ///
    /// Trace: FR-102
    /// Provenance: PLAT-931
    #[test]
    fn tc_931_apply_digest_from_file_refuses_a_mismatched_hand_pasted_digest() {
        let file = tempfile::NamedTempFile::new().expect("a temporary file");
        std::fs::write(file.path(), b"artifact bytes").expect("the file is writable");

        let wrong = format!("sha256:{}", "0".repeat(64));
        let mut record = serde_json::json!({
            "verificationStack": { "executableDigest": wrong },
        });
        let error = apply_digest_from_file(
            &mut record,
            &format!("executableDigest={}", file.path().display()),
        )
        .expect_err("a digest that does not match the file's real bytes is refused");
        assert!(
            error.contains("does not match"),
            "the refusal must say the two disagree; it said {error:?}"
        );
    }
}
