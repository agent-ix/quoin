// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Rust campaign CLI: exact source configuration and independent replay.

#[path = "campaign/config.rs"]
mod config;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use engineering_assurance::campaign::{CampaignDefinition, canonical_digest};
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::{Response, canonical_json};
use quoin_measurement::campaign::CampaignOutcome;
use quoin_measurement::campaign::run::run_campaign;
use quoin_measurement::campaign::verify::{CampaignVerdictReceipt, verify_retained_campaign};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use self::config::RunSelection;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceSelection {
    schema: String,
    sources: BTreeMap<String, PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CampaignCliPayload {
    #[serde(flatten)]
    receipt: CampaignVerdictReceipt,
    rendered: String,
}

pub(super) fn command() -> Command {
    Command::new("campaign")
        .about("Run and independently verify a generic measurement campaign")
        .subcommand(
            Command::new("run")
                .arg(Arg::new("repo").long("repo").default_value("."))
                .arg(Arg::new("definition").long("definition").required(true))
                .arg(Arg::new("run-id").long("run-id").required(true))
                .arg(Arg::new("config").long("config").required(true)),
        )
        .subcommand(
            Command::new("verify")
                .arg(Arg::new("repo").long("repo").default_value("."))
                .arg(
                    Arg::new("definition-digest")
                        .long("definition-digest")
                        .required(true),
                )
                .arg(Arg::new("run-id").long("run-id").required(true))
                .arg(Arg::new("sources").long("sources").required(true)),
        )
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match arguments.subcommand() {
        Some(("run", args)) => execute(args),
        Some(("verify", args)) => verify(args),
        _ => Err("campaign requires run or verify".to_owned()),
    }
}

fn execute(args: &ArgMatches) -> Result<Response, String> {
    let repo = PathBuf::from(required(args, "repo")?);
    let definition: CampaignDefinition =
        load_strict(&PathBuf::from(required(args, "definition")?))?;
    let run_id = required(args, "run-id")?;
    let config: RunSelection = load_strict(&PathBuf::from(required(args, "config")?))?;
    if config.schema != "quoin.campaign-run-config/v1" {
        return Err("unsupported campaign run-config schema".to_owned());
    }
    let (sources, bindings) = config.into_parts()?;
    run_campaign(&repo, &definition, &run_id, &sources, &bindings)
        .map_err(|error| error.to_string())?;
    let digest = canonical_digest(&definition).map_err(|error| error.to_string())?;
    let receipt = verify_retained_campaign(&repo, digest.as_str(), &run_id, &sources)
        .map_err(|error| error.to_string())?;
    response(receipt)
}

fn verify(args: &ArgMatches) -> Result<Response, String> {
    let repo = PathBuf::from(required(args, "repo")?);
    let digest = required(args, "definition-digest")?;
    let run_id = required(args, "run-id")?;
    let config_path = PathBuf::from(required(args, "sources")?);
    let selection: SourceSelection = load_strict(&config_path)?;
    if selection.schema != "quoin.campaign-sources/v1" {
        return Err("unsupported campaign source-selection schema".to_owned());
    }
    let receipt = verify_retained_campaign(&repo, &digest, &run_id, &selection.sources)
        .map_err(|error| error.to_string())?;
    response(receipt)
}

fn response(receipt: CampaignVerdictReceipt) -> Result<Response, String> {
    let outcome = receipt.decision.verdict;
    let rendered = canonical_json(&receipt).map_err(|error| error.to_string())?;
    let payload = serde_json::to_value(CampaignCliPayload { receipt, rendered })
        .map_err(|error| error.to_string())?;
    Ok(match outcome {
        CampaignOutcome::Accept => Response::ok(payload),
        CampaignOutcome::Reject => Response::partial(
            payload,
            &CoreError::new(
                CoreErrorCode::Rejected,
                "campaign evidence contradicts the definition",
            ),
        ),
        CampaignOutcome::Inconclusive => Response::partial(
            payload,
            &CoreError::new(
                CoreErrorCode::Inconclusive,
                "campaign evidence is incomplete",
            ),
        ),
    })
}

fn load_strict<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = quoin_measurement::campaign::store::read_bounded(path)
        .map_err(|error| error.to_string())?;
    let value = quoin_store::parse_strict_json(&raw).map_err(|error| error.to_string())?;
    let canonical = quoin_store::canonical_bytes(&value).map_err(|error| error.to_string())?;
    serde_json::from_slice(&canonical).map_err(|error| error.to_string())
}

fn required(args: &ArgMatches, name: &str) -> Result<String, String> {
    args.get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("missing --{name}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::*;
    use quoin_core::protocol::Outcome;
    use quoin_measurement::campaign::{CAMPAIGN_VERDICT_SCHEMA, CampaignDecision};

    /// Trace: FR-114-AC-5, TC-1946
    // Response projection only; the native CLI path has a separate test.
    #[test]
    fn inconclusive_campaign_response_renders_its_structured_receipt() {
        let receipt = CampaignVerdictReceipt {
            schema: CAMPAIGN_VERDICT_SCHEMA,
            campaign_id: "stage1".to_owned(),
            run_id: "v9-run".to_owned(),
            definition_digest: "definition-digest".to_owned(),
            source_graph_digest: "source-graph-digest".to_owned(),
            run_digest: "run-digest".to_owned(),
            attempt_inventory_digest: "attempt-inventory-digest".to_owned(),
            decision: CampaignDecision {
                schema: CAMPAIGN_VERDICT_SCHEMA,
                verdict: CampaignOutcome::Inconclusive,
                reasons: Vec::new(),
                members: Vec::new(),
            },
        };
        let expected = serde_json::to_value(&receipt).expect("receipt serializes");

        let response = response(receipt).expect("response builds");

        assert_eq!(response.outcome, Outcome::Partial);
        assert_eq!(response.diagnostics.len(), 1);
        let rendered = response
            .payload
            .get("rendered")
            .and_then(serde_json::Value::as_str)
            .expect("CLI-rendered receipt");
        let rendered_value: serde_json::Value =
            serde_json::from_str(rendered).expect("rendered receipt is JSON");
        assert_eq!(rendered_value, expected);
        let mut structured = response.payload;
        structured
            .as_object_mut()
            .expect("receipt object")
            .remove("rendered");
        assert_eq!(structured, expected);
    }
}
