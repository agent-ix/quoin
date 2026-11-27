// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Read-only, git-derived formal-rule approval status (VER-101).

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicU64, Ordering};

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_core::protocol::Response;

use crate::core_runtime::Failure;
use quoin_quire::ScopeRoot;
use serde_json::{Value, json};

const REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";

#[implements("FR-190-AC-7")]
pub(crate) fn command() -> Command {
    Command::new("formal-rules")
        .about("Read formal-rule approval state from signed git commits")
        .subcommand_required(true)
        .subcommand(with_trust_arguments(
            Command::new("status")
                .about("Report the current approval state of formalized QSL declarations")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .value_name("DIR")
                        .default_value("."),
                )
                .arg(
                    Arg::new("select-out")
                        .long("select-out")
                        .value_name("ABSOLUTE-PATH"),
                )
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        ))
        .subcommand(
            with_trust_arguments(crate::formal_rules_coverage::command())
                .arg(Arg::new("module").long("module").action(ArgAction::Append))
                .arg(
                    Arg::new("run-observation")
                        .long("run-observation")
                        .action(ArgAction::Append)
                        .num_args(1)
                        .value_name("FILE"),
                ),
        )
}

fn with_trust_arguments(command: Command) -> Command {
    command.args(trust_arguments())
}

pub(crate) fn trust_arguments() -> [Arg; 5] {
    [
        Arg::new("engineering-assurance")
            .long("engineering-assurance")
            .value_name("ABSOLUTE-PATH"),
        Arg::new("allowed-signers")
            .long("allowed-signers")
            .value_name("ABSOLUTE-PATH"),
        Arg::new("gitsign-certificate-identity")
            .long("gitsign-certificate-identity")
            .value_name("IDENTITY"),
        Arg::new("gitsign-certificate-oidc-issuer")
            .long("gitsign-certificate-oidc-issuer")
            .value_name("ISSUER"),
        Arg::new("gitsign-principal")
            .long("gitsign-principal")
            .value_name("PRINCIPAL"),
    ]
}

#[implements(
    "FR-190-AC-2",
    "FR-190-AC-3",
    "FR-190-AC-4",
    "FR-190-AC-5",
    "FR-190-AC-6",
    "FR-190-AC-7",
    "FR-190-AC-8",
    "FR-190-AC-9",
    "FR-190-AC-10",
    "FR-190-AC-11"
)]
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, crate::ShellError> {
    let Some((name, args)) = matches.subcommand() else {
        return Err(crate::ShellError::usage(
            "an unknown formal-rules command reached dispatch",
        ));
    };
    if name == "coverage" {
        return crate::formal_rules_coverage::run(args)
            .map_err(crate::ShellError::unavailable_invalid);
    }
    if name != "status" {
        return Err(crate::ShellError::usage(
            "an unknown formal-rules command reached dispatch",
        ));
    }
    let select_out = select_out_path(args).map_err(status_shell_error)?;
    let report = match status_report_shell(args) {
        Ok(report) => report,
        Err(error) => {
            if let Some(path) = &select_out {
                remove_select_out(path).map_err(|cleanup| {
                    crate::ShellError::usage_refused(format!(
                        "cannot remove stale --select-out {}: {cleanup}",
                        path.display()
                    ))
                })?;
            }
            return settle_status(error);
        }
    };
    if let Some(path) = select_out {
        let bytes = quoin_evidence::adapters::build_approved_claim_selection(&report, "quoin")
            .map_err(|error| error.to_string())
            .and_then(|selection| {
                serde_json::to_vec(&selection).map_err(|error| error.to_string())
            });
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                remove_select_out(&path).map_err(|cleanup| {
                    crate::ShellError::usage_refused(format!(
                        "cannot remove stale --select-out {}: {cleanup}",
                        path.display()
                    ))
                })?;
                return Err(crate::ShellError::usage_refused(error));
            }
        };
        if let Err(error) = write_selection_atomic(&path, &bytes) {
            remove_select_out(&path).map_err(|cleanup| {
                crate::ShellError::usage_refused(format!(
                    "cannot remove incomplete --select-out {} after write failure: {cleanup}",
                    path.display()
                ))
            })?;
            return Err(crate::ShellError::usage_refused(format!(
                "cannot write --select-out {}: {error}",
                path.display()
            )));
        }
    }
    let rendered = if args.get_flag("json") {
        serde_json::to_string(&report)
            .map_err(|error| crate::ShellError::unavailable(error.to_string()))?
    } else {
        let mut lines = Vec::new();
        for rule in report
            .get("rules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let state = rule.get("state").and_then(Value::as_str).unwrap_or("draft");
            let symbol = rule.get("symbol").and_then(Value::as_str).unwrap_or("");
            let reason = rule
                .get("reason")
                .and_then(Value::as_str)
                .map(|reason| format!(" ({reason})"))
                .unwrap_or_default();
            lines.push(format!("{state} {symbol}{reason}"));
        }
        lines.join("\n")
    };
    Ok(Response::ok(json!({"rendered": rendered})))
}

#[implements("FR-190-AC-7")]
fn select_out_path(args: &ArgMatches) -> Result<Option<PathBuf>, StatusError> {
    let Some(value) = args.get_one::<String>("select-out") else {
        return Ok(None);
    };
    if value.is_empty() || value.contains('\0') || !Path::new(value).is_absolute() {
        return Err(StatusError::Invalid(
            "--select-out must be a non-empty absolute path".to_owned(),
        ));
    }
    Ok(Some(PathBuf::from(value)))
}

#[implements("FR-190-AC-8", "FR-190-AC-9")]
fn remove_select_out(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[implements("FR-190-AC-10")]
fn write_selection_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path has no parent",
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path has no file name",
        )
    })?;

    for _ in 0..16 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(
            ".{}.quoin-selection.{}.{}.tmp",
            name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let write_result = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary selection path",
    ))
}

pub(crate) fn status_report_with_modules(
    args: &ArgMatches,
    module_paths: &[String],
) -> Result<Value, Failure> {
    validate_status_arguments(args)
        .map_err(|error| Failure::Message(status_error_message(error)))?;
    status(args, module_paths).map_err(status_failure)
}

/// Read the current QSL formalizes relations for assurance's FR-204 state
/// alignment without invoking signature verification or enabling signed
/// approvals on the selected repository.
pub(crate) fn assurance_declarations(
    repo: &Path,
    module_paths: &[String],
) -> Result<Vec<Value>, Failure> {
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let scope = ScopeRoot::open(&repo).map_err(|error| error.to_string())?;
    let mut notices = Vec::new();
    let selection = crate::analysis_evidence::module_selection(module_paths)?;
    let registry = selection
        .resolve(&scope, &mut notices)
        .map_err(|error| error.to_string())?;
    // Before the model is read and before the absent-model answer below: a
    // model that lost every declaration reads as absent (FR-029).
    quoin_quire::modules::require_complete_model(&registry)
        .map_err(|error| Failure::from_quire(&error, "formal_rules.assurance_declarations"))?;
    let model = registry
        .traceability()
        .ok_or_else(|| "no active module declares a traceability model".to_owned())?;
    let current = revision_rules(&repo, "HEAD", model)?;
    Ok(current
        .into_iter()
        .map(|rule| {
            json!({
                "symbol": rule.symbol,
                "formalizes": rule.formalizes,
                "state": "draft",
                "reason": "approval state is not consulted by quoin assurance"
            })
        })
        .collect())
}

fn status_report_shell(args: &ArgMatches) -> Result<Value, StatusStop> {
    validate_status_arguments(args)?;
    status(args, &[])
}

fn validate_status_arguments(args: &ArgMatches) -> Result<(), StatusError> {
    for name in ["engineering-assurance", "allowed-signers"] {
        let Some(value) = args.get_one::<String>(name) else {
            return Err(StatusError::Invalid(format!("--{name} is required")));
        };
        if value.is_empty() || value.contains('\0') || !Path::new(value).is_absolute() {
            return Err(StatusError::Invalid(format!(
                "--{name} must be an absolute path"
            )));
        }
    }
    let gitsign_values = [
        args.get_one::<String>("gitsign-certificate-identity"),
        args.get_one::<String>("gitsign-certificate-oidc-issuer"),
        args.get_one::<String>("gitsign-principal"),
    ];
    if !matches!(
        gitsign_values,
        [None, None, None] | [Some(_), Some(_), Some(_)]
    ) {
        return Err(StatusError::Invalid(
            "all three gitsign identity options must be supplied together".to_owned(),
        ));
    }
    if gitsign_values
        .iter()
        .flatten()
        .any(|value| !valid_gitsign_value(value))
        || gitsign_values[0].is_some_and(|value| value.starts_with('-'))
        || gitsign_values[1].is_some_and(|value| value.starts_with('-'))
    {
        return Err(StatusError::Invalid(
            "gitsign identity values violate FR-040 validation".to_owned(),
        ));
    }
    Ok(())
}

fn status_error_message(error: StatusError) -> String {
    match error {
        StatusError::Invalid(message) | StatusError::Unavailable(message) => message,
    }
}

pub(crate) enum StatusError {
    Invalid(String),
    Unavailable(String),
}

fn status_shell_error(error: StatusError) -> crate::ShellError {
    match error {
        StatusError::Invalid(message) => crate::ShellError::usage_refused(message),
        StatusError::Unavailable(message) => crate::ShellError::unavailable_invalid(message),
    }
}

impl From<String> for StatusError {
    fn from(message: String) -> Self {
        Self::Unavailable(message)
    }
}

/// Why a status read stopped: a failure the shell reports, or the `QQ-1022`
/// refusal (FR-029), which is the command's own answer.
enum StatusStop {
    Failed(StatusError),
    Refused(Response),
}

impl From<StatusError> for StatusStop {
    fn from(error: StatusError) -> Self {
        Self::Failed(error)
    }
}

impl From<String> for StatusStop {
    fn from(message: String) -> Self {
        Self::Failed(StatusError::Unavailable(message))
    }
}

/// The shell's answer for a stopped status read.
fn settle_status(stop: StatusStop) -> Result<Response, crate::ShellError> {
    match stop {
        StatusStop::Failed(error) => Err(status_shell_error(error)),
        StatusStop::Refused(response) => Ok(response),
    }
}

fn status_failure(stop: StatusStop) -> Failure {
    match stop {
        StatusStop::Failed(error) => Failure::Message(status_error_message(error)),
        StatusStop::Refused(response) => Failure::Refused(response),
    }
}

/// The stop for an engine error raised while asking whether the model is
/// complete: the `QQ-1022` refusal, or a failure to report.
fn model_incomplete(error: &quoin_quire::Error) -> StatusStop {
    match Failure::from_quire(error, "formal_rules.status") {
        Failure::Refused(response) => StatusStop::Refused(response),
        Failure::Message(message) => StatusStop::from(message),
    }
}

#[implements("FR-188-CON-3")]
fn status(args: &ArgMatches, module_paths: &[String]) -> Result<Value, StatusStop> {
    let repo = PathBuf::from(
        args.get_one::<String>("repo")
            .ok_or_else(|| "--repo default is unavailable".to_owned())?,
    );
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let ea = absolute_file(args, "engineering-assurance")?;
    let allowed_source = absolute_file(args, "allowed-signers")?;
    let (_trust_dir, allowed, trust_digest) = capture_trust_bytes(&allowed_source)?;
    let verification = json!({
        "quoin_version": env!("QUOIN_VERSION"),
        "engineering_assurance_version": tool_version(&ea)?,
        "allowed_signers_sha256": trust_digest,
    });
    let gitsign_values = [
        args.get_one::<String>("gitsign-certificate-identity"),
        args.get_one::<String>("gitsign-certificate-oidc-issuer"),
        args.get_one::<String>("gitsign-principal"),
    ];
    let gitsign = match gitsign_values {
        [None, None, None] => None,
        [Some(identity), Some(issuer), Some(principal)] => Some(json!({
            "certificate_identity": identity,
            "certificate_oidc_issuer": issuer,
            "principal": principal,
        })),
        _ => {
            return Err(StatusError::Invalid(
                "all three gitsign identity options must be supplied together".to_owned(),
            )
            .into());
        }
    };

    let scope = ScopeRoot::open(&repo).map_err(|error| error.to_string())?;
    let selection = crate::analysis_evidence::module_selection(module_paths)
        .map_err(StatusError::Unavailable)?;
    let registry = selection
        .resolve(&scope, &mut Vec::new())
        .map_err(|error| error.to_string())?;
    // Before the absent-model answer: an all-dropped model reads as absent.
    quoin_quire::modules::require_complete_model(&registry).map_err(|e| model_incomplete(&e))?;
    let model = registry
        .traceability()
        .ok_or_else(|| "no active module declares a traceability model".to_owned())?;
    let current = revision_rules(&repo, "HEAD", model)?;
    let candidates = git_output(
        &repo,
        &[
            OsStr::new("log"),
            OsStr::new("--format=%H"),
            OsStr::new("-E"),
            OsStr::new("--grep=^Formal-rule: "),
            OsStr::new("HEAD"),
        ],
    )?;
    let commits = String::from_utf8(candidates)
        .map_err(|_| "git returned non-UTF-8 commit ids".to_owned())?;
    let mut verdicts = Vec::new();
    for commit in commits.lines().filter(|line| !line.is_empty()) {
        let result = verify_with_trailers(&ea, &allowed, &repo, commit, gitsign.as_ref())?;
        verdicts.push((commit.to_owned(), result));
    }
    let mut rules = Vec::with_capacity(current.len());
    for rule in current {
        let decision = decide(&rule, &verdicts, &DecisionContext { repo: &repo, model })?;
        let mut fields = serde_json::Map::from_iter([
            ("symbol".to_owned(), json!(rule.symbol)),
            ("path".to_owned(), json!(rule.path)),
            ("label".to_owned(), json!(rule.label)),
            (
                "declaration_range".to_owned(),
                json!({"start":rule.declaration_range.start,"end":rule.declaration_range.end}),
            ),
            ("formalizes".to_owned(), json!(rule.formalizes)),
            ("state".to_owned(), json!(decision.state)),
        ]);
        if let Some(reason) = decision.reason {
            fields.insert("reason".to_owned(), json!(reason));
        }
        if let Some((commit, principal)) = decision.approval {
            fields.insert(
                "approval".to_owned(),
                json!({"commit": commit, "principal": principal}),
            );
        }
        rules.push(Value::Object(fields));
    }
    rules.sort_by(|a, b| {
        (
            a.get("path").and_then(Value::as_str),
            a.get("label").and_then(Value::as_str),
        )
            .cmp(&(
                b.get("path").and_then(Value::as_str),
                b.get("label").and_then(Value::as_str),
            ))
    });
    Ok(
        json!({"protocol":"quoin.formal-rule-status/v1", "verification": verification, "rules":rules}),
    )
}

struct Decision {
    state: &'static str,
    reason: Option<&'static str>,
    approval: Option<(String, String)>,
}

struct DecisionContext<'a> {
    repo: &'a Path,
    model: &'a quoin_quire::formal_rules::TraceabilityModel,
}

#[allow(
    clippy::too_many_lines,
    reason = "keep the closed approval decision order auditable"
)]
fn decide(
    rule: &quoin_quire::formal_rules::FormalRule,
    verdicts: &[(String, Value)],
    context: &DecisionContext<'_>,
) -> Result<Decision, String> {
    let mut matching = Vec::new();
    for (commit, result) in verdicts {
        let values = result
            .pointer("/trailers/Formal-rule")
            .and_then(Value::as_array);
        if values.is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some(&rule.symbol))
        }) {
            matching.push((commit, result));
        }
    }
    if matching.is_empty() {
        return Ok(draft("no_approval"));
    }
    let mut most_recent_reason = None;
    for (commit, result) in matching {
        if result.get("verified_commit_id").and_then(Value::as_str) != Some(commit.as_str()) {
            most_recent_reason.get_or_insert("approval_unverifiable");
            continue;
        }
        match result["status"].as_str().unwrap_or("") {
            "unsigned" => {
                most_recent_reason.get_or_insert("approval_unsigned");
                continue;
            }
            "signed_bad" => {
                most_recent_reason.get_or_insert("approval_bad_signature");
                continue;
            }
            "signed_untrusted" => {
                most_recent_reason.get_or_insert("approval_untrusted");
                continue;
            }
            "signed_valid" => {}
            _ => {
                most_recent_reason.get_or_insert("approval_unverifiable");
                continue;
            }
        }
        let signer = result.get("signer").unwrap_or(&Value::Null);
        if signer.get("mapping").and_then(Value::as_str) != Some("mapped")
            || signer.get("principal").and_then(Value::as_str).is_none()
        {
            most_recent_reason.get_or_insert("approval_unverifiable");
            continue;
        }
        let meaning = result.get("signed_off_meaning").unwrap_or(&Value::Null);
        if meaning.get("presence").and_then(Value::as_str) != Some("present") {
            most_recent_reason.get_or_insert("meaning_missing");
            continue;
        }
        let values = meaning
            .get("values")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if values.as_slice() != [json!("formal-rule-approved")] {
            most_recent_reason.get_or_insert("meaning_mismatch");
            continue;
        }
        let old = revision_rules(context.repo, commit, context.model)?;
        if let Some(candidate) = old.iter().find(|candidate| candidate.symbol == rule.symbol) {
            if candidate.source == rule.source {
                return Ok(Decision {
                    state: "approved",
                    reason: None,
                    approval: Some((
                        commit.clone(),
                        signer
                            .get("principal")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    )),
                });
            }
            most_recent_reason.get_or_insert("changed_since_approval");
        } else {
            most_recent_reason.get_or_insert("changed_since_approval");
        }
    }
    Ok(draft(most_recent_reason.unwrap_or("approval_unverifiable")))
}

/// Read the trust bytes once, hash exactly those bytes, and write a private
/// copy of exactly those bytes for engineering-assurance, so the digest in the
/// report names the file every verification used and not a file that changed
/// in between (FR-188-AC-24). The directory guard must outlive the verifications.
fn capture_trust_bytes(source: &Path) -> Result<(tempfile::TempDir, PathBuf, String), StatusError> {
    let bytes = std::fs::read(source)
        .map_err(|error| format!("--allowed-signers is not readable: {error}"))?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let copy = directory.path().join("allowed-signers");
    std::fs::write(&copy, &bytes).map_err(|error| error.to_string())?;
    let digest = format!("sha256:{}", quoin_store::attachment::sha256_hex(&bytes));
    Ok((directory, copy, digest))
}

/// The version engineering-assurance reports for itself: the verifier whose
/// verdicts the report relies on (21 CFR 11.50, 11.70).
fn tool_version(ea: &Path) -> Result<String, StatusError> {
    let output = ProcessCommand::new(ea)
        .arg("--version")
        .output()
        .map_err(|error| format!("engineering-assurance is unavailable: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Exactly one line, `engineering-assurance <version>`: anything else is
    // not a verifier that identifies itself.
    let version = text
        .trim_end()
        .strip_prefix("engineering-assurance ")
        .filter(|version| {
            !version.is_empty()
                && version.len() <= 64
                && version
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        });
    match version {
        Some(version) if output.status.success() => Ok(version.to_owned()),
        _ => Err(StatusError::Unavailable(
            "engineering-assurance did not report a version".to_owned(),
        )),
    }
}

fn draft(reason: &'static str) -> Decision {
    Decision {
        state: "draft",
        reason: Some(reason),
        approval: None,
    }
}

fn verify(
    ea: &Path,
    allowed: &Path,
    repo: &Path,
    revision: &str,
    gitsign: Option<&Value>,
) -> Result<Value, StatusError> {
    verify_requesting(ea, allowed, repo, revision, gitsign, false)
}

/// `verify`, also asking engineering-assurance for the `Formal-rule` trailers
/// the status decision reads. Only the status path asks: a verifier that
/// predates `trailer_keys` refuses the member outright.
fn verify_with_trailers(
    ea: &Path,
    allowed: &Path,
    repo: &Path,
    revision: &str,
    gitsign: Option<&Value>,
) -> Result<Value, StatusError> {
    verify_requesting(ea, allowed, repo, revision, gitsign, true)
}

fn verify_requesting(
    ea: &Path,
    allowed: &Path,
    repo: &Path,
    revision: &str,
    gitsign: Option<&Value>,
    trailers: bool,
) -> Result<Value, StatusError> {
    let request = signature_request(repo, revision, allowed, gitsign, trailers);
    let output = quoin_core::runtime::engineering_assurance_call(ea, "signature-verify", &request)
        .map_err(StatusError::Unavailable)?;
    if !output.status.success() {
        let refusal: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        if refusal.get("protocol").and_then(Value::as_str) == Some("engineering-assurance.error/v1")
        {
            let code = refusal.get("code").and_then(Value::as_str).unwrap_or("");
            let message = refusal.get("message").and_then(Value::as_str).unwrap_or("");
            if code == "signature_verify_allowed_signers_invalid" {
                return Err(StatusError::Invalid(format!(
                    "{code}: {message}; {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            return Err(StatusError::Unavailable(format!(
                "{code}: {message}; {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        return Err(StatusError::Unavailable(format!(
            "engineering-assurance signature-verify failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        StatusError::Unavailable(format!("invalid engineering-assurance result: {error}"))
    })?;
    if value.get("protocol").and_then(Value::as_str) != Some(RESULT_PROTOCOL)
        || value.get("kind").and_then(Value::as_str) != Some("commit")
    {
        return Err(StatusError::Unavailable(
            "engineering-assurance returned an unsupported signature result".to_owned(),
        ));
    }
    Ok(value)
}

/// Verify one commit signature for the retained-evidence host boundary.
pub(crate) fn verify_for_evidence(
    ea: &Path,
    allowed: &Path,
    repo: &Path,
    revision: &str,
    gitsign: Option<&Value>,
) -> Result<Value, String> {
    verify(ea, allowed, repo, revision, gitsign).map_err(status_error_message)
}

/// Verify one risk-decision commit through engineering-assurance without
/// adding a second signature implementation to Quoin.
pub(crate) fn verify_for_risk(
    ea: &Path,
    allowed: &Path,
    repo: &Path,
    revision: &str,
) -> Result<Value, StatusError> {
    verify(ea, allowed, repo, revision, None)
}

fn signature_request(
    repo: &Path,
    revision: &str,
    allowed: &Path,
    gitsign: Option<&Value>,
    trailers: bool,
) -> Value {
    let mut request = serde_json::Map::from_iter([
        ("protocol".to_owned(), json!(REQUEST_PROTOCOL)),
        ("repository".to_owned(), json!(repo)),
        ("revision".to_owned(), json!(revision)),
        ("kind".to_owned(), json!("commit")),
        ("allowed_signers".to_owned(), json!(allowed)),
    ]);
    if trailers {
        // The trailers come from engineering-assurance and only from it
        // (FR-188). Without this member the result carries none, and every
        // rule reads as `no_approval` (PLAT-1789).
        request.insert("trailer_keys".to_owned(), json!(["Formal-rule"]));
    }
    if let Some(gitsign) = gitsign {
        request.insert("gitsign".to_owned(), gitsign.clone());
    }
    Value::Object(request)
}

fn revision_rules(
    repo: &Path,
    commit: &str,
    model: &quoin_quire::formal_rules::TraceabilityModel,
) -> Result<Vec<quoin_quire::formal_rules::FormalRule>, String> {
    let names = git_output(
        repo,
        &[
            OsStr::new("ls-tree"),
            OsStr::new("-r"),
            OsStr::new("--name-only"),
            OsStr::new("-z"),
            OsStr::new(commit),
        ],
    )?;
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    for raw in names
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
    {
        let path = std::str::from_utf8(raw)
            .map_err(|_| "git tree contains a non-UTF-8 path".to_owned())?;
        if !path.ends_with(".native") {
            continue;
        }
        let relative = Path::new(path);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("git tree contains an unsafe QSL path".to_owned());
        }
        let spec = format!("{commit}:{path}");
        let bytes = git_output(
            repo,
            &[
                OsStr::new("show"),
                OsStr::new("--end-of-options"),
                OsStr::new(&spec),
            ],
        )?;
        let target = temp.path().join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(target, bytes).map_err(|error| error.to_string())?;
    }
    quoin_quire::formal_rules::extract(temp.path(), model).map_err(|error| error.to_string())
}

fn absolute_file(args: &ArgMatches, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(
        args.get_one::<String>(name)
            .ok_or_else(|| format!("--{name} is required"))?,
    );
    if !path.is_absolute() {
        return Err(format!("--{name} must be an absolute path"));
    }
    if !path.is_file() {
        return Err(format!("--{name} is not a readable file"));
    }
    Ok(path)
}

fn valid_gitsign_value(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn git_output(repo: &Path, args: &[&OsStr]) -> Result<Vec<u8>, String> {
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("git is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixture assertions are test failures"
)]
mod tests {
    use super::{
        DecisionContext, RESULT_PROTOCOL, command, decide, revision_rules, signature_request,
        status_shell_error, verify,
    };
    use ix_trace_rs::trace;
    use quoin_quire::formal_rules::FormalRule;
    use serde_json::{Value, json};
    use std::{fs, path::Path, process::Command};

    #[cfg(unix)]
    fn executable(directory: &Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = directory.join(name);
        // A stand-in verifier still answers `--version`: the report names it.
        let version =
            "if [ \"$1\" = --version ]; then echo 'engineering-assurance 0.0.0-test'; exit 0; fi";
        fs::write(&path, format!("#!/bin/sh\n{version}\n{body}\n")).expect("write executable");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("executable permissions");
        path
    }

    fn model() -> quoin_quire::formal_rules::TraceabilityModel {
        serde_json::from_value(serde_json::json!({
            "trace_tags": {
                "formalizes": [{
                    "name": "formalizes",
                    "language": "qsl",
                    "syntax": "comment",
                    "pattern": "^// formalizes: (.+)$"
                }]
            }
        }))
        .expect("traceability model")
    }

    fn git(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
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

    fn commit_repo(native: &str) -> (tempfile::TempDir, String) {
        let repo = tempfile::tempdir().expect("temporary git repository");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.name", "VER-101 test"]);
        git(
            repo.path(),
            &["config", "user.email", "ver101@example.invalid"],
        );
        fs::write(repo.path().join("a.native"), native).expect("QSL file");
        git(repo.path(), &["add", "a.native"]);
        git(repo.path(), &["commit", "-q", "-m", "formal rule"]);
        let commit = git(repo.path(), &["rev-parse", "HEAD"]);
        (repo, commit)
    }

    #[cfg(unix)]
    fn command_fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let repo = tempfile::tempdir().expect("temporary status repository");
        fs::write(
            repo.path().join("manifest.yaml"),
            "name: formal-rule-status-fixture\nversion: 0.1.0\ndescription: Status command fixture\nartifact_types:\n  - name: FR\ntraceability:\n  trace_tags:\n    formalizes:\n      - name: formalizes\n        language: qsl\n        syntax: comment\n        pattern: '^// formalizes: (.+)$'\n",
        )
        .expect("module manifest");
        fs::write(
            repo.path().join("a.native"),
            "language qsl\nprofile core\n// formalizes: FR-001-AC-1\nrule P1 { check x }\n",
        )
        .expect("formalized QSL rule");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.name", "VER-101 test"]);
        git(
            repo.path(),
            &["config", "user.email", "ver101@example.invalid"],
        );
        git(repo.path(), &["add", "manifest.yaml", "a.native"]);
        git(repo.path(), &["commit", "-q", "-m", "draft formal rule"]);
        let allowed = repo.path().join("allowed-signers");
        fs::write(&allowed, "fixture\n").expect("allowed signers");
        let ea = executable(repo.path(), "engineering-assurance", "exit 99");
        (repo, ea, allowed)
    }

    #[cfg(unix)]
    fn status_matches(repo: &Path, ea: &Path, allowed: &Path) -> clap::ArgMatches {
        status_matches_with_select_out(repo, ea, allowed, None)
    }

    #[cfg(unix)]
    fn status_matches_with_select_out(
        repo: &Path,
        ea: &Path,
        allowed: &Path,
        select_out: Option<&Path>,
    ) -> clap::ArgMatches {
        let mut arguments = vec![
            "formal-rules".to_owned(),
            "status".to_owned(),
            "--repo".to_owned(),
            repo.to_str().expect("repo path is UTF-8").to_owned(),
            "--engineering-assurance".to_owned(),
            ea.to_str().expect("EA path is UTF-8").to_owned(),
            "--allowed-signers".to_owned(),
            allowed
                .to_str()
                .expect("allowed signers path is UTF-8")
                .to_owned(),
            "--json".to_owned(),
        ];
        if let Some(path) = select_out {
            arguments.push("--select-out".to_owned());
            arguments.push(path.to_str().expect("selection path is UTF-8").to_owned());
        }
        command()
            .try_get_matches_from(arguments)
            .expect("status arguments")
    }

    #[cfg(unix)]
    fn approve_symbols(repo: &Path, ea: &Path, symbols: &[String]) -> String {
        let trailers = symbols
            .iter()
            .map(|symbol| format!("Formal-rule: {symbol}"))
            .collect::<Vec<_>>()
            .join("\n");
        git(
            repo,
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "approval",
                "-m",
                &trailers,
            ],
        );
        let commit = git(repo, &["rev-parse", "HEAD"]);
        let result = json!({
            "protocol": RESULT_PROTOCOL,
            "kind": "commit",
            "status": "signed_valid",
            "verified_commit_id": commit,
            "signer": {"mapping":"mapped", "principal":"alice@example.invalid"},
            "signed_off_meaning": {"presence":"present", "values":["formal-rule-approved"]},
            "trailers": {"Formal-rule": symbols},
        });
        executable(
            ea.parent().expect("EA fixture parent"),
            "engineering-assurance",
            &format!("cat >/dev/null\nprintf '%s\\n' '{}'; exit 0", result),
        );
        commit
    }

    #[cfg(unix)]
    fn formal_rule_symbol(path: &str, label: &str) -> String {
        format!("qsl:{path}::{label}")
    }

    #[cfg(unix)]
    fn committed_sources(repo: &Path, sources: &[(&str, &str)]) {
        let existing = repo.join("a.native");
        if existing.exists() {
            fs::remove_file(existing).expect("remove default QSL fixture");
        }
        for (path, source) in sources {
            let target = repo.join(path);
            fs::create_dir_all(target.parent().expect("QSL parent")).expect("QSL directory");
            fs::write(target, source).expect("QSL source");
        }
        git(repo, &["add", "--all"]);
        git(
            repo,
            &["commit", "-q", "-m", "set formal-rule fixture sources"],
        );
    }

    #[cfg(unix)]
    fn read_selection(path: &Path) -> Value {
        let bytes = fs::read(path).expect("selection file bytes");
        let selection: Value = serde_json::from_slice(&bytes).expect("selection JSON");
        assert_eq!(selection["protocol"], "filament-analyze.claim-selection/v1");
        selection
    }

    #[cfg(unix)]
    fn run_with_selection(
        repo: &Path,
        ea: &Path,
        allowed: &Path,
        select_out: &Path,
    ) -> (Value, Value) {
        let response = super::run(&status_matches_with_select_out(
            repo,
            ea,
            allowed,
            Some(select_out),
        ))
        .expect("status command writes selection");
        let rendered = response.payload["rendered"]
            .as_str()
            .expect("rendered status report");
        let report: Value = serde_json::from_str(rendered).expect("status JSON");
        (report, read_selection(select_out))
    }

    fn signature(commit: &str, symbol: &str) -> serde_json::Value {
        json!({
            "protocol": RESULT_PROTOCOL,
            "kind": "commit",
            "status": "signed_valid",
            "verified_commit_id": commit,
            "signer": {"mapping":"mapped", "principal":"alice@example.invalid"},
            "signed_off_meaning": {"presence":"present", "values":["formal-rule-approved"]},
            "trailers": {"Formal-rule":[symbol]},
        })
    }

    fn rule() -> FormalRule {
        FormalRule {
            symbol: "qsl:a.native::P1".to_owned(),
            path: "a.native".to_owned(),
            label: "P1".to_owned(),
            formalizes: vec!["FR-001".to_owned()],
            declaration_range: 0..8,
            source: b"// formalizes: FR-001\nrule P1 {}\n".to_vec(),
        }
    }

    #[trace("FR-188-AC-2")]
    #[test]
    fn absent_candidates_report_no_approval() {
        let repository = tempfile::tempdir().expect("temporary repository");
        let model = quoin_quire::formal_rules::TraceabilityModel::default();
        let result = decide(
            &rule(),
            &[],
            &DecisionContext {
                repo: repository.path(),
                model: &model,
            },
        )
        .expect("status decision");
        assert_eq!(result.state, "draft");
        assert_eq!(result.reason, Some("no_approval"));
    }

    #[test]
    fn draft_reason_comes_from_the_most_recent_naming_commit() {
        let repository = tempfile::tempdir().expect("temporary repository");
        let model = quoin_quire::formal_rules::TraceabilityModel::default();
        let with_status = |status: &str| {
            json!({
                "status": status,
                "kind": "commit",
                "verified_commit_id": "commit-id",
                "trailers": {"Formal-rule": ["qsl:a.native::P1"]}
            })
        };
        let candidates = vec![
            ("commit-id".to_owned(), with_status("signed_bad")),
            ("commit-id".to_owned(), with_status("unsigned")),
        ];
        let result = decide(
            &rule(),
            &candidates,
            &DecisionContext {
                repo: repository.path(),
                model: &model,
            },
        )
        .expect("status decision");
        assert_eq!(result.reason, Some("approval_bad_signature"));
    }

    #[trace(
        "FR-188-AC-3",
        "FR-188-AC-4",
        "FR-188-AC-5",
        "FR-188-AC-6",
        "FR-188-AC-7",
        "FR-188-AC-8",
        "FR-188-AC-9",
        "FR-188-AC-15"
    )]
    #[test]
    fn each_signature_and_meaning_failure_maps_to_its_closed_draft_reason() {
        let repository = tempfile::tempdir().expect("temporary repository");
        let model = model();
        let cases = [
            ("unsigned", json!({}), "approval_unsigned"),
            ("signed_bad", json!({}), "approval_bad_signature"),
            ("signed_untrusted", json!({}), "approval_untrusted"),
            ("unverifiable", json!({}), "approval_unverifiable"),
            (
                "signed_valid",
                json!({"signer":{"mapping":"unmapped"}}),
                "approval_unverifiable",
            ),
            (
                "signed_valid",
                json!({
                    "signer":{"mapping":"unmapped"},
                    "signed_off_meaning":{"presence":"missing","values":[]}
                }),
                "approval_unverifiable",
            ),
            (
                "signed_valid",
                json!({"signer":{"mapping":"mapped","principal":"alice"},"signed_off_meaning":{"presence":"missing","values":[]}}),
                "meaning_missing",
            ),
            (
                "signed_valid",
                json!({"signer":{"mapping":"mapped","principal":"alice"},"signed_off_meaning":{"presence":"present","values":["release-approved"]}}),
                "meaning_mismatch",
            ),
            (
                "signed_valid",
                json!({"signer":{"mapping":"mapped","principal":"alice"},"signed_off_meaning":{"presence":"present","values":["formal-rule-approved", "release-approved"]}}),
                "meaning_mismatch",
            ),
        ];
        for (status, extra, expected) in cases {
            let mut result = json!({
                "protocol": RESULT_PROTOCOL,
                "kind":"commit",
                "status":status,
                "verified_commit_id":"commit-id",
                "trailers":{"Formal-rule":["qsl:a.native::P1"]},
            });
            for (key, value) in extra.as_object().expect("object") {
                result[key] = value.clone();
            }
            let result = decide(
                &rule(),
                &[("commit-id".to_owned(), result)],
                &DecisionContext {
                    repo: repository.path(),
                    model: &model,
                },
            )
            .expect("status decision");
            assert_eq!(result.reason, Some(expected), "{status} => {expected}");
        }
    }

    #[trace("FR-188-AC-1", "FR-188-AC-13")]
    #[test]
    fn approved_state_uses_exact_symbol_and_marker_declaration_bytes() {
        let model = model();
        let native = "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 {\n  check x\n}\n";
        let (repo, commit) = commit_repo(native);
        let rule = quoin_quire::formal_rules::extract(repo.path(), &model)
            .expect("extract rule")
            .remove(0);
        let approval = signature(&commit, &rule.symbol);
        let approved = decide(
            &rule,
            &[(commit.clone(), approval.clone())],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("approved status");
        assert_eq!(approved.state, "approved");
        assert_eq!(
            approved.approval,
            Some((commit.clone(), "alice@example.invalid".to_owned()))
        );

        fs::write(
            repo.path().join("a.native"),
            format!("// unrelated\n{native}"),
        )
        .expect("edit unrelated line");
        let current = revision_rules(repo.path(), "HEAD", &model)
            .expect("extract after unrelated edit")
            .remove(0);
        let still_approved = decide(
            &current,
            &[(commit.clone(), approval)],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("unrelated line leaves approval valid");
        assert_eq!(still_approved.state, "approved");
    }

    #[trace("FR-188-AC-10", "FR-188-AC-11", "FR-188-AC-12")]
    #[test]
    fn changed_body_renamed_label_and_other_symbol_do_not_inherit_approval() {
        let model = model();
        let native = "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 {\n  check x\n}\n";
        let (repo, commit) = commit_repo(native);
        let original = quoin_quire::formal_rules::extract(repo.path(), &model)
            .expect("extract original")
            .remove(0);
        let approval = signature(&commit, &original.symbol);

        fs::write(
            repo.path().join("a.native"),
            native.replace("check x", "check y"),
        )
        .expect("edit declaration body");
        let changed = quoin_quire::formal_rules::extract(repo.path(), &model)
            .expect("extract changed")
            .remove(0);
        let changed_status = decide(
            &changed,
            &[(commit.clone(), approval.clone())],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("changed status");
        assert_eq!(changed_status.reason, Some("changed_since_approval"));

        fs::write(
            repo.path().join("a.native"),
            native.replace("rule P1", "rule Pre1"),
        )
        .expect("rename declaration");
        let renamed = quoin_quire::formal_rules::extract(repo.path(), &model)
            .expect("extract renamed")
            .remove(0);
        let renamed_status = decide(
            &renamed,
            &[(commit.clone(), approval.clone())],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("renamed status");
        assert_eq!(renamed_status.reason, Some("no_approval"));

        let both = "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 {}\n// formalizes: FR-001\nrule P2 {}\n";
        fs::write(repo.path().join("a.native"), both).expect("two rules");
        let rules =
            quoin_quire::formal_rules::extract(repo.path(), &model).expect("extract two rules");
        let p2 = rules
            .iter()
            .find(|candidate| candidate.label == "P2")
            .expect("P2 rule");
        let p2_status = decide(
            p2,
            &[(commit.clone(), approval)],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("P1 approval does not transfer to P2");
        assert_eq!(p2_status.reason, Some("no_approval"));
    }

    #[trace("FR-188-AC-14")]
    #[test]
    fn most_recent_valid_naming_commit_is_the_approval_of_record() {
        let model = model();
        let native = "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 { check x }\n";
        let (repo, first) = commit_repo(native);
        let original = revision_rules(repo.path(), "HEAD", &model)
            .expect("extract initial HEAD")
            .remove(0);
        let first_approval = signature(&first, &original.symbol);
        fs::write(repo.path().join("unrelated.txt"), "new commit\n").expect("unrelated file");
        git(repo.path(), &["add", "unrelated.txt"]);
        git(
            repo.path(),
            &[
                "commit",
                "-q",
                "-m",
                "second approval",
                "-m",
                "Formal-rule: qsl:a.native::P1",
            ],
        );
        let second = git(repo.path(), &["rev-parse", "HEAD"]);
        let second_approval = signature(&second, &original.symbol);
        let current = revision_rules(repo.path(), "HEAD", &model)
            .expect("extract latest HEAD")
            .remove(0);

        let result = decide(
            &current,
            &[
                (second.clone(), second_approval),
                (first.clone(), first_approval),
            ],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("status decision");
        assert_eq!(
            result.approval,
            Some((second, "alice@example.invalid".to_owned()))
        );
    }

    #[trace("FR-188-AC-21", "FR-188-AC-22")]
    #[test]
    fn dirty_worktree_does_not_change_fixed_head_rule_bytes_or_repository_state() {
        let model = model();
        let native = "language qsl\nprofile core\n// formalizes: FR-001\n// é before declaration\nrule P1 { check x }\n";
        let (repo, commit) = commit_repo(native);
        let before_head = git(repo.path(), &["rev-parse", "HEAD"]);
        let before_status = git(repo.path(), &["status", "--porcelain"]);
        let original = revision_rules(repo.path(), "HEAD", &model)
            .expect("extract committed rule")
            .remove(0);
        assert!(original.declaration_range.start < original.declaration_range.end);
        assert_eq!(
            &original.source[original.source.len() - b"rule P1 { check x }\n".len()..],
            b"rule P1 { check x }\n"
        );

        fs::write(
            repo.path().join("a.native"),
            native.replace("check x", "check dirty"),
        )
        .expect("dirty the declaration in the worktree");
        let current = revision_rules(repo.path(), "HEAD", &model)
            .expect("extract fixed HEAD after worktree edit")
            .remove(0);
        assert_eq!(current, original);
        let result = decide(
            &current,
            &[(commit.clone(), signature(&commit, &current.symbol))],
            &DecisionContext {
                repo: repo.path(),
                model: &model,
            },
        )
        .expect("fixed HEAD remains approved");
        assert_eq!(result.state, "approved");
        assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), before_head);
        assert_ne!(git(repo.path(), &["status", "--porcelain"]), before_status);
    }

    /// FR-029-AC-22 and AC-20: the formal-rules declaration read behind
    /// `quoin assurance` refuses with `QQ-1022` before the absent-model check,
    /// including when every declaration was dropped and the model is empty.
    #[trace("FR-029-AC-22", "FR-029-AC-20")]
    #[test]
    fn the_assurance_declaration_read_refuses_a_dropped_declaration() {
        let manifests = [
            (
                "one",
                "name: drop-one\nversion: 0.1.0\ndescription: x\nartifact_types:\n  - name: FR\n\
                 traceability:\n  flow_rules:\n    - name: bad\n      subject: [nowhere]\n      \
                 match: [{path: [], field: id, present: true}]\n      check: c\n      rationale: r\n",
                "module=drop-one kind=flow_rule item=bad",
            ),
            (
                "all",
                "name: drop-all\nversion: 0.1.0\ndescription: x\nartifact_types:\n  - name: FR\n\
                 traceability:\n  required_relations: 5\n",
                "module=drop-all kind=traceability_section item=traceability",
            ),
        ];
        for (label, manifest, line) in manifests {
            let repo = tempfile::tempdir().expect("scope");
            fs::create_dir_all(repo.path().join("spec")).expect("spec");
            fs::write(repo.path().join("manifest.yaml"), manifest).expect("manifest");
            let refused = super::assurance_declarations(repo.path(), &[]).expect_err(label);
            let super::Failure::Refused(response) = refused else {
                panic!("{label}: expected the QQ-1022 refusal");
            };
            assert_eq!(response.outcome.code(), 2, "{label}");
            assert!(response.payload.is_null(), "{label}");
            let diagnostic = &response.diagnostics[0];
            assert_eq!(diagnostic.context["quire_code"], "QQ-1022", "{label}");
            assert_eq!(diagnostic.context["dropped_declarations"], line, "{label}");
        }
    }

    #[cfg(unix)]
    #[trace("FR-188-AC-18", "FR-188-AC-20")]
    #[test]
    fn verifier_refusals_keep_usage_and_unavailable_exit_classes_without_report() {
        let directory = tempfile::tempdir().expect("temporary verifier directory");
        let allowed = directory.path().join("allowed-signers");
        fs::write(&allowed, "fixture\n").expect("allowed signers");
        let repo = directory.path();
        let refusal = executable(
            directory.path(),
            "ea-invalid-allowed",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_allowed_signers_invalid\",\"message\":\"bad allowed signers\"}'; printf '%s\\n' diagnostic >&2; exit 2",
        );
        let error = verify(&refusal, &allowed, repo, "HEAD", None)
            .expect_err("allowed-signers refusal is not a report");
        assert!(
            matches!(&error, super::StatusError::Invalid(_)),
            "allowed-signers refusal must retain its dedicated error class"
        );
        let shell = status_shell_error(error);
        assert_eq!(shell.exit, 2);
        assert!(
            shell
                .message
                .contains("signature_verify_allowed_signers_invalid")
        );

        let refusal = executable(
            directory.path(),
            "ea-other-refusal",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_request_invalid\",\"message\":\"bad request\"}'; printf '%s\\n' diagnostic >&2; exit 2",
        );
        let error = verify(&refusal, &allowed, repo, "HEAD", None)
            .expect_err("other refusal is not a report");
        let shell = status_shell_error(error);
        assert_eq!(shell.exit, 3);
        assert!(shell.message.contains("signature_verify_request_invalid"));

        let missing = directory.path().join("missing-ea");
        let shell = status_shell_error(
            verify(&missing, &allowed, repo, "HEAD", None)
                .expect_err("missing executable is unavailable"),
        );
        assert_eq!(shell.exit, 3);
    }

    #[trace("FR-188-AC-18", "FR-188-AC-23")]
    #[test]
    fn signature_request_omits_or_forwards_exact_gitsign_values() {
        let repo = Path::new("/repo");
        let allowed = Path::new("/allowed-signers");
        let without = signature_request(repo, "HEAD", allowed, None, false);
        assert!(without.get("gitsign").is_none());
        let gitsign = json!({
            "certificate_identity":"identity",
            "certificate_oidc_issuer":"issuer",
            "principal":"alice"
        });
        let with = signature_request(repo, "HEAD", allowed, Some(&gitsign), false);
        assert_eq!(with["gitsign"], gitsign);

        let partial = command()
            .try_get_matches_from([
                "formal-rules",
                "status",
                "--engineering-assurance",
                "/ea",
                "--allowed-signers",
                "/allowed",
                "--gitsign-certificate-identity",
                "identity",
            ])
            .expect("parser accepts individual optional flags for cross-validation");
        let error = super::run(&partial).expect_err("partial gitsign set is rejected");
        assert_eq!(error.exit, 2);

        let missing = command()
            .try_get_matches_from(["formal-rules", "status"])
            .expect("command parses before its own FR-188 validation");
        let error = super::run(&missing).expect_err("required trust flags are rejected");
        assert_eq!(error.exit, 2);
    }

    #[cfg(unix)]
    #[trace("FR-188-AC-17", "FR-188-AC-19")]
    #[test]
    fn status_command_is_read_only_and_gate_ack_without_signature_is_not_approval() {
        let (repo, ea, allowed) = command_fixture();
        git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "Workflow gate acknowledged",
                "-m",
                "Gate-Acknowledged: formalize-requirement",
            ],
        );
        let head_before = git(repo.path(), &["log", "--format=%H"]);
        let status_before = git(repo.path(), &["status", "--porcelain"]);

        let response =
            super::run(&status_matches(repo.path(), &ea, &allowed)).expect("status report");
        let rendered = response.payload["rendered"].as_str().expect("JSON report");
        let report: Value = serde_json::from_str(rendered).expect("status JSON");
        assert_eq!(report["rules"][0]["state"], "draft");
        assert_eq!(report["rules"][0]["reason"], "no_approval");
        assert_eq!(git(repo.path(), &["log", "--format=%H"]), head_before);
        assert_eq!(git(repo.path(), &["status", "--porcelain"]), status_before);
    }

    #[cfg(unix)]
    #[trace("FR-188-CON-3", "NFR-039-M-1")]
    #[test]
    fn status_verifies_only_commits_with_a_formal_rule_trailer() {
        let (repo, _ea, allowed) = command_fixture();
        committed_sources(
            repo.path(),
            &[(
                "a.native",
                "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 {}\n// formalizes: FR-002\nrule P2 {}\n",
            )],
        );
        git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "not a candidate",
                "-m",
                "Notes: Formal-rule: qsl:a.native::P1",
            ],
        );
        let non_candidate = git(repo.path(), &["rev-parse", "HEAD"]);
        git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "malformed trailer prefix",
                "-m",
                "Formal-ruleish: qsl:a.native::P1",
            ],
        );
        let second_non_candidate = git(repo.path(), &["rev-parse", "HEAD"]);
        git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "formal rule approval",
                "-m",
                "Formal-rule: qsl:a.native::P1",
                "-m",
                "Formal-rule: qsl:a.native::P2",
            ],
        );
        let candidate = git(repo.path(), &["rev-parse", "HEAD"]);

        let requests = repo.path().join("signature-verify-requests.jsonl");
        let verifier = executable(
            repo.path(),
            "recording-ea",
            &format!(
                "cat >> '{}'\nprintf '%s\\n' '{}'\n",
                requests.display(),
                json!({
                    "protocol": RESULT_PROTOCOL,
                    "kind": "commit",
                    "status": "unsigned",
                    "verified_commit_id": candidate.clone(),
                    "trailers": {"Formal-rule": ["qsl:a.native::P1", "qsl:a.native::P2"]}
                })
            ),
        );

        let response = super::run(&status_matches(repo.path(), &verifier, &allowed))
            .expect("status report succeeds with an unsigned candidate");
        let report: Value = serde_json::from_str(
            response.payload["rendered"]
                .as_str()
                .expect("rendered status report"),
        )
        .expect("status report JSON");
        let rules = report["rules"].as_array().expect("formal rules");
        assert_eq!(rules.len(), 2);
        assert!(
            rules
                .iter()
                .all(|rule| rule["reason"] == "approval_unsigned")
        );

        let requests = fs::read_to_string(requests).expect("recorded verifier requests");
        let revisions = requests
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line)
                    .expect("request JSON")
                    .get("revision")
                    .and_then(Value::as_str)
                    .expect("requested revision")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            revisions,
            vec![candidate],
            "one verifier call covers both rules on a candidate commit; excluded {non_candidate} and {second_non_candidate}"
        );
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-2")]
    #[test]
    fn select_out_writes_an_empty_selection_when_no_rule_is_approved() {
        let (repo, ea, allowed) = command_fixture();
        let output = repo.path().join("selection.json");

        let (report, selection) = run_with_selection(repo.path(), &ea, &allowed, &output);

        assert_eq!(report["rules"][0]["state"], "draft");
        assert_eq!(selection["spans"], json!([]));
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-3")]
    #[test]
    fn select_out_excludes_a_declaration_changed_after_approval() {
        let (repo, ea, allowed) = command_fixture();
        let symbol = formal_rule_symbol("a.native", "P1");
        let approval = approve_symbols(repo.path(), &ea, &[symbol]);
        fs::write(
            repo.path().join("a.native"),
            "language qsl\nprofile core\n// formalizes: FR-001-AC-1\nrule P1 { check y }\n",
        )
        .expect("edit the approved declaration");
        git(repo.path(), &["add", "a.native"]);
        git(
            repo.path(),
            &["commit", "-q", "-m", "edit approved declaration"],
        );
        assert_ne!(git(repo.path(), &["rev-parse", "HEAD"]), approval);
        let output = repo.path().join("selection.json");

        let (report, selection) = run_with_selection(repo.path(), &ea, &allowed, &output);

        assert_eq!(report["rules"][0]["state"], "draft");
        assert_eq!(report["rules"][0]["reason"], "changed_since_approval");
        assert_eq!(selection["spans"], json!([]));
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-4")]
    #[test]
    fn select_out_offsets_are_utf8_bytes_after_multibyte_source() {
        let (repo, ea, allowed) = command_fixture();
        let source = "language qsl\nprofile core\n// café before declaration\n// formalizes: FR-001-AC-4\nrule P1 { check x }\n";
        committed_sources(repo.path(), &[("rules/é.native", source)]);
        let symbol = formal_rule_symbol("rules/é.native", "P1");
        approve_symbols(repo.path(), &ea, &[symbol]);
        let output = repo.path().join("selection.json");

        let (report, selection) = run_with_selection(repo.path(), &ea, &allowed, &output);

        let span = &selection["spans"][0];
        let start = usize::try_from(span["start"].as_u64().expect("start offset"))
            .expect("start fits usize");
        let end =
            usize::try_from(span["end"].as_u64().expect("end offset")).expect("end fits usize");
        let rule = report["rules"]
            .as_array()
            .expect("status rules")
            .iter()
            .find(|rule| rule["state"] == "approved")
            .expect("approved status row");
        assert_eq!(span["identity"], "rules/é.native");
        assert_eq!(span["start"], rule["declaration_range"]["start"]);
        assert_eq!(span["end"], rule["declaration_range"]["end"]);
        assert_eq!(&source.as_bytes()[start..end], b"rule P1 { check x }");
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-5")]
    #[test]
    fn select_out_orders_identities_by_utf8_bytes() {
        let (repo, ea, allowed) = command_fixture();
        let source =
            "language qsl\nprofile core\n// formalizes: FR-001-AC-5\nrule P1 { check x }\n";
        committed_sources(
            repo.path(),
            &[
                ("rules/é.native", source),
                ("rules/z.native", source),
                ("rules/b.native", source),
            ],
        );
        let symbols = [
            formal_rule_symbol("rules/é.native", "P1"),
            formal_rule_symbol("rules/z.native", "P1"),
            formal_rule_symbol("rules/b.native", "P1"),
        ];
        approve_symbols(repo.path(), &ea, &symbols);
        let output = repo.path().join("selection.json");

        let (_, selection) = run_with_selection(repo.path(), &ea, &allowed, &output);

        let identities = selection["spans"]
            .as_array()
            .expect("selection spans")
            .iter()
            .map(|span| span["identity"].as_str().expect("span identity").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            identities,
            vec!["rules/b.native", "rules/z.native", "rules/é.native"]
        );
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-6")]
    #[test]
    fn select_out_orders_declarations_in_one_file_by_start_offset() {
        let (repo, ea, allowed) = command_fixture();
        let source = "language qsl\nprofile core\n// formalizes: FR-001-AC-6\nrule Zed { check x }\n// formalizes: FR-002-AC-6\nrule Alpha { check y }\n";
        committed_sources(repo.path(), &[("rules/same.native", source)]);
        let symbols = [
            formal_rule_symbol("rules/same.native", "Zed"),
            formal_rule_symbol("rules/same.native", "Alpha"),
        ];
        approve_symbols(repo.path(), &ea, &symbols);
        let output = repo.path().join("selection.json");

        let (_, selection) = run_with_selection(repo.path(), &ea, &allowed, &output);

        let spans = selection["spans"].as_array().expect("selection spans");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0]["identity"], "rules/same.native");
        assert_eq!(spans[1]["identity"], "rules/same.native");
        assert!(
            spans[0]["start"].as_u64().expect("first offset")
                < spans[1]["start"].as_u64().expect("second offset")
        );
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-7")]
    #[test]
    fn relative_select_out_is_refused_before_repository_access() {
        let (repo, ea, allowed) = command_fixture();
        let relative = format!("relative-selection-{}.json", std::process::id());
        let missing_repo = repo.path().join("does-not-exist");
        let matches = status_matches_with_select_out(
            &missing_repo,
            &ea,
            &allowed,
            Some(Path::new(&relative)),
        );

        let error = super::run(&matches).expect_err("relative selection path is refused");

        assert_eq!(error.exit, 2);
        assert!(error.message.contains("--select-out must be"));
        assert!(!Path::new(&relative).exists());
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-8")]
    #[test]
    fn select_out_removes_existing_file_when_status_exits_two() {
        let (repo, ea, allowed) = command_fixture();
        let symbol = formal_rule_symbol("a.native", "P1");
        approve_symbols(repo.path(), &ea, &[symbol]);
        let refusal = executable(
            repo.path(),
            "engineering-assurance",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_allowed_signers_invalid\",\"message\":\"bad signers\"}'; exit 2",
        );
        let output = repo.path().join("selection.json");
        fs::write(&output, b"stale selection").expect("stale selection file");

        let error = super::run(&status_matches_with_select_out(
            repo.path(),
            &refusal,
            &allowed,
            Some(&output),
        ))
        .expect_err("FR-188 invalid refusal must not leave a stale selection");

        assert_eq!(error.exit, 2, "{}", error.message);
        assert!(!output.exists());
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-9")]
    #[test]
    fn select_out_removes_existing_file_when_status_exits_three() {
        let (repo, ea, allowed) = command_fixture();
        let symbol = formal_rule_symbol("a.native", "P1");
        approve_symbols(repo.path(), &ea, &[symbol]);
        let refusal = executable(
            repo.path(),
            "engineering-assurance",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_request_invalid\",\"message\":\"bad request\"}'; exit 2",
        );
        let output = repo.path().join("selection.json");
        fs::write(&output, b"stale selection").expect("stale selection file");

        let error = super::run(&status_matches_with_select_out(
            repo.path(),
            &refusal,
            &allowed,
            Some(&output),
        ))
        .expect_err("FR-188 unavailable refusal must not leave a stale selection");

        assert_eq!(error.exit, 3);
        assert!(!output.exists());
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-10")]
    #[test]
    fn select_out_missing_directory_fails_without_partial_files() {
        let (repo, ea, allowed) = command_fixture();
        let parent = repo.path().join("missing-directory");
        let output = parent.join("selection.json");

        let error = super::run(&status_matches_with_select_out(
            repo.path(),
            &ea,
            &allowed,
            Some(&output),
        ))
        .expect_err("missing output directory must fail");

        assert_eq!(error.exit, 2);
        assert!(!output.exists());
        assert!(!parent.exists());
    }

    #[cfg(unix)]
    #[trace("FR-190-AC-11")]
    #[test]
    fn select_out_document_is_accepted_by_run_package_consumer() {
        let (repo, ea, allowed) = command_fixture();
        let source =
            "language qsl\nprofile core\n// formalizes: FR-001-AC-1\nrule P1 { check x }\n";
        committed_sources(repo.path(), &[("approved/rule.native", source)]);
        let symbol = formal_rule_symbol("approved/rule.native", "P1");
        approve_symbols(repo.path(), &ea, &[symbol]);
        let selection_path = repo.path().join("selection.json");

        let (report, selection) = run_with_selection(repo.path(), &ea, &allowed, &selection_path);
        let approved = report["rules"]
            .as_array()
            .expect("status rules")
            .iter()
            .find(|rule| rule["state"] == "approved")
            .expect("approved rule");
        let start = approved["declaration_range"]["start"]
            .as_u64()
            .expect("declaration start");
        let end = approved["declaration_range"]["end"]
            .as_u64()
            .expect("declaration end");
        assert_eq!(selection["spans"].as_array().unwrap().len(), 1);
        assert_eq!(selection["spans"][0]["identity"], "approved/rule.native");
        assert_eq!(selection["spans"][0]["start"], start);
        assert_eq!(selection["spans"][0]["end"], end);

        // The pinned consumer fixture has the same semantic graph. Bind its selected claim and
        // reached expression to the exact source span Quoin just emitted.
        let mut package: Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/formal-rules/checked-package-v2-success.json"
        ))
        .expect("checked package fixture");
        for entry in package["source_map"].as_array_mut().expect("source map") {
            let path = entry["node_id"]["path"][0]
                .as_str()
                .expect("source map node path")
                .to_owned();
            if path == "claim/approved" || path == "value/literal" {
                for region in entry["regions"].as_array_mut().expect("source regions") {
                    region["source"]["authority"] = json!("quoin");
                    region["source"]["identity"] = json!("approved/rule.native");
                    if path == "value/literal" {
                        region["start"] = json!(start);
                        region["end"] = json!(start + 1);
                    } else {
                        region["start"] = json!(start + 1);
                        region["end"] = json!(end);
                    }
                }
            }
        }
        for source in package["lock"]["sources"]
            .as_array_mut()
            .expect("locked sources")
        {
            if source["identity"] == "approved/rule.native" {
                source["authority"] = json!("quoin");
            }
        }
        let package_bytes = serde_json::to_vec(&package).expect("serialize checked package");
        let mut evidence = filament_contract_model::CheckedPackageEvidence::default();
        evidence.support_feature("filament.value.complete/v1");
        let checked = match filament_contract_model::CheckedPackageV2::read(
            &package_bytes,
            filament_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            filament_contract_model::CheckedPackageV2ReadResult::Admitted(package) => *package,
            other => panic!("checked package fixture refused: {other:?}"),
        };
        let selected_claim: filament_contract_model::CheckedNodeId =
            serde_json::from_value(json!({
                "package": "test/formal-rules",
                "path": ["claim/approved"],
                "ordinal": 0
            }))
            .expect("selected claim id");
        let lowered = checked.lower(
            &[selected_claim],
            &filament_contract_model::CompleteLoweringProfileV2 {
                reference: filament_contract_model::LoweringProfileRef {
                    authority: "filament-analyze".into(),
                    identity: "fr190-consumer-test".into(),
                },
                required_source_selections: Vec::new(),
                supported_tags: filament_contract_model::CheckedNodeTag::ALL
                    .iter()
                    .copied()
                    .collect(),
                require_bounds: true,
                work_limit: u64::MAX,
            },
        );
        let package_path = repo.path().join("checked-package.json");
        fs::write(
            &package_path,
            lowered
                .package
                .canonical_bytes()
                .expect("lowered package bytes"),
        )
        .expect("write checked package");
        let output_path = repo.path().join("consumer-report.json");
        let arguments = [
            "--package",
            package_path.to_str().expect("package path"),
            "--output",
            output_path.to_str().expect("output path"),
            "--z3",
            "/usr/bin/false",
            "--cvc5",
            "/usr/bin/false",
            "--producer-execute",
            "/usr/bin/false",
            "--select",
            selection_path.to_str().expect("selection path"),
        ]
        .into_iter()
        .map(std::ffi::OsString::from);
        let command = filament_analyze::parse_run_package_arguments(arguments)
            .expect("run-package arguments accept the selection option");
        let filament_analyze::RunPackageCommand::Run(arguments) = command else {
            panic!("expected run-package command");
        };

        let error = filament_analyze::run_package(&arguments)
            .expect_err("stub solvers stop after the consumer accepts the selection");
        assert_eq!(error.class(), filament_analyze::EXIT_FAILURE, "{error}");
        assert!(!error.message().contains("--select"), "{error}");
    }

    #[cfg(unix)]
    #[trace("FR-188-AC-20")]
    #[test]
    fn signature_verify_refusals_return_no_report_with_the_contract_exit_code() {
        let (repo, _ea, allowed) = command_fixture();
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

        let allowed_refusal = executable(
            repo.path(),
            "ea-allowed-signers-refusal",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_allowed_signers_invalid\",\"message\":\"bad allowed signers\"}'; printf '%s\\n' diagnostic >&2; exit 2",
        );
        let error = super::run(&status_matches(repo.path(), &allowed_refusal, &allowed))
            .expect_err("allowed-signers refusal must not produce a report");
        assert_eq!(error.exit, 2);
        assert!(
            error
                .message
                .contains("signature_verify_allowed_signers_invalid")
        );

        let other_refusal = executable(
            repo.path(),
            "ea-other-refusal",
            "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_request_invalid\",\"message\":\"bad request\"}'; printf '%s\\n' diagnostic >&2; exit 2",
        );
        let error = super::run(&status_matches(repo.path(), &other_refusal, &allowed))
            .expect_err("other refusal must not produce a report");
        assert_eq!(error.exit, 3);
        assert!(error.message.contains("signature_verify_request_invalid"));
    }

    /// A verifier that reports no version yields no report: the version in the
    /// report is the evidence of what verified it.
    #[trace("FR-188-AC-24")]
    #[test]
    fn a_verifier_that_reports_no_version_is_unavailable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("directory");
        for (index, line) in [
            "",
            "engineering-assurance",
            "something-else 1.0.0",
            "engineering-assurance 1.0.0 extra",
        ]
        .into_iter()
        .enumerate()
        {
            let ea = dir.path().join(format!("ea-{index}"));
            fs::write(&ea, format!("#!/bin/sh\necho '{line}'\n")).expect("write");
            let mut permissions = fs::metadata(&ea).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&ea, permissions).expect("permissions");
            assert!(
                matches!(
                    super::tool_version(&ea),
                    Err(super::StatusError::Unavailable(_))
                ),
                "{line:?} was accepted as a version"
            );
        }
        let healthy = executable(dir.path(), "healthy", "exit 0");
        assert_eq!(
            super::tool_version(&healthy).ok().as_deref(),
            Some("0.0.0-test")
        );
    }

    /// Only the status path asks for trailers; the evidence path's request is
    /// unchanged so a verifier that predates `trailer_keys` still serves it.
    #[trace("FR-188-AC-25")]
    #[test]
    fn only_the_status_request_carries_trailer_keys() {
        let (repo, allowed) = (Path::new("/repo"), Path::new("/allowed"));
        let with = signature_request(repo, "HEAD", allowed, None, true);
        assert_eq!(with["trailer_keys"], json!(["Formal-rule"]));
        assert!(
            signature_request(repo, "HEAD", allowed, None, false)
                .get("trailer_keys")
                .is_none()
        );
    }
}
