// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained read-only `quoin assurance` view.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::completeness::read_frontmatter;
use crate::core_runtime::invoke;

pub(crate) fn command() -> Command {
    Command::new("assurance")
        .about("Render the assurance case: claim → argument → evidence, from the store")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("module").long("module"))
        .arg(
            Arg::new("claim-type")
                .long("claim-type")
                .action(ArgAction::Append),
        )
        .arg(Arg::new("independence-policy").long("independence-policy"))
        .arg(Arg::new("argument").long("argument"))
        .arg(Arg::new("decisions").long("decisions"))
        .arg(Arg::new("discharge").long("discharge"))
        .arg(Arg::new("evidence").long("evidence"))
        .arg(
            Arg::new("formalization-records")
                .long("formalization-records")
                .value_name("FILE")
                .help("Consume a top-level array of Contract IR FR-071 records"),
        )
        .arg(
            Arg::new("run-observation")
                .long("run-observation")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("FILE"),
        )
        .arg(Arg::new("as-of").long("as-of"))
        .args(crate::formal_rules::trust_arguments())
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_one::<String>("argument").is_some() {
        if arguments
            .get_one::<String>("formalization-records")
            .is_some()
        {
            return Err("--formalization-records applies only to derived assurance".to_owned());
        }
        return authored(arguments);
    }
    derived(arguments)
}

fn authored(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_many::<String>("run-observation").is_some() {
        return Err("--run-observation applies only to derived assurance".to_owned());
    }
    if arguments.get_many::<String>("claim-type").is_some() {
        return Err("--argument cannot be combined with --claim-type".to_owned());
    }
    let repo = required(arguments, "repo")?;
    let argument_id = required(arguments, "argument")?;
    let decisions_path = required(arguments, "decisions")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let as_of = required(arguments, "as-of")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let documents = field(&frontmatter.payload, "documents")?;
    let matches = documents
        .as_array()
        .ok_or_else(|| "completeness.read_frontmatter did not return documents".to_owned())?
        .iter()
        .filter(|document| {
            document
                .get("frontmatter")
                .and_then(|frontmatter| frontmatter.get("id"))
                .and_then(serde_json::Value::as_str)
                == Some(argument_id.as_str())
        })
        .collect::<Vec<_>>();
    let argument = match matches.as_slice() {
        [document] => field(document, "frontmatter")?,
        [] => {
            return Err(format!(
                "AssuranceArgument {argument_id} was not found under {repo}/spec"
            ));
        }
        _ => {
            return Err(format!(
                "AssuranceArgument {argument_id} is declared more than once"
            ));
        }
    };
    let decisions = json_file(&decisions_path, "sufficiency decisions")?;
    if !decisions.is_array() {
        return Err("sufficiency decisions must be a JSON array".to_owned());
    }
    let discharge = arguments
        .get_one::<String>("discharge")
        .map(|path| json_file(path, "discharge report"))
        .transpose()?;
    // PLAT-966: without an index every cited reference reads as unresolved,
    // so the top claim can only be `supported` when the caller supplies one.
    let evidence = match arguments.get_one::<String>("evidence") {
        Some(path) => {
            let evidence = json_file(path, "evidence index entries")?;
            if !evidence.is_array() {
                return Err("evidence index must be a JSON array".to_owned());
            }
            evidence
        }
        None => serde_json::json!([]),
    };
    let view = invoke(
        "assurance.build_authored_argument",
        &serde_json::json!({ "argument": argument, "decisions": decisions, "asOf": as_of, "discharge": discharge, "evidence": evidence }),
    )?;
    if !view.outcome.carries_payload() {
        return Ok(view);
    }
    present(
        "assurance.render_authored_argument",
        &view.payload,
        arguments.get_flag("json"),
    )
}

fn derived(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let signature_verifier = crate::analysis_evidence::signature_verifier(arguments)?;
    let run_observations = read_run_observations(arguments)?;
    let modules = values(arguments, "module");
    let formalization_path = arguments.get_one::<String>("formalization-records");
    let mut coverage_request = serde_json::json!({ "scope": repo, "modules": modules });
    if formalization_path.is_some() {
        coverage_request
            .as_object_mut()
            .ok_or_else(|| "coverage request must be an object".to_owned())?
            .insert(
                "include_formalization_identities".to_owned(),
                serde_json::Value::Bool(true),
            );
    }
    let coverage = invoke("quire.coverage", &coverage_request)?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let obligations = field(&coverage.payload, "obligations")?;
    let formalization_records = read_formalization_records(
        formalization_path,
        &repo,
        &modules,
        &obligations,
        &coverage.payload,
    )?;
    let policy = policy(arguments, &obligations)?;
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "independence_policy": policy }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let catalog = invoke(
        "catalog.methods",
        &if modules.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "roots": modules })
        },
    )?;
    if !catalog.outcome.carries_payload() {
        return Ok(catalog);
    }
    let audit = run_audit(&obligations, &store.payload, &catalog.payload, &policy)?;
    if !audit.outcome.carries_payload() {
        return Ok(audit);
    }
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let trust = invoke(
        "evidence.trust_assessments",
        &serde_json::json!({ "repo": repo }),
    )?;
    if !trust.outcome.carries_payload() {
        return Ok(trust);
    }
    let case_request = build_case_request(&CaseRequestInputs {
        arguments,
        repo: &repo,
        signature_verifier: signature_verifier.as_ref(),
        modules: &modules,
        run_observations: &run_observations,
        coverage: &coverage.payload,
        formalization_records: &formalization_records,
        store: &store.payload,
        audit: &audit.payload,
        frontmatter: &frontmatter.payload,
        trust: &trust.payload,
    })?;
    let case = invoke("assurance.build_case", &case_request)?;
    if !case.outcome.carries_payload() {
        return Ok(case);
    }
    present(
        "assurance.render_case",
        &case.payload,
        arguments.get_flag("json"),
    )
}

struct CaseRequestInputs<'a> {
    arguments: &'a ArgMatches,
    repo: &'a str,
    signature_verifier: Option<&'a crate::analysis_evidence::SignatureVerifier>,
    modules: &'a [String],
    run_observations: &'a [crate::run_observation::RunObservationFile],
    coverage: &'a serde_json::Value,
    formalization_records: &'a [quoin_assurance::FormalizationRecordView],
    store: &'a serde_json::Value,
    audit: &'a serde_json::Value,
    frontmatter: &'a serde_json::Value,
    trust: &'a serde_json::Value,
}

fn build_case_request(input: &CaseRequestInputs<'_>) -> Result<serde_json::Value, String> {
    let CaseRequestInputs {
        arguments,
        repo,
        signature_verifier,
        modules,
        run_observations,
        coverage,
        formalization_records,
        store,
        audit,
        frontmatter,
        trust,
    } = input;
    let mut unreadable = field(frontmatter, "unreadable")?
        .as_array()
        .cloned()
        .unwrap_or_default();
    unreadable.extend(field(trust, "unreadable")?.as_array().into_iter().flatten().filter_map(serde_json::Value::as_str).map(|path| serde_json::json!({ "path": path, "reason": "trust decision is unreadable or invalid" })));
    let observations: Vec<serde_json::Value> = ["runs", "scans"]
        .iter()
        .filter_map(|key| store.get(*key)?.as_array())
        .flatten()
        .filter_map(|record| {
            Some(serde_json::json!({
                "suite": record.get("suite")?,
                "timestamp": record.get("timestamp")?,
            }))
        })
        .collect();
    let (case_obligations, analysis_evidence) = analysis_case_inputs(
        repo,
        modules,
        store,
        field(coverage, "obligations")?,
        run_observations,
        *signature_verifier,
    )?;
    let mut case_request = serde_json::json!({
        "bindings": field(store, "bindings")?, "observations": observations,
        "healthy": field(&field(audit, "report")?, "healthy")?,
        "unevaluated": field(&field(audit, "report")?, "unevaluated")?,
        "documents": field(frontmatter, "documents")?, "obligations": case_obligations,
        "findings": field(&field(audit, "report")?, "findings")?,
        "unreadable": unreadable, "producer_trust": field(trust, "assessments")?,
        "evidence_independence": audit.get("independence"),
        "formalization_records": formalization_records
    });
    let object = case_request
        .as_object_mut()
        .ok_or_else(|| "assurance case request must be an object".to_owned())?;
    object.insert("analysis_evidence".to_owned(), analysis_evidence);
    if let Some(claim_types) = arguments.get_many::<String>("claim-type") {
        object.insert(
            "claim_types".to_owned(),
            serde_json::json!(claim_types.cloned().collect::<Vec<_>>()),
        );
    }
    Ok(case_request)
}

fn read_formalization_records(
    path: Option<&String>,
    repo: &str,
    modules: &[String],
    obligations: &serde_json::Value,
    coverage: &serde_json::Value,
) -> Result<Vec<quoin_assurance::FormalizationRecordView>, String> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let rules = crate::formal_rules::assurance_declarations(std::path::Path::new(repo), modules)?;
    let identities = field(coverage, "formalization_identities")?;
    crate::formalization_records::read_and_join(path, obligations, &identities, &rules)
}

fn read_run_observations(
    arguments: &ArgMatches,
) -> Result<Vec<crate::run_observation::RunObservationFile>, String> {
    let paths = arguments
        .get_many::<String>("run-observation")
        .map(|paths| paths.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    crate::run_observation::read_run_observations(&paths)
}

fn run_audit(
    obligations: &serde_json::Value,
    store: &serde_json::Value,
    catalog: &serde_json::Value,
    policy: &serde_json::Value,
) -> Result<Response, String> {
    invoke(
        "auditor.audit",
        &serde_json::json!({ "input": {
            "obligations": obligations,
            "bindings": field(store, "bindings")?, "runs": field(store, "runs")?,
            "scans": field(store, "scans")?, "injections": field(store, "injections")?,
            "mockInspectionSuites": field(store, "mock_inspection_suites")?,
            "vacuousScanSuites": field(store, "vacuous_scan_suites")?,
            "independence": field(store, "independence")?, "catalog": catalog,
            "independencePolicy": policy
        }}),
    )
}

fn analysis_case_inputs(
    repo: &str,
    modules: &[String],
    store_inputs: &serde_json::Value,
    obligation_value: serde_json::Value,
    observations: &[crate::run_observation::RunObservationFile],
    signature_verifier: Option<&crate::analysis_evidence::SignatureVerifier>,
) -> Result<(Vec<quoin_quire_types::Obligation>, serde_json::Value), String> {
    let mut obligations: Vec<quoin_quire_types::Obligation> =
        serde_json::from_value(obligation_value)
            .map_err(|error| format!("quire.coverage obligations are malformed: {error}"))?;
    let analysis = crate::analysis_evidence::compute(
        repo,
        modules,
        store_inputs,
        &obligations,
        observations,
        signature_verifier,
    )?;
    let report = serde_json::to_value(&analysis.report)
        .map_err(|error| format!("analysis evidence report is malformed: {error}"))?;
    crate::analysis_evidence::attach_unique_target_identities(
        &mut obligations,
        report
            .get("coverage_matrix")
            .and_then(serde_json::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?;
    let items = serde_json::json!({
        "record_binders": report.get("record_binders").cloned().unwrap_or_else(|| serde_json::json!([])),
        "evidence_results": report.get("evidence_results").cloned().unwrap_or_else(|| serde_json::json!([])),
    });
    Ok((obligations, items))
}

fn present(operation: &str, value: &serde_json::Value, json: bool) -> Result<Response, String> {
    let rendered = if json {
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
    } else {
        let response = invoke(operation, value)?;
        if !response.outcome.carries_payload() {
            return Ok(response);
        }
        field(&response.payload, "rendered")?
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
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
    let response = invoke(
        "evidence.parse_policy",
        &serde_json::json!({ "text": text, "known_obligations": known_obligations }),
    )?;
    if !response.outcome.carries_payload() {
        return Err("independence policy was refused".to_owned());
    }
    field(&response.payload, "policy")
}

fn json_file(path: &str, label: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{label} are not readable JSON: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label} are not readable JSON: {error}"))
}
fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;
    use ix_trace_rs::trace;

    #[trace("FR-040", "FR-062", "FR-153-AC-12")]
    #[test]
    fn tc_447_742_preserves_the_retained_assurance_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--repo",
                "other",
                "--module",
                "module",
                "--claim-type",
                "StR",
                "--claim-type",
                "hazard",
                "--independence-policy",
                "policy.json",
                "--run-observation",
                "one.json",
                "--run-observation",
                "two.json",
                "--json",
            ])
            .expect("the retained derived-view grammar parses");

        assert_eq!(
            matches.get_one::<String>("repo").map(String::as_str),
            Some("other")
        );
        assert_eq!(
            matches
                .get_many::<String>("claim-type")
                .expect("claim types")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["StR", "hazard"]
        );
        assert!(matches.get_flag("json"));
        assert_eq!(
            matches
                .get_many::<String>("run-observation")
                .expect("run observations")
                .count(),
            2
        );
    }

    #[trace("FR-047", "FR-062")]
    #[test]
    fn tc_447_743_preserves_the_authored_argument_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--argument",
                "ARG-001",
                "--decisions",
                "decisions.json",
                "--discharge",
                "discharge.json",
                "--evidence",
                "evidence.json",
                "--as-of",
                "2026-09-14T00:00:00Z",
            ])
            .expect("the retained authored-argument grammar parses");

        assert_eq!(
            matches.get_one::<String>("argument").map(String::as_str),
            Some("ARG-001")
        );
        assert_eq!(
            matches.get_one::<String>("evidence").map(String::as_str),
            Some("evidence.json")
        );
        assert_eq!(
            matches.get_one::<String>("as-of").map(String::as_str),
            Some("2026-09-14T00:00:00Z")
        );
    }
}
