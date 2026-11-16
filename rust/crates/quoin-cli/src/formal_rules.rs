// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Read-only, git-derived formal-rule approval status (VER-101).

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use quoin_quire::{ModuleSelection, ScopeRoot};
use serde_json::{Value, json};

const REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";

pub(crate) fn command() -> Command {
    Command::new("formal-rules")
        .about("Read formal-rule approval state from signed git commits")
        .subcommand_required(true)
        .subcommand(
            Command::new("status")
                .about("Report the current approval state of formalized QSL declarations")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .value_name("DIR")
                        .default_value("."),
                )
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("engineering-assurance")
                        .long("engineering-assurance")
                        .value_name("ABSOLUTE-PATH"),
                )
                .arg(
                    Arg::new("allowed-signers")
                        .long("allowed-signers")
                        .value_name("ABSOLUTE-PATH"),
                )
                .arg(
                    Arg::new("gitsign-certificate-identity")
                        .long("gitsign-certificate-identity")
                        .value_name("IDENTITY"),
                )
                .arg(
                    Arg::new("gitsign-certificate-oidc-issuer")
                        .long("gitsign-certificate-oidc-issuer")
                        .value_name("ISSUER"),
                )
                .arg(
                    Arg::new("gitsign-principal")
                        .long("gitsign-principal")
                        .value_name("PRINCIPAL"),
                ),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Response, crate::ShellError> {
    let Some(("status", args)) = matches.subcommand() else {
        return Err(crate::ShellError::usage(
            "an unknown formal-rules command reached dispatch",
        ));
    };
    for name in ["engineering-assurance", "allowed-signers"] {
        let Some(value) = args.get_one::<String>(name) else {
            return Err(crate::ShellError::usage_refused(format!(
                "--{name} is required"
            )));
        };
        if value.is_empty() || value.contains('\0') || !Path::new(value).is_absolute() {
            return Err(crate::ShellError::usage_refused(format!(
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
        return Err(crate::ShellError::usage_refused(
            "all three gitsign identity options must be supplied together",
        ));
    }
    if gitsign_values
        .iter()
        .flatten()
        .any(|value| !valid_gitsign_value(value))
        || gitsign_values[0].is_some_and(|value| value.starts_with('-'))
        || gitsign_values[1].is_some_and(|value| value.starts_with('-'))
    {
        return Err(crate::ShellError::usage_refused(
            "gitsign identity values violate FR-040 validation",
        ));
    }
    let report = status(args).map_err(status_shell_error)?;
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

enum StatusError {
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

fn status(args: &ArgMatches) -> Result<Value, StatusError> {
    let repo = PathBuf::from(
        args.get_one::<String>("repo")
            .ok_or_else(|| "--repo default is unavailable".to_owned())?,
    );
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let ea = absolute_file(args, "engineering-assurance")?;
    let allowed = absolute_file(args, "allowed-signers")?;
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
            ));
        }
    };

    let scope = ScopeRoot::open(&repo).map_err(|error| error.to_string())?;
    let mut notices = Vec::new();
    let registry = ModuleSelection::ScopeOrAmbient
        .resolve(&scope, &mut notices)
        .map_err(|error| error.to_string())?;
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
        let result = verify(&ea, &allowed, &repo, commit, gitsign.as_ref())?;
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
    Ok(json!({"protocol":"quoin.formal-rule-status/v1", "rules":rules}))
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
    let request = signature_request(repo, revision, allowed, gitsign);
    let mut child = ProcessCommand::new(ea)
        .args(["signature-verify", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("engineering-assurance is unavailable: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "engineering-assurance stdin unavailable".to_owned())?
        .write_all(request.to_string().as_bytes())
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
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

fn signature_request(
    repo: &Path,
    revision: &str,
    allowed: &Path,
    gitsign: Option<&Value>,
) -> Value {
    let mut request = serde_json::Map::from_iter([
        ("protocol".to_owned(), json!(REQUEST_PROTOCOL)),
        ("repository".to_owned(), json!(repo)),
        ("revision".to_owned(), json!(revision)),
        ("kind".to_owned(), json!("commit")),
        ("allowed_signers".to_owned(), json!(allowed)),
        ("trailer_keys".to_owned(), json!(["Formal-rule"])),
    ]);
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
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write executable");
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
        command()
            .try_get_matches_from([
                "formal-rules",
                "status",
                "--repo",
                repo.to_str().expect("repo path is UTF-8"),
                "--engineering-assurance",
                ea.to_str().expect("EA path is UTF-8"),
                "--allowed-signers",
                allowed.to_str().expect("allowed signers path is UTF-8"),
                "--json",
            ])
            .expect("status arguments")
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
        let without = signature_request(repo, "HEAD", allowed, None);
        assert!(without.get("gitsign").is_none());
        let gitsign = json!({
            "certificate_identity":"identity",
            "certificate_oidc_issuer":"issuer",
            "principal":"alice"
        });
        let with = signature_request(repo, "HEAD", allowed, Some(&gitsign));
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
}
