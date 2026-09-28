// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The retained read-only `quoin evidence audit` adapter.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::{Outcome, Response};

use super::{json_arg, repo_arg, required, revision};
use crate::core_runtime::invoke;

pub(super) fn command() -> Command {
    Command::new("audit")
        .about("Audit the evidence store: suspect links, staleness, vacuity")
        .arg(repo_arg())
        .arg(module_arg())
        .arg(
            Arg::new("ratchet")
                .long("ratchet")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
        .arg(json_arg())
        .args(policy_args())
}

/// `--module`: the traceability model's roots, shared with `quoin matrix`.
pub(crate) fn module_arg() -> Arg {
    Arg::new("module").long("module").action(ArgAction::Append)
}

/// The auditor's policy inputs, shared with `quoin matrix` so both commands
/// read one flag grammar into one [`assemble`].
pub(crate) fn policy_args() -> [Arg; 3] {
    [
        Arg::new("multiplicity-requires")
            .long("multiplicity-requires")
            .action(ArgAction::Append),
        Arg::new("mutation-floor")
            .long("mutation-floor")
            .action(ArgAction::Append),
        Arg::new("independence-policy").long("independence-policy"),
    ]
}

/// An audit, with the two inputs a caller joins it against.
pub(crate) struct Audited {
    /// The `quire.coverage` payload the audit's obligations came from.
    pub(crate) coverage: serde_json::Value,
    /// The binding graph the audit read, as `evidence.audit_inputs` served it.
    pub(crate) bindings: serde_json::Value,
    /// The `auditor.audit` response, whose payload carries `report`.
    pub(crate) response: Response,
}

/// What [`assemble`] produced.
pub(crate) enum Assembly {
    /// Every step answered.
    Audited(Audited),
    /// A step declined; its response is the command's answer.
    Declined(Response),
}

/// Assemble and run the audit: coverage, the store, the catalog, the policy.
///
/// The one assembly `quoin evidence audit` and `quoin matrix` share, so the
/// two commands cannot audit the same working tree differently.
pub(crate) fn assemble(
    arguments: &ArgMatches,
    repo: &str,
    head: Option<&str>,
    accepted: Option<&serde_json::Value>,
) -> Result<Assembly, String> {
    let modules = values(arguments, "module");
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": modules }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Ok(Assembly::Declined(coverage));
    }
    let obligations = field(&coverage.payload, "obligations")?;
    let policy = policy(arguments, &obligations)?;
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "head_commit": head, "independence_policy": policy }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(Assembly::Declined(store));
    }
    let catalog_request = if modules.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::json!({ "roots": modules })
    };
    let catalog = invoke("catalog.methods", &catalog_request)?;
    if !catalog.outcome.carries_payload() {
        return Ok(Assembly::Declined(catalog));
    }
    let bindings = field(&store.payload, "bindings")?;
    let input = serde_json::json!({
        "obligations": obligations,
        "bindings": bindings,
        "runs": field(&store.payload, "runs")?,
        "scans": field(&store.payload, "scans")?,
        "injections": field(&store.payload, "injections")?,
        "mockInspectionSuites": field(&store.payload, "mock_inspection_suites")?,
        "vacuousScanSuites": field(&store.payload, "vacuous_scan_suites")?,
        "independence": field(&store.payload, "independence")?,
        "catalog": catalog.payload,
        "headCommit": head,
        "multiplicityRequires": values(arguments, "multiplicity-requires"),
        "mutationFloor": mutation_floor(&values(arguments, "mutation-floor"))?,
        "independencePolicy": policy,
    });
    let request = serde_json::json!({ "input": input, "accepted": accepted });
    let audited = invoke("auditor.audit", &request)?;
    if !audited.outcome.carries_payload() {
        return Ok(Assembly::Declined(audited));
    }
    Ok(Assembly::Audited(Audited {
        coverage: coverage.payload,
        bindings,
        response: audited,
    }))
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let head = revision(&repo);
    let head = (!head.is_empty()).then_some(head.as_str());
    let (accepted, missing_baseline) = baseline(arguments, &repo)?;
    let audited = match assemble(arguments, &repo, head, accepted.as_ref())? {
        Assembly::Audited(audited) => audited.response,
        Assembly::Declined(response) => return Ok(response),
    };
    let report = audited
        .payload
        .get("report")
        .ok_or_else(|| "auditor.audit did not return report".to_owned())?;
    let reported = audited
        .payload
        .get("reported")
        .or_else(|| report.get("findings"))
        .ok_or_else(|| "auditor.audit did not return findings".to_owned())?;
    let unevaluated = report
        .get("unevaluated")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let findings = reported.as_array().map_or(0, Vec::len);
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&serde_json::json!({
            "findings": reported,
            "healthy": report.get("healthy"),
            "unevaluated": report.get("unevaluated"),
            "ratchet": audited.payload.get("reported").is_some(),
            "independence": audited.payload.get("independence"),
        }))
        .map_err(|error| error.to_string())?
    } else {
        render_text(
            report,
            reported,
            audited.payload.get("reported").is_some(),
            missing_baseline,
        )?
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: audited.diagnostics,
        outcome: if arguments.get_flag("strict") && (findings > 0 || unevaluated > 0) {
            Outcome::Partial
        } else {
            audited.outcome
        },
    })
}

fn policy(
    arguments: &ArgMatches,
    obligations: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(path) = arguments.get_one::<String>("independence-policy") else {
        return Ok(serde_json::Value::Null);
    };
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let known_obligations = obligations
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    let parsed = invoke(
        "evidence.parse_policy",
        &serde_json::json!({ "text": text, "known_obligations": known_obligations }),
    )?;
    if !parsed.outcome.carries_payload() {
        return Err("independence policy was refused".to_owned());
    }
    field(&parsed.payload, "policy")
}

fn baseline(
    arguments: &ArgMatches,
    repo: &str,
) -> Result<(Option<serde_json::Value>, bool), String> {
    if !arguments.get_flag("ratchet") {
        return Ok((None, false));
    }
    let response = invoke(
        "evidence.read_baseline",
        &serde_json::json!({ "repo": repo }),
    )?;
    if !response.outcome.carries_payload() {
        return Ok((None, false));
    }
    let Some(file) = response
        .payload
        .get("baseline")
        .filter(|value| !value.is_null())
    else {
        return Ok((None, true));
    };
    Ok((Some(field(file, "accepted")?), false))
}

fn mutation_floor(entries: &[String]) -> Result<serde_json::Value, String> {
    if entries.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    let mut floor = serde_json::Map::new();
    for entry in entries {
        let Some((criticality, raw)) = entry.split_once('=') else {
            return Err(format!(
                "--mutation-floor expects <criticality>=<score>, got '{entry}'. Example: --mutation-floor P0=0.8"
            ));
        };
        let score = raw.parse::<f64>().map_err(|_| format!("--mutation-floor expects <criticality>=<score>, got '{entry}'. Example: --mutation-floor P0=0.8"))?;
        if criticality.is_empty() || !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "--mutation-floor score must be a ratio in [0, 1], got '{raw}'. 80% is 0.8, not 80."
            ));
        }
        floor.insert(criticality.to_owned(), serde_json::json!(score));
    }
    Ok(serde_json::Value::Object(floor))
}

fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}

fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}

fn render_text(
    report: &serde_json::Value,
    reported: &serde_json::Value,
    ratcheted: bool,
    missing_baseline: bool,
) -> Result<String, String> {
    let mut lines = Vec::new();
    if missing_baseline {
        lines.push("--ratchet requested but no baseline exists; reporting the full backlog, not new violations. Write the baseline with: quoin evidence baseline".to_owned());
    }
    let findings = reported
        .as_array()
        .ok_or_else(|| "auditor.audit did not return findings".to_owned())?;
    let unevaluated = report
        .get("unevaluated")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "auditor.audit did not return unevaluated checks".to_owned())?;
    if findings.is_empty() && unevaluated.is_empty() {
        lines.push(format!(
            "{} obligation(s) with healthy evidence; nothing to report",
            report
                .get("healthy")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len)
        ));
    } else {
        for finding in findings {
            lines.push(format!(
                "[{}] {}: {}",
                finding
                    .get("severity")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                finding
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                finding
                    .get("summary")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            ));
        }
        for check in unevaluated {
            lines.push(format!(
                "[not-evaluated] {}: {}: {}",
                check
                    .get("check")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                check
                    .get("obligation")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                check
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            ));
        }
        lines.push(String::new());
        lines.push(format!(
            "{} finding(s), {} check(s) not evaluated, {} healthy{}",
            findings.len(),
            unevaluated.len(),
            report
                .get("healthy")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len),
            if ratcheted {
                " (new violations only)"
            } else {
                ""
            }
        ));
    }
    Ok(lines.join("\n"))
}
