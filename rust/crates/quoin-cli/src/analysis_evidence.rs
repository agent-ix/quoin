// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Host-side assembly for the read-only FR-153 analysis evidence bridge.

use std::process::Command;

use quoin_evidence::EvidenceSource;
use quoin_evidence::paths::{ANALYSIS_REPORTS_DIR, RUNS_DIR, record_path};
use quoin_evidence::qualify::{
    CiAssociation as EvidenceCiAssociation, ExecutionAssociation, HostObservations, RetainedRun,
    RunObservation,
};
use quoin_evidence::source::DiskEvidence;
use quoin_evidence::store::{list_recorded_suites, read_runs, read_trust_decisions};
use quoin_evidence::types::{Binding, MockInjection};
use quoin_quire::coverage::{ModuleSelection, Request as ScopeRequest};
use quoin_quire::ids::{ModuleRoot, ScopeRoot};
use quoin_quire_types::Obligation;

use crate::run_observation::RunObservationFile;

/// Carry an exact native target onto a CLI obligation only where both the
/// obligation id and target are unique in the evidence-mode matrix.
pub(crate) fn attach_unique_target_identities(
    obligations: &mut [Obligation],
    coverage_matrix: &[serde_json::Value],
) -> Result<(), String> {
    let mut criteria_by_id: std::collections::BTreeMap<
        String,
        Vec<quoin_quire_types::TargetLocus>,
    > = std::collections::BTreeMap::new();
    let mut criteria_counts = std::collections::BTreeMap::<String, usize>::new();
    for requirement in coverage_matrix {
        let criteria = requirement
            .get("criteria")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "analysis evidence coverage requirement omitted criteria".to_owned())?;
        for criterion in criteria {
            let id = criterion
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "analysis evidence criterion omitted id".to_owned())?;
            *criteria_counts.entry(id.to_owned()).or_default() += 1;
            if let Some(identity) = criterion.get("identity").filter(|value| !value.is_null()) {
                let identity = serde_json::from_value(identity.clone()).map_err(|error| {
                    format!("analysis evidence criterion identity is malformed: {error}")
                })?;
                criteria_by_id
                    .entry(id.to_owned())
                    .or_default()
                    .push(identity);
            }
        }
    }
    let mut obligation_counts = std::collections::BTreeMap::<&str, usize>::new();
    for obligation in obligations.iter() {
        *obligation_counts.entry(&obligation.id).or_default() += 1;
    }
    for obligation in obligations {
        if obligation_counts.get(obligation.id.as_str()) != Some(&1) {
            continue;
        }
        if criteria_counts.get(&obligation.id) != Some(&1) {
            continue;
        }
        if let Some([identity]) = criteria_by_id.get(&obligation.id).map(Vec::as_slice) {
            obligation.identity = Some(identity.clone());
        }
    }
    Ok(())
}

/// Run the FR-153 qualifier over every current retained run and caller observation.
pub(crate) fn compute(
    repo: &str,
    module_paths: &[String],
    audit_inputs: &serde_json::Value,
    obligations: &[Obligation],
    observations: &[RunObservationFile],
) -> Result<quoin_quire::evidence::Outcome, String> {
    let modules = module_selection(module_paths)?;
    let scope = ScopeRoot::open(repo).map_err(|error| error.to_string())?;
    let host = host_observations(repo, &observations);
    let (runs, bindings) = retained_runs(repo, &observations)?;
    let injections = decode_field::<Vec<MockInjection>>(audit_inputs, "injections")?;
    let mock_inspection_suites =
        decode_field::<Vec<String>>(audit_inputs, "mock_inspection_suites")?;
    let requested = obligations
        .iter()
        .filter(|obligation| obligation.method.as_deref() == Some("Analysis"))
        .map(|obligation| obligation.id.clone())
        .collect();
    quoin_quire::evidence::compute(&quoin_quire::evidence::Request {
        scope: ScopeRequest { scope, modules },
        requested,
        runs,
        bindings,
        injections,
        mock_inspection_suites,
        host,
    })
    .map_err(|error| error.to_string())
}

fn module_selection(paths: &[String]) -> Result<ModuleSelection, String> {
    if paths.is_empty() {
        return Ok(ModuleSelection::Ambient);
    }
    let roots = paths
        .iter()
        .map(ModuleRoot::open)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(ModuleSelection::Closed(roots))
}

fn decode_field<T: serde::de::DeserializeOwned>(
    object: &serde_json::Value,
    field: &str,
) -> Result<Option<T>, String> {
    let value = object
        .get(field)
        .ok_or_else(|| format!("evidence.audit_inputs omitted {field}"))?;
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|error| format!("evidence.audit_inputs {field} is malformed: {error}"))
}

fn retained_runs(
    repo: &str,
    observations: &[RunObservationFile],
) -> Result<(Vec<RetainedRun>, Vec<Binding>), String> {
    let mut source = DiskEvidence::new(repo);
    let mut skipped = Vec::new();
    let bindings_text = source
        .read("bindings.json")
        .map_err(|error| error.to_string())?;
    let bindings = match bindings_text {
        Some(text) => {
            let file: quoin_evidence::types::BindingsFile = serde_json::from_str(&text)
                .map_err(|error| format!("bindings.json is malformed: {error}"))?;
            file.bindings
        }
        None => Vec::new(),
    };
    let mut runs = Vec::new();
    for suite in list_recorded_suites(&source).map_err(|error| error.to_string())? {
        for record in read_runs(&source, &suite, &mut skipped).map_err(|error| error.to_string())? {
            let file = record_path(RUNS_DIR, &record.suite, &record.timestamp);
            let projection_path =
                record_path(ANALYSIS_REPORTS_DIR, &record.suite, &record.timestamp);
            let report = source
                .read(&projection_path)
                .map_err(|error| error.to_string())?
                .map(|text| {
                    serde_json::from_str(&text)
                        .map_err(|error| format!("{projection_path} is malformed: {error}"))
                })
                .transpose()?;
            let observation = observations
                .iter()
                .find(|observation| observation.record == file)
                .map_or_else(RunObservation::default, run_observation);
            runs.push(RetainedRun {
                file,
                record,
                report,
                observation,
            });
        }
    }
    for observation in observations {
        if !runs.iter().any(|run| run.file == observation.record) {
            return Err(format!(
                "run observation names no readable selected record: {}",
                observation.record
            ));
        }
    }
    Ok((runs, bindings))
}

fn run_observation(input: &RunObservationFile) -> RunObservation {
    let performed_revision = input
        .checkout_sha
        .as_deref()
        .filter(|sha| valid_object_id(sha))
        .map(ToOwned::to_owned);
    let execution = input.ci_association.as_ref().map(|ci| {
        ExecutionAssociation::Ci(EvidenceCiAssociation {
            system: ci.system.clone(),
            project: ci.project.clone(),
            run: ci.run.clone(),
            attempt: ci.attempt.clone(),
            job: ci.job.clone(),
            invocation: ci.invocation.clone(),
        })
    });
    // Actor fields remain unavailable until signature evidence is bound to the
    // history-derived pair-introducing commit by the host.
    RunObservation {
        performed_revision,
        execution,
        actor: None,
    }
}

fn host_observations(repo: &str, observations: &[RunObservationFile]) -> HostObservations {
    let selected_head = git_head(repo).filter(|head| {
        observations
            .iter()
            .filter_map(|observation| observation.selected_head.as_deref())
            .filter(|candidate| valid_object_id(candidate))
            .all(|candidate| candidate == head)
    });
    let mut store = DiskEvidence::new(repo);
    let mut skipped = Vec::new();
    let reliance = read_trust_decisions(&store, &mut skipped)
        .ok()
        .map(|decisions| {
            let matches = decisions
                .into_iter()
                .filter(|decision| {
                    decision.r#use.id == quoin_evidence::qualify::POSITIVE_USE_DECISION
                })
                .collect::<Vec<_>>();
            (matches.len() == 1)
                .then(|| matches.into_iter().next())
                .flatten()
        })
        .flatten();
    HostObservations {
        reliance,
        selected_head,
    }
}

fn git_head(repo: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", repo, "rev-parse", "--verify", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    valid_object_id(&head).then_some(head)
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
