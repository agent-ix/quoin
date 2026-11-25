// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Host-side assembly for the read-only FR-153 analysis evidence bridge.

use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output};

use clap::ArgMatches;
use quoin_evidence::EvidenceSource;
use quoin_evidence::paths::{ANALYSIS_REPORTS_DIR, RUNS_DIR, record_path, suite_dir};
use quoin_evidence::qualify::{
    ActorAdmission, CiAssociation as EvidenceCiAssociation, ExecutionAssociation, HostObservations,
    RetainedRun, RunObservation,
};
use quoin_evidence::source::DiskEvidence;
use quoin_evidence::store::{
    list_recorded_suites, list_runs, read_trust_decisions, validate_run_record,
};
use quoin_evidence::types::{Binding, MockInjection, RunRecord};
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::ids::{ModuleRoot, ScopeRoot};
use quoin_quire::modules::ModuleSelection;
use quoin_quire_types::Obligation;

use crate::core_runtime::Failure;
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
    signature_verifier: Option<&SignatureVerifier>,
) -> Result<quoin_quire::evidence::Outcome, Failure> {
    quoin_quire::evidence::compute(&request(
        repo,
        module_paths,
        audit_inputs,
        obligations,
        observations,
        signature_verifier,
    )?)
    .map_err(|error| Failure::from_quire(&error, "evidence.compute"))
}

/// Render design rows with the exact native identities used for qualification,
/// and hand back the retained runs the qualification read, so the matrix reads
/// run identities from the same records.
pub(crate) fn compute_for_matrix(
    repo: &str,
    module_paths: &[String],
    audit_inputs: &serde_json::Value,
    obligations: &[Obligation],
    observations: &[RunObservationFile],
    signature_verifier: Option<&SignatureVerifier>,
) -> Result<
    (
        quoin_quire::evidence::Outcome,
        serde_json::Value,
        Vec<RetainedRun>,
    ),
    Failure,
> {
    let request = request(
        repo,
        module_paths,
        audit_inputs,
        obligations,
        observations,
        signature_verifier,
    )?;
    let (outcome, design) = quoin_quire::evidence::compute_for_matrix(&request)
        .map_err(|error| Failure::from_quire(&error, "evidence.compute_for_matrix"))?;
    let design = serde_json::to_value(design)
        .map_err(|error| format!("analysis design matrix is malformed: {error}"))?;
    Ok((outcome, design, request.runs))
}

fn request(
    repo: &str,
    module_paths: &[String],
    audit_inputs: &serde_json::Value,
    obligations: &[Obligation],
    observations: &[RunObservationFile],
    signature_verifier: Option<&SignatureVerifier>,
) -> Result<quoin_quire::evidence::Request, String> {
    let modules = module_selection(module_paths)?;
    let scope = ScopeRoot::open(repo).map_err(|error| error.to_string())?;
    let canonical_repo = Path::new(repo)
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let canonical_repo = canonical_repo
        .to_str()
        .ok_or_else(|| "repository path is not UTF-8".to_owned())?;
    let host = host_observations(canonical_repo, observations);
    let current_formal_rule_status = observations
        .iter()
        .any(|observation| observation.run_kind.as_deref() == Some("formal_rule"))
        .then(|| fresh_formal_rule_status(canonical_repo, module_paths, signature_verifier))
        .flatten();
    let (runs, bindings) = retained_runs(
        canonical_repo,
        observations,
        signature_verifier,
        current_formal_rule_status.as_ref(),
    )?;
    let injections = decode_field::<Vec<MockInjection>>(audit_inputs, "injections")?;
    let mock_inspection_suites =
        decode_field::<Vec<String>>(audit_inputs, "mock_inspection_suites")?;
    let requested = obligations
        .iter()
        .filter(|obligation| obligation.method.as_deref() == Some("Analysis"))
        .map(|obligation| obligation.id.clone())
        .collect();
    let requested_targets = obligations
        .iter()
        .filter(|obligation| obligation.method.as_deref() == Some("Analysis"))
        .map(|obligation| obligation.identity.clone())
        .collect();
    Ok(quoin_quire::evidence::Request {
        scope: ScopeRequest { scope, modules },
        requested,
        requested_targets,
        runs,
        bindings,
        injections,
        mock_inspection_suites,
        host,
    })
}

/// Host-configured verifier and signer trust store for actor admission.
pub(crate) struct SignatureVerifier {
    /// Engineering-assurance executable.
    pub(crate) executable: PathBuf,
    /// SSH allowed-signers file used by engineering-assurance.
    pub(crate) allowed_signers: PathBuf,
    /// Optional Sigstore identity expectations.
    pub(crate) gitsign: Option<serde_json::Value>,
}

/// Read the configured signature verifier and its trust inputs from the host CLI.
pub(crate) fn signature_verifier(
    arguments: &ArgMatches,
) -> Result<Option<SignatureVerifier>, String> {
    let executable = arguments.get_one::<String>("engineering-assurance");
    let allowed_signers = arguments.get_one::<String>("allowed-signers");
    let (Some(executable), Some(allowed_signers)) = (executable, allowed_signers) else {
        if executable.is_none() && allowed_signers.is_none() {
            return Ok(None);
        }
        return Err(
            "--engineering-assurance and --allowed-signers must be supplied together".to_owned(),
        );
    };
    let executable = absolute_existing_file(executable, "--engineering-assurance")?;
    let allowed_signers = absolute_existing_file(allowed_signers, "--allowed-signers")?;
    let identity = arguments.get_one::<String>("gitsign-certificate-identity");
    let issuer = arguments.get_one::<String>("gitsign-certificate-oidc-issuer");
    let principal = arguments.get_one::<String>("gitsign-principal");
    let gitsign = match (identity, issuer, principal) {
        (None, None, None) => None,
        (Some(identity), Some(issuer), Some(principal)) => Some(serde_json::json!({
            "certificate_identity": identity,
            "certificate_oidc_issuer": issuer,
            "principal": principal,
        })),
        _ => return Err("all three gitsign identity options must be supplied together".to_owned()),
    };
    Ok(Some(SignatureVerifier {
        executable,
        allowed_signers,
        gitsign,
    }))
}

fn absolute_existing_file(value: &str, flag: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if value.is_empty() || value.contains('\0') || !path.is_absolute() {
        return Err(format!("{flag} must be an absolute path"));
    }
    if !path.is_file() {
        return Err(format!("{flag} is not a readable file"));
    }
    Ok(path)
}

pub(crate) fn module_selection(paths: &[String]) -> Result<ModuleSelection, String> {
    if paths.is_empty() {
        // Match `quire.coverage`: an inline module at the repository root is
        // the selected model before ambient discovery. The matrix and release
        // snapshot pass no module flags by default, and their analysis join
        // must use the same model as the coverage walk.
        return Ok(ModuleSelection::ScopeOrAmbient);
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
    signature_verifier: Option<&SignatureVerifier>,
    current_formal_rule_status: Option<&serde_json::Value>,
) -> Result<(Vec<RetainedRun>, Vec<Binding>), String> {
    let source = DiskEvidence::new(repo);
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
        let directory = suite_dir(RUNS_DIR, &suite);
        for name in list_runs(&source, &suite).map_err(|error| error.to_string())? {
            let Some(snapshot) = RunSnapshot::read(&source, format!("{directory}/{name}"))? else {
                continue;
            };
            let observed = observations
                .iter()
                .find(|observation| observation.record == snapshot.run.path);
            runs.push(snapshot.into_run(
                repo,
                observed,
                signature_verifier,
                current_formal_rule_status,
            ));
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

struct ParsedFile<T> {
    path: String,
    bytes: Vec<u8>,
    value: T,
}

struct RunSnapshot {
    run: ParsedFile<RunRecord>,
    projection: Option<ParsedFile<serde_json::Value>>,
}

impl RunSnapshot {
    fn read(source: &DiskEvidence, path: String) -> Result<Option<Self>, String> {
        let Some(bytes) = source
            .read_bytes(&path)
            .map_err(|error| error.to_string())?
        else {
            return Err(format!("listed evidence record disappeared: {path}"));
        };
        let record = serde_json::from_slice::<RunRecord>(&bytes)
            .map_err(|error| format!("{path} is unreadable: {error}"))?;
        validate_run_record(&record).map_err(|error| format!("{path} is invalid: {error}"))?;
        if record_path(RUNS_DIR, &record.suite, &record.timestamp) != path {
            return Err(format!(
                "{path} does not match its record suite and timestamp"
            ));
        }
        let projection_path = record_path(ANALYSIS_REPORTS_DIR, &record.suite, &record.timestamp);
        let projection = if let Some(bytes) = source
            .read_bytes(&projection_path)
            .map_err(|error| error.to_string())?
        {
            let value = serde_json::from_slice(&bytes)
                .map_err(|error| format!("{projection_path} is malformed: {error}"))?;
            Some(ParsedFile {
                path: projection_path,
                bytes,
                value,
            })
        } else {
            None
        };
        Ok(Some(Self {
            run: ParsedFile {
                path,
                bytes,
                value: record,
            },
            projection,
        }))
    }

    fn pair(&self) -> Option<EvidencePair<'_>> {
        self.projection.as_ref().map(|projection| EvidencePair {
            run_path: &self.run.path,
            run_bytes: &self.run.bytes,
            projection_path: &projection.path,
            projection_bytes: &projection.bytes,
        })
    }

    fn into_run(
        self,
        repo: &str,
        input: Option<&RunObservationFile>,
        signature_verifier: Option<&SignatureVerifier>,
        current_formal_rule_status: Option<&serde_json::Value>,
    ) -> RetainedRun {
        let observation = input.map_or_else(RunObservation::default, |input| {
            run_observation(
                repo,
                input,
                self.pair(),
                signature_verifier,
                current_formal_rule_status,
            )
        });
        RetainedRun {
            file: self.run.path,
            record: self.run.value,
            report: self.projection.map(|projection| projection.value),
            observation,
        }
    }
}

#[derive(Clone, Copy)]
struct EvidencePair<'a> {
    run_path: &'a str,
    run_bytes: &'a [u8],
    projection_path: &'a str,
    projection_bytes: &'a [u8],
}

fn run_observation(
    repo: &str,
    input: &RunObservationFile,
    pair: Option<EvidencePair<'_>>,
    signature_verifier: Option<&SignatureVerifier>,
    current_formal_rule_status: Option<&serde_json::Value>,
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
    let (actor, actor_rejection) = pair.map_or((None, None), |pair| {
        actor_observation(repo, input, pair, signature_verifier)
    });
    RunObservation {
        run_kind: input.run_kind.clone(),
        performed_revision,
        execution,
        actor,
        actor_rejection,
        current_formal_rule_status: current_formal_rule_status.cloned(),
        selected_target: input.selected_target.clone(),
    }
}

fn fresh_formal_rule_status(
    repo: &str,
    module_paths: &[String],
    signature_verifier: Option<&SignatureVerifier>,
) -> Option<serde_json::Value> {
    let verifier = signature_verifier?;
    let mut argv = vec![
        "formal-rules".to_owned(),
        "status".to_owned(),
        "--json".to_owned(),
        "--repo".to_owned(),
        repo.to_owned(),
        "--engineering-assurance".to_owned(),
        verifier.executable.display().to_string(),
        "--allowed-signers".to_owned(),
        verifier.allowed_signers.display().to_string(),
    ];
    if let Some(gitsign) = &verifier.gitsign {
        for (field, flag) in [
            ("certificate_identity", "--gitsign-certificate-identity"),
            (
                "certificate_oidc_issuer",
                "--gitsign-certificate-oidc-issuer",
            ),
            ("principal", "--gitsign-principal"),
        ] {
            argv.push(flag.to_owned());
            argv.push(gitsign.get(field)?.as_str()?.to_owned());
        }
    }
    let matches = crate::formal_rules::command()
        .try_get_matches_from(argv)
        .ok()?;
    let (_, args) = matches.subcommand()?;
    crate::formal_rules::status_report_with_modules(args, module_paths).ok()
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
        .and_then(|decisions| {
            // Trust decisions are append-only, so a revalidation is a new
            // decision: the one with the latest `decided_at` governs the use.
            // Two distinct decisions sharing that instant are ambiguous, and an
            // ambiguous reliance is no reliance (FR-093-AC-9).
            let mut matches = decisions
                .into_iter()
                .filter(|decision| {
                    decision.r#use.id == quoin_evidence::qualify::POSITIVE_USE_DECISION
                })
                .collect::<Vec<_>>();
            matches.sort_by(|a, b| a.decided_at.cmp(&b.decided_at));
            let latest = matches.pop()?;
            let tied = matches
                .last()
                .is_some_and(|previous| previous.decided_at == latest.decided_at);
            (!tied).then_some(latest)
        });
    HostObservations {
        reliance,
        selected_head,
        selected_head_conflict,
    }
}

fn actor_observation(
    repo: &str,
    input: &RunObservationFile,
    pair: EvidencePair<'_>,
    signature_verifier: Option<&SignatureVerifier>,
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
    let Some(commit_id) = first_pair_introduction(repo, pair) else {
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
    let Some(signature_verifier) = signature_verifier else {
        return (None, None);
    };
    let Ok(actual_signature) = crate::formal_rules::verify_for_evidence(
        &signature_verifier.executable,
        &signature_verifier.allowed_signers,
        Path::new(repo),
        &commit_id,
        signature_verifier.gitsign.as_ref(),
    ) else {
        return (None, None);
    };
    if !same_signature_claim(signature, &actual_signature) {
        return (
            None,
            Some("the supplied signature result does not match host verification".to_owned()),
        );
    }
    let principal = match signature_signer(&actual_signature) {
        SignatureSigner::Admitted(principal) => principal,
        SignatureSigner::Rejected(reason) => return (None, Some(reason)),
        SignatureSigner::Unavailable => return (None, None),
    };
    if authority.actor != principal {
        return (
            None,
            Some("actor authority does not match the mapped signer principal".to_owned()),
        );
    }
    (
        Some(ActorAdmission {
            actor: principal,
            admission_ref: authority.admission_ref.clone(),
            authorized: authority.authorized.clone(),
        }),
        None,
    )
}

fn same_signature_claim(supplied: &serde_json::Value, verified: &serde_json::Value) -> bool {
    ["protocol", "kind", "status", "verified_commit_id", "signer"]
        .into_iter()
        .all(|field| supplied.get(field) == verified.get(field))
}

enum SignatureSigner {
    Admitted(String),
    Rejected(String),
    Unavailable,
}

fn signature_signer(signature: &serde_json::Value) -> SignatureSigner {
    let Some(status) = signature.get("status").and_then(serde_json::Value::as_str) else {
        return SignatureSigner::Unavailable;
    };
    match status {
        "signed_valid" => {}
        "signed_untrusted" | "signed_bad" | "unsigned" => {
            return SignatureSigner::Rejected(format!(
                "the introducing commit signature status is {status}"
            ));
        }
        _ => return SignatureSigner::Unavailable,
    }
    let Some(signer) = signature
        .get("signer")
        .and_then(serde_json::Value::as_object)
    else {
        return SignatureSigner::Unavailable;
    };
    let Some(mapping) = signer.get("mapping").and_then(serde_json::Value::as_str) else {
        return SignatureSigner::Unavailable;
    };
    match mapping {
        "mapped" => {}
        "unmapped" => {
            return SignatureSigner::Rejected(
                "the introducing commit signature has no mapped signer principal".to_owned(),
            );
        }
        _ => return SignatureSigner::Unavailable,
    }
    let Some(principal_value) = signer.get("principal") else {
        return SignatureSigner::Rejected(
            "the introducing commit signature has no signer principal".to_owned(),
        );
    };
    let Some(principal) = principal_value.as_str() else {
        return if principal_value.is_null() {
            SignatureSigner::Rejected(
                "the introducing commit signature has no signer principal".to_owned(),
            )
        } else {
            SignatureSigner::Unavailable
        };
    };
    if principal.is_empty() {
        return SignatureSigner::Rejected(
            "the introducing commit signature has no signer principal".to_owned(),
        );
    }
    SignatureSigner::Admitted(principal.to_owned())
}

fn first_pair_introduction(repo: &str, pair: EvidencePair<'_>) -> Option<String> {
    let run_path = format!("spec/evidence/{}", pair.run_path);
    let projection_path = format!("spec/evidence/{}", pair.projection_path);
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
                break;
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
    if !additions.contains(&run_path) || !additions.contains(&projection_path) {
        return None;
    }
    (git_show(repo, &id, &run_path).as_deref() == Some(pair.run_bytes)
        && git_show(repo, &id, &projection_path).as_deref() == Some(pair.projection_bytes))
    .then_some(id)
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
    use super::{
        RunSnapshot, SignatureVerifier, first_pair_introduction, fresh_formal_rule_status,
        host_observations,
    };
    use crate::run_observation::RunObservationFile;
    use ix_trace_rs::trace;
    use quoin_evidence::qualify::RetainedRun;
    use quoin_evidence::source::DiskEvidence;
    use std::fs;
    use std::process::Command;

    const RUN: &str = "runs/unit/2026-11-13T00~3A00~3A00Z.json";
    const PROJECTION: &str = "analysis-reports/unit/2026-11-13T00~3A00~3A00Z.json";
    const RUN_BYTES: &[u8] = br#"{"schemaVersion":1,"suite":"unit","tool":"fixture","timestamp":"2026-11-13T00:00:00Z","entries":[{"symbol":"analysis-result","outcome":"pass","traceIds":["FR-001-AC-1"]}]}"#;
    const PROJECTION_BYTES: &[u8] = br#"{"protocol":"quoin.analysis-report-projection/v1"}"#;
    const SUPPORTED_TEST_EA_VERSION: &str = "engineering-assurance 0.10.1";

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

    #[cfg(unix)]
    fn executable(directory: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = directory.join(name);
        fs::write(
            &path,
            format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'engineering-assurance 0.10.1'; exit 0; fi\n{body}\n"),
        )
        .expect("write verifier");
        let mut permissions = fs::metadata(&path)
            .expect("verifier metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("verifier executable");
        path
    }

    #[cfg(unix)]
    #[trace("FR-184-AC-25")]
    #[test]
    fn the_host_refreshes_formal_rule_status_and_fails_closed_when_verification_is_unavailable() {
        let repo = tempfile::tempdir().expect("temporary repository");
        fs::write(
            repo.path().join("manifest.yaml"),
            "name: status-refresh-fixture\nversion: 0.1.0\ndescription: Status refresh fixture\nartifact_types:\n  - name: FR\ntraceability:\n  trace_tags:\n    formalizes:\n      - name: formalizes\n        language: qsl\n        syntax: comment\n        pattern: '^// formalizes: (.+)$'\n",
        )
        .expect("module manifest");
        fs::write(
            repo.path().join("a.native"),
            "language qsl\nprofile core\n// formalizes: FR-001-AC-1\nrule P1 { check x }\n",
        )
        .expect("formalized rule");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.name", "Quoin status test"]);
        git(
            repo.path(),
            &["config", "user.email", "quoin-status@example.invalid"],
        );
        git(repo.path(), &["add", "manifest.yaml", "a.native"]);
        git(repo.path(), &["commit", "-q", "-m", "draft formal rule"]);
        git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "Formal rule approval",
                "-m",
                "Formal-rule: qsl:a.native::P1",
            ],
        );
        let allowed_signers = repo.path().join("allowed-signers");
        fs::write(&allowed_signers, "fixture\n").expect("allowed signers");
        let verifier = executable(
            repo.path(),
            "controlled-ea",
            "cat >/dev/null\nprintf '%s\\n' '{\"protocol\":\"engineering-assurance.signature-verify-result/v1\",\"kind\":\"commit\",\"status\":\"unsigned\",\"verified_commit_id\":\"stale-id\",\"trailers\":{\"Formal-rule\":[\"qsl:a.native::P1\"]}}'",
        );
        let verifier = SignatureVerifier {
            executable: verifier,
            allowed_signers: allowed_signers.clone(),
            gitsign: None,
        };
        let repo_path = repo.path().to_str().expect("repository path");
        let direct_arguments = crate::formal_rules::command()
            .try_get_matches_from([
                "formal-rules",
                "status",
                "--json",
                "--repo",
                repo_path,
                "--engineering-assurance",
                verifier.executable.to_str().expect("verifier path"),
                "--allowed-signers",
                verifier
                    .allowed_signers
                    .to_str()
                    .expect("allowed signers path"),
            ])
            .expect("direct formal status arguments");
        let (_, direct_status) = direct_arguments.subcommand().expect("formal status");
        let direct_report = crate::formal_rules::status_report_with_modules(direct_status, &[]);
        if let Err(crate::core_runtime::Failure::Message(message)) = &direct_report {
            panic!("host status failed: {message}");
        }
        assert!(direct_report.is_ok(), "host status returned a refusal");
        let refreshed = fresh_formal_rule_status(repo_path, &[], Some(&verifier))
            .expect("current status is re-derived");
        assert_eq!(refreshed["protocol"], "quoin.formal-rule-status/v1");
        assert_eq!(refreshed["rules"][0]["state"], "draft");
        assert_eq!(refreshed["rules"][0]["reason"], "approval_unverifiable");

        let unavailable = SignatureVerifier {
            executable: executable(repo.path(), "unavailable-ea", "exit 9"),
            allowed_signers,
            gitsign: None,
        };
        assert!(fresh_formal_rule_status(repo_path, &[], Some(&unavailable)).is_none());
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
        fs::write(&run, RUN_BYTES).expect("run bytes");
        fs::write(&projection, PROJECTION_BYTES).expect("projection bytes");
        git(repo.path(), &["add", "spec/evidence"]);
        git(repo.path(), &["commit", "-q", "-m", "add paired evidence"]);
        let commit = git(repo.path(), &["rev-parse", "HEAD"]);
        (repo, commit)
    }

    fn snapshot(repo: &std::path::Path) -> RunSnapshot {
        RunSnapshot::read(&DiskEvidence::new(repo), RUN.to_owned())
            .expect("retained run is readable")
            .expect("retained run is canonical")
    }

    #[trace("FR-030-AC-29")]
    #[test]
    fn tc_plat_1794_run_snapshot_refuses_invalid_utf8_instead_of_dropping_record() {
        let repo = tempfile::tempdir().expect("temporary repository");
        let path = repo.path().join("spec/evidence").join(RUN);
        fs::create_dir_all(path.parent().expect("run parent")).expect("run directory");
        fs::write(&path, [b'{', 0xff, b'}']).expect("invalid UTF-8 record bytes");

        let result = RunSnapshot::read(&DiskEvidence::new(repo.path()), RUN.to_owned());
        let Err(error) = result else {
            panic!("invalid UTF-8 evidence was silently dropped");
        };
        assert!(error.contains("unreadable"), "{error}");
    }

    #[trace("FR-030-AC-13", "FR-030-AC-29")]
    #[test]
    fn tc_plat_1794_run_snapshot_reports_a_listed_record_that_disappeared() {
        let repo = tempfile::tempdir().expect("temporary repository");
        let path = repo.path().join("spec/evidence").join(RUN);
        fs::create_dir_all(path.parent().expect("run parent")).expect("run directory");
        fs::write(&path, RUN_BYTES).expect("valid run bytes");
        let source = DiskEvidence::new(repo.path());
        fs::remove_file(path).expect("record disappeared after listing");

        let result = RunSnapshot::read(&source, RUN.to_owned());
        let Err(error) = result else {
            panic!("missing retained evidence was silently dropped");
        };
        assert!(error.contains("disappeared"), "{error}");
    }

    fn observed_run(
        repo: &std::path::Path,
        input: &RunObservationFile,
        verifier: Option<&SignatureVerifier>,
    ) -> RetainedRun {
        snapshot(repo).into_run(
            repo.to_str().expect("repo path"),
            Some(input),
            verifier,
            None,
        )
    }

    fn observe_actor(
        repo: &std::path::Path,
        input: &RunObservationFile,
        verifier: Option<&SignatureVerifier>,
    ) -> (Option<super::ActorAdmission>, Option<String>) {
        let observation = observed_run(repo, input, verifier).observation;
        (observation.actor, observation.actor_rejection)
    }

    fn explicit_supported_ea() -> std::path::PathBuf {
        let executable = std::env::var_os("QUOIN_TEST_ENGINEERING_ASSURANCE")
            .map(std::path::PathBuf::from)
            .expect("set QUOIN_TEST_ENGINEERING_ASSURANCE to run ignored real-verifier tests");
        assert!(
            executable.is_absolute() && executable.is_file(),
            "QUOIN_TEST_ENGINEERING_ASSURANCE must name an existing absolute executable"
        );
        let version = Command::new(&executable)
            .arg("--version")
            .output()
            .expect("the supplied Engineering Assurance executable runs");
        assert!(version.status.success(), "EA --version failed");
        assert!(
            String::from_utf8_lossy(&version.stdout).trim() == SUPPORTED_TEST_EA_VERSION,
            "expected {SUPPORTED_TEST_EA_VERSION}, received: {}",
            String::from_utf8_lossy(&version.stdout)
        );
        let capability = Command::new(&executable)
            .args(["signature-verify", "--help"])
            .output()
            .expect("the supplied Engineering Assurance executable runs");
        assert!(
            capability.status.success(),
            "the supplied EA executable does not support signature-verify: {}",
            String::from_utf8_lossy(&capability.stderr)
        );
        executable
    }

    fn signed_pair_repo() -> (
        tempfile::TempDir,
        String,
        tempfile::TempDir,
        SignatureVerifier,
    ) {
        let key_dir = tempfile::tempdir().expect("temporary SSH keys");
        let private_key = key_dir.path().join("signing_key");
        let generated = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&private_key)
            .output()
            .expect("ssh-keygen runs");
        assert!(generated.status.success());
        let public = Command::new("ssh-keygen")
            .args(["-y", "-f"])
            .arg(&private_key)
            .output()
            .expect("public key derives");
        assert!(public.status.success());
        let public = String::from_utf8(public.stdout).expect("UTF-8 public key");
        let allowed_signers = key_dir.path().join("allowed_signers");
        fs::write(
            &allowed_signers,
            format!("alice@example.invalid namespaces=\"git\" {public}"),
        )
        .expect("allowed signers file");

        let repo = tempfile::tempdir().expect("temporary git repository");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.name", "VER-97 test"]);
        git(
            repo.path(),
            &["config", "user.email", "ver97@example.invalid"],
        );
        git(repo.path(), &["config", "gpg.format", "ssh"]);
        git(
            repo.path(),
            &[
                "config",
                "user.signingkey",
                private_key.to_str().expect("key path"),
            ],
        );
        git(repo.path(), &["config", "commit.gpgsign", "true"]);
        let run = repo.path().join("spec/evidence").join(RUN);
        let projection = repo.path().join("spec/evidence").join(PROJECTION);
        fs::create_dir_all(run.parent().expect("run parent")).expect("run directory");
        fs::create_dir_all(projection.parent().expect("projection parent"))
            .expect("projection directory");
        fs::write(&run, RUN_BYTES).expect("run bytes");
        fs::write(&projection, PROJECTION_BYTES).expect("projection bytes");
        git(repo.path(), &["add", "spec/evidence"]);
        git(repo.path(), &["commit", "-q", "-m", "add paired evidence"]);
        let commit = git(repo.path(), &["rev-parse", "HEAD"]);
        let executable = explicit_supported_ea();
        let verifier = SignatureVerifier {
            executable,
            allowed_signers,
            gitsign: None,
        };
        (repo, commit, key_dir, verifier)
    }

    fn observation(signature_result: &serde_json::Value) -> RunObservationFile {
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
    #[ignore = "real verifier integration; set QUOIN_TEST_ENGINEERING_ASSURANCE and pass --ignored"]
    #[test]
    fn exact_pair_introduction_commit_admits_mapped_actor() {
        let (repo, commit, _keys, verifier) = signed_pair_repo();
        let actual_signature = crate::formal_rules::verify_for_evidence(
            &verifier.executable,
            &verifier.allowed_signers,
            repo.path(),
            &commit,
            None,
        )
        .expect("the actual commit signature verifies");
        assert_eq!(
            first_pair_introduction(
                repo.path().to_str().expect("repo path"),
                snapshot(repo.path()).pair().expect("projection present"),
            ),
            Some(commit.clone())
        );
        let (actor, rejection) = observe_actor(
            repo.path(),
            &observation(&actual_signature),
            Some(&verifier),
        );
        assert_eq!(
            actor.expect("verified signature is admitted").actor,
            "alice@example.invalid"
        );
        assert_eq!(rejection, None);

        let untrusted = serde_json::json!({
            "protocol": "engineering-assurance.signature-verify-result/v1",
            "status": "signed_untrusted",
            "kind": "commit",
            "verified_commit_id": commit,
            "signer": {"mapping": "unmapped"},
        });
        let (actor, rejection) =
            observe_actor(repo.path(), &observation(&untrusted), Some(&verifier));
        assert!(actor.is_none());
        assert!(rejection.is_some());

        git(
            repo.path(),
            &["commit", "--allow-empty", "-S", "-q", "-m", "later commit"],
        );
        let later = git(repo.path(), &["rev-parse", "HEAD"]);
        assert_ne!(later, commit);
        let later_signature = crate::formal_rules::verify_for_evidence(
            &verifier.executable,
            &verifier.allowed_signers,
            repo.path(),
            &later,
            None,
        )
        .expect("later signature verifies");
        let (actor, _) =
            observe_actor(repo.path(), &observation(&later_signature), Some(&verifier));
        assert!(
            actor.is_none(),
            "later tree-equivalent signature is not pair-bound"
        );
    }

    #[trace("FR-153-AC-16")]
    #[ignore = "real verifier integration; set QUOIN_TEST_ENGINEERING_ASSURANCE and pass --ignored"]
    #[test]
    fn actor_admission_uses_the_parsed_pair_and_mapped_actor() {
        use quoin_evidence::qualify::{
            self, Cause, HostObservations, Premise, Qualification, Request,
        };

        let (repo, commit, _keys, verifier) = signed_pair_repo();
        let signature = crate::formal_rules::verify_for_evidence(
            &verifier.executable,
            &verifier.allowed_signers,
            repo.path(),
            &commit,
            None,
        )
        .expect("the actual signature verifies");
        let observed = observation(&signature);
        let control = observed_run(repo.path(), &observed, Some(&verifier));
        let admitted = control
            .observation
            .actor
            .expect("healthy signed pair is admitted");
        assert_eq!(admitted.actor, "alice@example.invalid");
        assert_eq!(control.observation.actor_rejection, None);

        let mut other_actor = observed.clone();
        other_actor
            .actor_authority
            .as_mut()
            .expect("actor authority")
            .actor = "another@example.invalid".to_owned();
        let (actor, rejection) = observe_actor(repo.path(), &other_actor, Some(&verifier));
        assert!(actor.is_none());
        assert_eq!(
            rejection.as_deref(),
            Some("actor authority does not match the mapped signer principal"),
        );

        let captured = snapshot(repo.path());
        fs::write(
            repo.path().join("spec/evidence").join(PROJECTION),
            [PROJECTION_BYTES, b"\n"].concat(),
        )
        .expect("store a byte-distinct projection with the same parsed value");
        let replay = captured.into_run(
            repo.path().to_str().expect("repo path"),
            Some(&observed),
            Some(&verifier),
            None,
        );
        assert_eq!(replay.report, control.report);
        assert_eq!(replay.observation.actor, Some(admitted));
        assert_eq!(replay.observation.actor_rejection, None);

        let current = observed_run(repo.path(), &observed, Some(&verifier));
        assert_eq!(current.report, control.report);
        assert!(current.observation.actor.is_none());
        assert_eq!(current.observation.actor_rejection, None);
        let answer = qualify::qualify(&Request {
            obligation: "FR-001-AC-1",
            declared_method: Some("Analysis"),
            runs: &[current],
            audit: None,
            host: &HostObservations::default(),
        });
        assert!(
            matches!(&answer, Qualification::NotQualified(refusals) if refusals.iter().any(|refusal| {
                refusal.cause == Cause::Unevaluated
                    && refusal.premise == Some(Premise::ActorAuthority)
                    && refusal.detail == "actor authority premise: no signer admitted for the record"
            })),
            "{answer:?}"
        );
    }

    /// FR-116-AC-18: copied success fields cannot substitute for host verification.
    #[trace("FR-116-AC-18")]
    #[test]
    fn self_asserted_signed_valid_result_remains_unevaluated() {
        let (repo, commit) = pair_repo();
        let (actor, rejection) =
            observe_actor(repo.path(), &observation(&signature(&commit)), None);
        assert!(actor.is_none());
        assert!(rejection.is_none());
    }

    /// FR-116-AC-18: an unsigned introducing commit is rejected even when the
    /// host receives a result produced by the real verifier.
    #[trace("FR-116-AC-18")]
    #[ignore = "real verifier integration; set QUOIN_TEST_ENGINEERING_ASSURANCE and pass --ignored"]
    #[test]
    fn actual_verifier_refuses_an_unsigned_pair_introduction() {
        let (repo, commit) = pair_repo();
        let (_signed_repo, _signed_commit, _keys, verifier) = signed_pair_repo();
        let actual_signature = crate::formal_rules::verify_for_evidence(
            &verifier.executable,
            &verifier.allowed_signers,
            repo.path(),
            &commit,
            None,
        )
        .expect("the verifier returns an explicit unsigned result");
        assert_eq!(
            actual_signature
                .get("status")
                .and_then(serde_json::Value::as_str),
            Some("unsigned"),
        );

        let (actor, rejection) = observe_actor(
            repo.path(),
            &observation(&actual_signature),
            Some(&verifier),
        );
        assert!(actor.is_none());
        assert!(rejection.unwrap().contains("status is unsigned"));
    }

    /// FR-153-AC-18: every missing or wrong revision binding and every tag
    /// result leaves actor authority unavailable.
    #[trace("FR-153-AC-18")]
    #[ignore = "real verifier integration; set QUOIN_TEST_ENGINEERING_ASSURANCE and pass --ignored"]
    #[test]
    fn absent_null_malformed_mismatched_tag_and_unresolved_ids_fail_closed() {
        let (repo, _, _keys, verifier) = signed_pair_repo();
        let cases = vec![
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid"}),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid", "verified_commit_id":null}),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"signed_valid", "verified_commit_id":"not-an-object-id"}),
            signature("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"tag", "status":"signed_valid", "verified_commit_id":null}),
            serde_json::json!({"protocol":"engineering-assurance.signature-verify-result/v1", "kind":"commit", "status":"unverifiable", "reason":"revision_unresolved", "verified_commit_id":null}),
        ];
        for signature_result in cases {
            let (actor, rejection) = observe_actor(
                repo.path(),
                &observation(&signature_result),
                Some(&verifier),
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
        fs::write(run, RUN_BYTES).expect("uncommitted run bytes");
        fs::write(projection, PROJECTION_BYTES).expect("uncommitted projection bytes");
        let (actor, rejection) = observe_actor(
            repo.path(),
            &observation(&signature("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
            None,
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

    fn trust_decision(
        id: &str,
        decided_at: &str,
        relied: bool,
    ) -> quoin_evidence::types::TrustDecision {
        let context = serde_json::json!({
            "name": "quire", "version": "1.0", "configuration": "c",
            "validationCorpus": "v", "inputContract": "i", "environment": "e",
        });
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "use": {
                "id": quoin_evidence::qualify::POSITIVE_USE_DECISION,
                "intendedFunction": "qualify an analysis act",
                "permittedDecisions": [quoin_evidence::qualify::POSITIVE_USE_DECISION],
            },
            "decision": if relied { "relied-upon" } else { "not-relied-upon" },
            "acceptedContext": context,
            "observedContext": context,
            "revalidateOn": [
                "producer-version", "configuration", "validation-corpus",
                "input-contract", "environment",
            ],
            "validationEvidence": [{"id": "V-1", "reference": "evidence/v-1"}],
            "limitations": [],
            "owner": "owner",
            "decidedAt": decided_at,
        }))
        .expect("typed trust decision")
    }

    /// The decision with the latest `decidedAt` governs; a tie is no reliance.
    #[trace("FR-093-AC-9")]
    #[test]
    fn the_latest_trust_decision_for_the_use_governs() {
        let repo = tempfile::tempdir().expect("repository");
        let path = repo.path().to_str().expect("repo path");
        let mut store = quoin_evidence::DiskEvidence::new(repo.path());
        quoin_evidence::store::write_trust_decision(
            &mut store,
            &trust_decision("ETD-1", "2026-01-01T00:00:00Z", true),
        )
        .expect("first decision");
        assert_eq!(
            host_observations(path, &[])
                .reliance
                .expect("reliance")
                .id
                .as_str(),
            "ETD-1"
        );

        // A revocation is a new decision, and it governs.
        quoin_evidence::store::write_trust_decision(
            &mut store,
            &trust_decision("ETD-2", "2026-02-01T00:00:00Z", false),
        )
        .expect("revocation");
        let governing = host_observations(path, &[]).reliance.expect("reliance");
        assert_eq!(governing.id.as_str(), "ETD-2");

        // Two distinct decisions at the latest instant are ambiguous.
        quoin_evidence::store::write_trust_decision(
            &mut store,
            &trust_decision("ETD-3", "2026-02-01T00:00:00Z", true),
        )
        .expect("tied decision");
        assert!(host_observations(path, &[]).reliance.is_none());
    }
}
