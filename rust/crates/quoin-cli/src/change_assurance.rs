// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for `quoin change-assurance` (quoin#373, Stage 8).
//!
//! The core process owns strict document parsing, digest decisions, evidence
//! storage, and schema assets. This module owns only the retained command
//! grammar and presentation.

use std::fmt::Write as _;
use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_bridge::invoke;

/// The change-assurance command grammar.
pub(crate) fn command() -> Command {
    Command::new("change-assurance")
        .about("Form, retain, and verify explicit change-assurance evidence")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("recover")
                .about("Remove interrupted intake staging directories")
                .arg(repo_arg())
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("schema")
                .about("Emit a packaged change-assurance JSON Schema asset")
                .arg(Arg::new("name").long("name").value_name("NAME"))
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("seal-record")
                .about("Seal an explicit change-assurance record and retain it")
                .arg(Arg::new("input").long("input").required(true))
                .arg(repo_arg())
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("seal-attestation")
                .about("Seal a proof attestation over an existing result file")
                .arg(Arg::new("input").long("input").required(true))
                .arg(Arg::new("output").long("output").required(true))
                .arg(Arg::new("media-type").long("media-type").required(true))
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("intake")
                .about("Retain an exact sealed attestation and its result bytes")
                .arg(Arg::new("attestation").long("attestation").required(true))
                .arg(Arg::new("output").long("output").required(true))
                .arg(repo_arg())
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("verify-receipt")
                .about("Re-verify a sealed verification receipt")
                .arg(Arg::new("input").long("input").required(true))
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("receipt")
                .about("Verify a candidate from retained inputs and emit its receipt")
                .arg(Arg::new("record").long("record").required(true))
                .arg(
                    Arg::new("candidate-revision")
                        .long("candidate-revision")
                        .required(true),
                )
                .arg(Arg::new("parent").long("parent").action(ArgAction::Append))
                .arg(Arg::new("select").long("select").action(ArgAction::Append))
                .arg(Arg::new("decisions").long("decisions").required(true))
                .arg(Arg::new("audits").long("audits"))
                .arg(repo_arg())
                .arg(json_arg()),
        )
}

/// Execute a parsed change-assurance command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (subcommand, arguments) = matches
        .subcommand()
        .ok_or_else(|| "a change-assurance subcommand is required".to_owned())?;
    match subcommand {
        "recover" => recover(arguments),
        "schema" => schema(arguments),
        "seal-record" => seal_record(arguments),
        "seal-attestation" => seal_attestation(arguments),
        "intake" => intake(arguments),
        "verify-receipt" => verify_receipt(arguments),
        "receipt" => receipt(arguments),
        _ => Err("an unknown change-assurance command reached dispatch".to_owned()),
    }
}

fn repo_arg() -> Arg {
    Arg::new("repo")
        .long("repo")
        .value_name("PATH")
        .default_value(".")
}

fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help("Emit canonical JSON")
}

fn recover(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let response = invoke(
        "change_assurance.recover",
        &serde_json::json!({ "repo": repo }),
    )?;
    render(response, arguments.get_flag("json"), |payload| {
        let removed = payload
            .get("removed")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "change_assurance.recover did not return removed".to_owned())?;
        if arguments.get_flag("json") {
            Ok(
                serde_json::to_string(&serde_json::json!({ "removed": removed }))
                    .map_err(|error| error.to_string())?,
            )
        } else {
            Ok(format!(
                "removed {removed} interrupted intake staging director{}",
                if removed == 1 { "y" } else { "ies" }
            ))
        }
    })
}

fn schema(arguments: &ArgMatches) -> Result<Response, String> {
    let name = arguments.get_one::<String>("name");
    let response = invoke(
        "change_assurance.schema",
        &serde_json::json!({ "name": name }),
    )?;
    render(response, arguments.get_flag("json"), |payload| {
        if let Some(name) = name {
            return payload
                .get("schema")
                .and_then(serde_json::Value::as_str)
                .map(|schema| schema.trim_end_matches('\n').to_owned())
                .ok_or_else(|| format!("change_assurance.schema did not return {name}"));
        }
        let schemas = payload
            .get("schemas")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "change_assurance.schema did not return schemas".to_owned())?;
        if arguments.get_flag("json") {
            return serde_json::to_string(&serde_json::json!({ "schemas": schemas }))
                .map_err(|error| error.to_string());
        }
        Ok(schemas
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join("\n"))
    })
}

fn seal_record(arguments: &ArgMatches) -> Result<Response, String> {
    let input = bytes_of(&required(arguments, "input")?)?;
    let repo = required(arguments, "repo")?;
    let response = invoke(
        "change_assurance.seal_record",
        &serde_json::json!({ "repo": repo, "record_hex": hex_of(&input) }),
    )?;
    render(response, true, |payload| {
        let record = payload
            .get("record")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "change_assurance.seal_record did not return record".to_owned())?;
        let record_id = record
            .get("record_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "sealed record has no record_id".to_owned())?;
        let revision = record
            .get("revision")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "sealed record has no revision".to_owned())?;
        let digest = record
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "sealed record has no digest".to_owned())?;
        let path = payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "change_assurance.seal_record did not return path".to_owned())?;
        if arguments.get_flag("json") {
            return serde_json::to_string(&serde_json::json!({
                "digest": digest, "path": path, "record_id": record_id, "revision": revision,
            }))
            .map_err(|error| error.to_string());
        }
        Ok(format!(
            "sealed {record_id} revision {revision}\n  digest: {digest}\n  retained: {path}"
        ))
    })
}

fn seal_attestation(arguments: &ArgMatches) -> Result<Response, String> {
    let input = bytes_of(&required(arguments, "input")?)?;
    let output = bytes_of(&required(arguments, "output")?)?;
    let media_type = required(arguments, "media-type")?;
    let response = invoke(
        "change_assurance.seal_attestation",
        &serde_json::json!({
            "attestation_hex": hex_of(&input), "output_hex": hex_of(&output), "media_type": media_type,
        }),
    )?;
    render(response, true, |payload| {
        serde_json::to_string(payload.get("attestation").ok_or_else(|| {
            "change_assurance.seal_attestation did not return attestation".to_owned()
        })?)
        .map_err(|error| error.to_string())
    })
}

fn intake(arguments: &ArgMatches) -> Result<Response, String> {
    let attestation = bytes_of(&required(arguments, "attestation")?)?;
    let output = bytes_of(&required(arguments, "output")?)?;
    let repo = required(arguments, "repo")?;
    let response = invoke(
        "change_assurance.intake",
        &serde_json::json!({
            "repo": repo, "attestation_hex": hex_of(&attestation), "output_hex": hex_of(&output),
        }),
    )?;
    render(response, arguments.get_flag("json"), |payload| {
        let directory = payload
            .get("directory")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "change_assurance.intake did not return directory".to_owned())?;
        let size = payload
            .get("size_bytes")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "change_assurance.intake did not return size_bytes".to_owned())?;
        if arguments.get_flag("json") {
            serde_json::to_string(
                &serde_json::json!({ "directory": directory, "size_bytes": size }),
            )
            .map_err(|error| error.to_string())
        } else {
            Ok(format!(
                "retained attestation → {directory}\n  output bytes: {size}"
            ))
        }
    })
}

fn verify_receipt(arguments: &ArgMatches) -> Result<Response, String> {
    let input = bytes_of(&required(arguments, "input")?)?;
    let response = invoke(
        "change_assurance.verify_receipt",
        &serde_json::json!({ "receipt_hex": hex_of(&input) }),
    )?;
    receipt_response(response, arguments.get_flag("json"), true)
}

fn receipt(arguments: &ArgMatches) -> Result<Response, String> {
    let decisions = bytes_of(&required(arguments, "decisions")?)?;
    let audits = arguments
        .get_one::<String>("audits")
        .map(|source| bytes_of(source))
        .transpose()?;
    let selections = arguments
        .get_many::<String>("select")
        .into_iter()
        .flatten()
        .map(|selection| parse_selection(selection))
        .collect::<Result<Vec<_>, _>>()?;
    let parents = arguments
        .get_many::<String>("parent")
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let response = invoke(
        "change_assurance.receipt",
        &serde_json::json!({
            "repo": required(arguments, "repo")?,
            "record_digest": required(arguments, "record")?,
            "candidate_revision": required(arguments, "candidate-revision")?,
            "parent_digests": parents,
            "selections": selections,
            "decisions_hex": hex_of(&decisions),
            "audits_hex": audits.as_deref().map(hex_of),
        }),
    )?;
    receipt_response(response, arguments.get_flag("json"), false)
}

fn parse_selection(raw: &str) -> Result<serde_json::Value, String> {
    let Some((proof_id, digest)) = raw.split_once('=') else {
        return Err(format!(
            "--select {raw} must be <proof-id>=<64-character lowercase hex digest>"
        ));
    };
    let proof_id = proof_id.trim();
    let digest = digest.trim();
    if proof_id.is_empty()
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!(
            "--select {raw} must be <proof-id>=<64-character lowercase hex digest>"
        ));
    }
    Ok(serde_json::json!({ "proof_id": proof_id, "attestation_digest": digest }))
}

fn receipt_response(response: Response, json: bool, verified: bool) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let receipt = response
        .payload
        .get("receipt")
        .ok_or_else(|| "change-assurance receipt operation did not return receipt".to_owned())?;
    let outcome = receipt
        .get("outcome")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "receipt has no outcome".to_owned())?;
    let text = if json {
        if verified {
            let fields = [
                "digest",
                "record_digest",
                "candidate_revision",
                "outcome",
                "reasons",
            ];
            let mut output = serde_json::Map::new();
            for field in fields {
                if let Some(value) = receipt.get(field) {
                    output.insert(field.to_owned(), value.clone());
                }
            }
            serde_json::to_string(&output).map_err(|error| error.to_string())?
        } else {
            serde_json::to_string(receipt).map_err(|error| error.to_string())?
        }
    } else {
        receipt_text(receipt, verified)?
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": text }),
        diagnostics: response.diagnostics,
        outcome: if outcome == "valid" {
            response.outcome
        } else {
            quoin_core::protocol::Outcome::Partial
        },
    })
}

fn receipt_text(receipt: &serde_json::Value, verified: bool) -> Result<String, String> {
    let digest = receipt
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "receipt has no digest".to_owned())?;
    let outcome = receipt
        .get("outcome")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "receipt has no outcome".to_owned())?;
    let mut lines = vec![
        if verified {
            format!("receipt {digest} verified")
        } else {
            format!("receipt {digest}")
        },
        format!("  outcome: {outcome}"),
    ];
    if let Some(reasons) = receipt.get("reasons").and_then(serde_json::Value::as_array) {
        let reasons = reasons
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            lines.push(format!("  reasons: {}", reasons.join(", ")));
        }
    }
    if !verified && let Some(proofs) = receipt.get("proofs").and_then(serde_json::Value::as_array) {
        for proof in proofs {
            let id = proof
                .get("proof_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let result = proof
                .get("outcome")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let reasons = proof
                .get("reasons")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            lines.push(if reasons.is_empty() {
                format!("  {id}: {result}")
            } else {
                format!("  {id}: {result} ({})", reasons.join(", "))
            });
        }
    }
    Ok(lines.join("\n"))
}

fn render(
    response: Response,
    _json: bool,
    text: impl FnOnce(&serde_json::Value) -> Result<String, String>,
) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    Ok(Response {
        payload: serde_json::json!({ "rendered": text(&response.payload)? }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    })
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}

fn bytes_of(source: &str) -> Result<Vec<u8>, String> {
    if source != "-" {
        return std::fs::read(source).map_err(|error| format!("cannot read {source}: {error}"));
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read standard input: {error}"))?;
    Ok(bytes)
}

fn hex_of(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::command;

    /// Trace: FR-068, FR-101
    #[test]
    fn tc_373_change_assurance_schema_and_recover_preserve_their_grammar() {
        assert!(
            command()
                .try_get_matches_from(["change-assurance", "schema"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "change-assurance",
                    "schema",
                    "--name",
                    "proof-attestation-v1.schema.json",
                    "--json",
                ])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["change-assurance", "recover", "--repo", "candidate"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["change-assurance", "seal-record"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "change-assurance",
                    "seal-attestation",
                    "--input",
                    "body.json",
                    "--output",
                    "result.xml",
                    "--media-type",
                    "application/xml",
                ])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "change-assurance",
                    "receipt",
                    "--record",
                    "digest",
                    "--candidate-revision",
                    "revision",
                    "--decisions",
                    "decisions.json",
                    "--select",
                    "proof=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ])
                .is_ok()
        );
    }
}
