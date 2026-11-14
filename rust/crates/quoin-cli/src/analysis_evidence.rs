// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Host-side assembly for the read-only FR-153 analysis evidence bridge.

use std::path::Path;
use std::process::{Command, Output};

use quoin_evidence::EvidenceSource;
use quoin_evidence::paths::{ANALYSIS_REPORTS_DIR, RUNS_DIR, record_path};
use quoin_evidence::qualify::{
    ActorAdmission, CiAssociation as EvidenceCiAssociation, ExecutionAssociation, HostObservations,
    RetainedRun, RunObservation,
};
use quoin_evidence::source::DiskEvidence;
use quoin_evidence::store::{list_recorded_suites, read_runs, read_trust_decisions};
use quoin_evidence::types::{Binding, MockInjection};
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::ids::{ModuleRoot, ScopeRoot};
use quoin_quire::modules::ModuleSelection;
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
    let mut obligation_counts = std::collections::BTreeMap::<String, usize>::new();
    for obligation in obligations.iter() {
        *obligation_counts.entry(obligation.id.clone()).or_default() += 1;
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
    let source = DiskEvidence::new(repo);
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
                .map_or_else(RunObservation::default, |input| {
                    run_observation(repo, input, &file, &projection_path)
                });
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

fn run_observation(
    repo: &str,
    input: &RunObservationFile,
    run_file: &str,
    projection_file: &str,
) -> RunObservation {
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
    let (actor, actor_rejection) = actor_observation(repo, input, run_file, projection_file);
    RunObservation {
        performed_revision,
        execution,
        actor,
        actor_rejection,
    }
}

fn host_observations(repo: &str, observations: &[RunObservationFile]) -> HostObservations {
    let supplied_heads = observations
        .iter()
        .filter_map(|observation| observation.selected_head.as_deref())
        .collect::<Vec<_>>();
    let heads = observations
        .iter()
        .filter_map(|observation| observation.selected_head.as_deref())
        .filter(|head| valid_object_id(head))
        .collect::<std::collections::BTreeSet<_>>();
    let selected_head_conflict = heads.len() > 1;
    let selected_head_unavailable = supplied_heads.iter().any(|head| !valid_object_id(head));
    let selected_head = (!selected_head_conflict && !selected_head_unavailable)
        .then(|| heads.iter().next().map(|head| (*head).to_owned()))
        .flatten();
    let store = DiskEvidence::new(repo);
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
        selected_head_conflict,
    }
}

fn actor_observation(
    repo: &str,
    input: &RunObservationFile,
    run_file: &str,
    projection_file: &str,
) -> (Option<ActorAdmission>, Option<String>) {
    let Some(authority) = input.actor_authority.as_ref() else {
        return (None, None);
    };
    let Some(signature) = input.signature_result.as_ref() else {
        return (None, None);
    };
    if signature
        .get("protocol")
        .and_then(serde_json::Value::as_str)
        != Some("engineering-assurance.signature-verify-result/v1")
        || signature.get("kind").and_then(serde_json::Value::as_str) != Some("commit")
    {
        return (None, None);
    }
    let Some(commit_id) = first_pair_introduction(repo, run_file, projection_file) else {
        return (None, None);
    };
    let Some(verified_commit_id) = signature
        .get("verified_commit_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| valid_object_id(id))
    else {
        return (None, None);
    };
    if verified_commit_id != commit_id {
        return (None, None);
    }
    if authority.actor.is_empty() || authority.admission_ref.is_empty() {
        return (None, None);
    }
    if signature.get("status").and_then(serde_json::Value::as_str) != Some("signed_valid") {
        return (
            None,
            Some("the introducing commit signature is not signed_valid".to_owned()),
        );
    }
    let signer = signature.get("signer");
    let mapping = signer
        .and_then(|signer| signer.get("mapping"))
        .and_then(serde_json::Value::as_str);
    let principal = signer
        .and_then(|signer| signer.get("principal"))
        .and_then(serde_json::Value::as_str)
        .filter(|principal| !principal.is_empty());
    if mapping != Some("mapped") || principal.is_none() {
        return (
            None,
            Some("the introducing commit signature has no mapped signer principal".to_owned()),
        );
    }
    let principal = principal.unwrap_or_default();
    if authority.actor != principal {
        return (
            None,
            Some("actor authority does not match the mapped signer principal".to_owned()),
        );
    }
    (
        Some(ActorAdmission {
            actor: principal.to_owned(),
            admission_ref: authority.admission_ref.clone(),
            authorized: authority.authorized.clone(),
        }),
        None,
    )
}

fn first_pair_introduction(repo: &str, run_file: &str, projection_file: &str) -> Option<String> {
    let run_path = format!("spec/evidence/{run_file}");
    let projection_path = format!("spec/evidence/{projection_file}");
    let output = git(
        repo,
        &[
            "log",
            "--first-parent",
            "--reverse",
            "--format=commit %H",
            "--name-status",
            "--no-renames",
            "--diff-filter=A",
            "--",
            &run_path,
            &projection_path,
        ],
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut commit = None::<String>;
    let mut additions = std::collections::BTreeSet::new();
    for line in String::from_utf8(output.stdout).ok()?.lines() {
        if let Some(id) = line.strip_prefix("commit ") {
            if additions.contains(&run_path) && additions.contains(&projection_path) {
                return pair_bytes_match(repo, id, &run_path, &projection_path)
                    .then(|| id.to_owned());
            }
            commit = Some(id.to_owned());
            additions.clear();
        } else if let Some((status, path)) = line.split_once('\t')
            && status == "A"
        {
            additions.insert(path.to_owned());
        }
    }
    let id = commit?;
    (additions.contains(&run_path) && additions.contains(&projection_path))
        .then(|| pair_bytes_match(repo, &id, &run_path, &projection_path).then_some(id))
        .flatten()
}

fn pair_bytes_match(repo: &str, commit: &str, run_path: &str, projection_path: &str) -> bool {
    let Some(run_current) = std::fs::read(Path::new(repo).join(run_path)).ok() else {
        return false;
    };
    let Some(projection_current) = std::fs::read(Path::new(repo).join(projection_path)).ok() else {
        return false;
    };
    git_show(repo, commit, run_path).as_deref() == Some(run_current.as_slice())
        && git_show(repo, commit, projection_path).as_deref() == Some(projection_current.as_slice())
}

fn git_show(repo: &str, commit: &str, path: &str) -> Option<Vec<u8>> {
    let output = git(
        repo,
        &["show", "--end-of-options", &format!("{commit}:{path}")],
    )
    .ok()?;
    output.status.success().then_some(output.stdout)
}

fn git(repo: &str, arguments: &[&str]) -> Result<Output, std::io::Error> {
    Command::new("git")
        .args(["-C", repo])
        .args(arguments)
        .output()
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::{actor_observation, first_pair_introduction, host_observations};
    use crate::run_observation::RunObservationFile;
    use ix_trace_rs::trace;
    use std::fs;
    use std::process::Command;

    const RUN: &str = "runs/unit/2026-11-13T00:00:00Z.json";
    const PROJECTION: &str = "analysis-reports/unit/2026-11-13T00:00:00Z.json";

    fn git(repo: &std::path::Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(["-C", repo.to_str().expect("UTF-8 temp path")])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git output is UTF-8")
            .trim()
            .to_owned()
    }

    fn pair_repo() -> (tempfile::TempDir, String) {
        let repo = tempfile::tempdir().expect("temporary git repository");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.name", "VER-97 test"]);
        git(
            repo.path(),
            &["config", "user.email", "ver97@example.invalid"],
        );
        let run = repo.path().join("spec/evidence").join(RUN);
        let projection = repo.path().join("spec/evidence").join(PROJECTION);
        fs::create_dir_all(run.parent().expect("run parent")).expect("run directory");
        fs::create_dir_all(projection.parent().expect("projection parent"))
            .expect("projection directory");
        fs::write(&run, br#"{"protocol":"quoin.run/v1"}"#).expect("run bytes");
        fs::write(
            &projection,
            br#"{"protocol":"quoin.analysis-report-projection/v1"}"#,
        )
        .expect("projection bytes");
        git(repo.path(), &["add", "spec/evidence"]);
        git(repo.path(), &["commit", "-q", "-m", "add paired evidence"]);
        let commit = git(repo.path(), &["rev-parse", "HEAD"]);
        (repo, commit)
    }

    fn observation(signature_result: serde_json::Value) -> RunObservationFile {
        serde_json::from_value(serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": RUN,
            "signature_result": signature_result,
            "actor_authority": {
                "actor": "alice@example.invalid",
                "admission_ref": "policy-7",
                "authorized": ["FR-001-AC-1"],
            }
        }))
        .expect("typed observation")
    }

    fn signature(commit_id: &str) -> serde_json::Value {
        serde_json::json!({
            "protocol": "engineering-assurance.signature-verify-result/v1",
            "status": "signed_valid",
            "kind": "commit",
            "verified_commit_id": commit_id,
            "signer": {"mapping": "mapped", "principal": "alice@example.invalid"},
        })
    }

    /// FR-153-AC-16: actor evidence is admitted only for the commit adding the
    /// exact run/projection pair and matching both currently read byte strings.
    #[trace("FR-153-AC-16")]
    #[test]
    fn exact_pair_introduction_commit_admits_mapped_actor() {
        let (repo, commit) = pair_repo();
        assert_eq!(
            first_pair_introduction(repo.path().to_str().expect("repo path"), RUN, PROJECTION),
            Some(commit.clone())
        );
        let (actor, rejection) = actor_observation(
            repo.path().to_str().expect("repo path"),
            &observation(signature(&commit)),
            RUN,
            PROJECTION,
        );
        assert_eq!(
            actor.expect("exact signature is admitted").actor,
            "alice@example.invalid"
        );
        assert_eq!(rejection, None);

        git(
            repo.path(),
            &["commit", "--allow-empty", "-q", "-m", "later commit"],
        );
        let later = git(repo.path(), &["rev-parse", "HEAD"]);
        assert_ne!(later, commit);
        let (actor, _) = actor_observation(
            repo.path().to_str().expect("repo path"),
            &observation(signature(&later)),
            RUN,
            PROJECTION,
        );
        assert!(
            actor.is_none(),
            "later tree-equivalent signature is not pair-bound"
        );
    }

    /// FR-153-AC-18: every missing or wrong revision binding and every tag
    /// result leaves actor authority unavailable.
    #[trace("FR-153-AC-18")]
    #[test]
    fn absent_null_malformed_mismatched_tag_and_unresolved_ids_fail_closed() {
        let (repo, commit) = pair_repo();
        let mut cases = vec![
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid"}),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid", "verified_commit_id":null}),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid", "verified_commit_id":"not-an-object-id"}),
            signature("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            signature(&commit),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"unverifiable", "reason":"revision_unresolved", "verified_commit_id":null}),
        ];
        cases[4]["kind"] = serde_json::json!("tag");
        for signature_result in cases {
            let (actor, rejection) = actor_observation(
                repo.path().to_str().expect("repo path"),
                &observation(signature_result),
                RUN,
                PROJECTION,
            );
            assert!(actor.is_none(), "invalid identity must not admit an actor");
            assert!(
                rejection.is_none(),
                "invalid identity binding remains unevaluated"
            );
        }
    }

    /// FR-153-AC-18: a signature cannot bind actor authority when there is no
    /// history-derived commit introducing both selected files.
    #[trace("FR-153-AC-18")]
    #[test]
    fn unresolved_pair_introduction_leaves_actor_unavailable() {
        let repo = tempfile::tempdir().expect("temporary git repository");
        git(repo.path(), &["init", "-q"]);
        let run = repo.path().join("spec/evidence").join(RUN);
        let projection = repo.path().join("spec/evidence").join(PROJECTION);
        fs::create_dir_all(run.parent().expect("run parent")).expect("run directory");
        fs::create_dir_all(projection.parent().expect("projection parent"))
            .expect("projection directory");
        fs::write(run, br#"{"protocol":"quoin.run/v1"}"#).expect("uncommitted run bytes");
        fs::write(
            projection,
            br#"{"protocol":"quoin.analysis-report-projection/v1"}"#,
        )
        .expect("uncommitted projection bytes");
        let (actor, rejection) = actor_observation(
            repo.path().to_str().expect("repo path"),
            &observation(signature("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
            RUN,
            PROJECTION,
        );
        assert!(actor.is_none());
        assert!(rejection.is_none());
    }

    /// FR-153-AC-14: the selected HEAD is caller-supplied; conflicting valid
    /// values are preserved as a conflict instead of consulting local Git.
    #[trace("FR-153-AC-14")]
    #[test]
    fn conflicting_selected_heads_are_not_resolved_from_git() {
        let (repo, _) = pair_repo();
        let first = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let second = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let observations = [first, second]
            .into_iter()
            .map(|head| {
                serde_json::from_value(serde_json::json!({
                    "protocol": "quoin.analysis-run-observation/v1",
                    "record": RUN,
                    "selected_head": head,
                }))
                .expect("observation")
            })
            .collect::<Vec<RunObservationFile>>();
        let host = host_observations(repo.path().to_str().expect("repo path"), &observations);
        assert!(host.selected_head_conflict);
        assert!(host.selected_head.is_none());

        let malformed = vec![
            serde_json::from_value(serde_json::json!({
                "protocol": "quoin.analysis-run-observation/v1",
                "record": RUN,
                "selected_head": "",
            }))
            .expect("typed empty observation"),
        ];
        let host = host_observations(repo.path().to_str().expect("repo path"), &malformed);
        assert!(!host.selected_head_conflict);
        assert!(host.selected_head.is_none());
    }
}
