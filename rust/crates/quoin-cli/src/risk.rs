// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin risk advise` (FR-145) and `quoin risk discrepancies` (FR-154): the
//! read-only risk reports.
//!
//! The reports are built by `quoin-risk-advice`; this module is argv, a refusal
//! for a missing scope, and the choice of format. It writes no file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::{Diagnostic, Response};
use quoin_quire::{ModuleRoot, ModuleSelection};
use serde_json::Value;

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Advise on risk decisions without making them.";

fn scope_arg() -> Arg {
    Arg::new("scope")
        .value_name("SCOPE")
        .default_value(".")
        .help("Directory holding the safety records, policies, requirements and anomalies")
}

fn format_arg() -> Arg {
    Arg::new("format")
        .long("format")
        .value_name("FORMAT")
        .default_value("markdown")
        .value_parser(["markdown", "json"])
        .help("Output format")
}

pub(crate) fn module_arg() -> Arg {
    Arg::new("module")
        .long("module")
        .value_name("DIR")
        .action(clap::ArgAction::Append)
        .help("Module root that declares the safety record types; repeat to add roots. Without it the installed modules are searched")
}

pub(crate) fn command() -> Command {
    Command::new("risk")
        .about(ABOUT)
        .subcommand_required(true)
        .subcommand(
            Command::new("advise")
                .about("Print the policy cell of each estimate, its evidence gaps and the undecided decisions")
                .arg(scope_arg())
                .arg(format_arg())
                .arg(module_arg())
                .arg(
                    Arg::new("engineering-assurance")
                        .long("engineering-assurance")
                        .value_name("ABSOLUTE-PATH")
                        .help("Engineering Assurance executable used to resolve decision signers"),
                )
                .arg(
                    Arg::new("allowed-signers")
                        .long("allowed-signers")
                        .value_name("ABSOLUTE-PATH")
                        .help("Allowed-signers file passed to signature-verify"),
                ),
        )
        .subcommand(
            Command::new("discrepancies")
                .about("Report where a recorded risk decision disagrees with the authored policy")
                .arg(scope_arg())
                .arg(format_arg())
                .arg(module_arg())
                .arg(
                    Arg::new("strict")
                        .long("strict")
                        .action(clap::ArgAction::SetTrue)
                        .help("Exit 1 when the report has one or more findings"),
                ),
        )
}

/// The one-line summary of `quoin risk-summary`.
pub(crate) const SUMMARY_ABOUT: &str = "Print the hazard-to-decision chain, one row per situation.";

pub(crate) fn summary_command() -> Command {
    Command::new("risk-summary")
        .about(SUMMARY_ABOUT)
        .arg(scope_arg())
        .arg(format_arg())
        .arg(module_arg())
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match arguments.subcommand() {
        Some(("advise", advise)) => run_advise(advise),
        Some(("discrepancies", discrepancies)) => run_discrepancies(discrepancies),
        _ => Err("an unknown risk command reached dispatch".to_owned()),
    }
}

/// The scope and module selection, or the refusal that names what is wrong.
fn inputs(arguments: &ArgMatches) -> Result<(PathBuf, ModuleSelection), Response> {
    let scope = arguments
        .get_one::<String>("scope")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if !scope.is_dir() {
        return Err(refused(
            "risk.scope_missing",
            &format!(
                "scope {} does not exist (no such file or directory)",
                scope.display()
            ),
        ));
    }
    let modules = arguments
        .get_many::<String>("module")
        .map_or(Ok(ModuleSelection::Ambient), |roots| {
            roots
                .map(ModuleRoot::open)
                .collect::<Result<Vec<_>, _>>()
                .map(ModuleSelection::Closed)
        })
        .map_err(|error| refused("risk.module_missing", &error.to_string()))?;
    Ok((scope, modules))
}

/// The `file; absent from the report` diagnostic for each unreadable file.
fn unreadable_diagnostics(files: &[String]) -> Vec<Diagnostic> {
    files
        .iter()
        .map(|file| Diagnostic {
            code: "risk.document_unreadable".to_owned(),
            message: format!("{file}; the file is absent from the report"),
            context: BTreeMap::new(),
        })
        .collect()
}

fn run_advise(arguments: &ArgMatches) -> Result<Response, String> {
    let (scope, modules) = match inputs(arguments) {
        Ok(inputs) => inputs,
        Err(refusal) => return Ok(refusal),
    };
    let mut advice = match quoin_risk_advice::advise(&scope, &modules) {
        Ok(advice) => advice,
        Err(error) => {
            return Ok(refused(
                &format!("risk.{}", error.code.as_str()),
                &error.message,
            ));
        }
    };
    let verifier_paths = match verifier_paths(arguments) {
        Ok(paths) => paths,
        Err(refusal) => return Ok(refusal),
    };
    let Some((ea, allowed_signers)) = verifier_paths else {
        let diagnostics = unreadable_diagnostics(&advice.unreadable);
        let rendered = render_advice(arguments, &advice)?;
        return Ok(Response {
            payload: serde_json::json!({ "rendered": rendered.trim_end() }),
            diagnostics,
            outcome: Outcome::Ok,
        });
    };

    let repository = match scope.canonicalize() {
        Ok(repository) => repository,
        Err(error) => {
            return Ok(refused(
                "risk.scope_unreadable",
                &format!("cannot resolve scope {}: {error}", scope.display()),
            ));
        }
    };
    let commits: BTreeSet<String> = advice
        .decision_attribution
        .iter()
        .filter_map(|entry| entry.decision_commit().map(str::to_owned))
        .collect();
    let mut signer_principals = BTreeMap::new();
    for commit in commits {
        let result =
            match crate::formal_rules::verify_for_risk(&ea, &allowed_signers, &repository, &commit)
            {
                Ok(result) => result,
                Err(crate::formal_rules::StatusError::Invalid(message)) => {
                    return Ok(refused("risk.allowed_signers_invalid", &message));
                }
                Err(crate::formal_rules::StatusError::Unavailable(message)) => {
                    return Ok(invalid("risk.signature_verification_failed", &message));
                }
            };
        let principal = match verified_signer_principal(&result, &commit) {
            Ok(principal) => principal,
            Err(message) => {
                return Ok(invalid(
                    "risk.signature_verification_result_invalid",
                    &message,
                ));
            }
        };
        signer_principals.insert(commit, principal);
    }
    quoin_risk_advice::apply_verified_signers(&mut advice, &signer_principals);
    let diagnostics = unreadable_diagnostics(&advice.unreadable);
    let rendered = render_advice(arguments, &advice)?;
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered.trim_end() }),
        diagnostics,
        outcome: Outcome::Ok,
    })
}

fn render_advice(
    arguments: &ArgMatches,
    advice: &quoin_risk_advice::Advice,
) -> Result<String, String> {
    match arguments.get_one::<String>("format").map(String::as_str) {
        Some("json") => advice.to_json().map_err(|error| error.to_string()),
        _ => Ok(advice.to_markdown()),
    }
}

fn verifier_paths(arguments: &ArgMatches) -> Result<Option<(PathBuf, PathBuf)>, Response> {
    let ea = arguments.get_one::<String>("engineering-assurance");
    let allowed = arguments.get_one::<String>("allowed-signers");
    let (Some(ea), Some(allowed)) = (ea, allowed) else {
        return if ea.is_none() && allowed.is_none() {
            Ok(None)
        } else {
            Err(refused(
                "risk.signature_options_incomplete",
                "--engineering-assurance and --allowed-signers must be supplied together",
            ))
        };
    };
    let ea = Path::new(ea);
    if !ea.is_absolute() || !ea.is_file() {
        return Err(refused(
            "risk.engineering_assurance_invalid",
            "--engineering-assurance must name an existing absolute file",
        ));
    }
    let allowed = Path::new(allowed);
    if !allowed.is_absolute() || !allowed.is_file() {
        return Err(refused(
            "risk.allowed_signers_invalid",
            "--allowed-signers must name an existing absolute file",
        ));
    }
    Ok(Some((ea.to_path_buf(), allowed.to_path_buf())))
}

fn verified_signer_principal(result: &Value, commit: &str) -> Result<Option<String>, String> {
    if result.get("protocol").and_then(Value::as_str)
        != Some("engineering-assurance.signature-verify-result/v1")
        || result.get("kind").and_then(Value::as_str) != Some("commit")
    {
        return Err("engineering-assurance returned an unsupported signature result".to_owned());
    }
    let status = result
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "engineering-assurance signature result has no status".to_owned())?;
    if !matches!(
        status,
        "signed_valid" | "signed_untrusted" | "signed_bad" | "unsigned" | "unverifiable"
    ) {
        return Err("engineering-assurance returned an unknown signature status".to_owned());
    }
    if let Some(verified_commit) = result.get("verified_commit_id").and_then(Value::as_str)
        && verified_commit != commit
    {
        return Err(format!(
            "engineering-assurance verified {verified_commit}, expected decision commit {commit}"
        ));
    }
    if status != "signed_valid" {
        return Ok(None);
    }
    if result.get("verified_commit_id").and_then(Value::as_str) != Some(commit) {
        return Err(
            "engineering-assurance did not identify the verified decision commit".to_owned(),
        );
    }
    let signer = result
        .get("signer")
        .and_then(Value::as_object)
        .ok_or_else(|| "signed_valid result has no signer".to_owned())?;
    if signer.get("mapping").and_then(Value::as_str) != Some("mapped") {
        return Err("signed_valid result has no mapped signer".to_owned());
    }
    signer
        .get("principal")
        .and_then(Value::as_str)
        .filter(|principal| !principal.trim().is_empty())
        .map(|principal| Some(principal.to_owned()))
        .ok_or_else(|| "mapped signer result has no principal".to_owned())
}

pub(crate) fn run_summary(arguments: &ArgMatches) -> Result<Response, String> {
    let (scope, modules) = match inputs(arguments) {
        Ok(inputs) => inputs,
        Err(refusal) => return Ok(refusal),
    };
    let summary = match quoin_risk_advice::summarize(&scope, &modules) {
        Ok(summary) => summary,
        Err(error) => {
            return Ok(refused(
                &format!("risk.{}", error.code.as_str()),
                &error.message,
            ));
        }
    };
    let diagnostics = unreadable_diagnostics(&summary.unreadable);
    let rendered = match arguments.get_one::<String>("format").map(String::as_str) {
        Some("json") => summary.to_json().map_err(|error| error.to_string())?,
        _ => summary.to_markdown(),
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered.trim_end() }),
        diagnostics,
        outcome: Outcome::Ok,
    })
}

fn run_discrepancies(arguments: &ArgMatches) -> Result<Response, String> {
    let (scope, modules) = match inputs(arguments) {
        Ok(inputs) => inputs,
        Err(refusal) => return Ok(refusal),
    };
    let report = match quoin_risk_advice::discrepancies(&scope, &modules) {
        Ok(report) => report,
        Err(error) => {
            return Ok(refused(
                &format!("risk.{}", error.code.as_str()),
                &error.message,
            ));
        }
    };
    let rendered = match arguments.get_one::<String>("format").map(String::as_str) {
        Some("json") => report.to_json().map_err(|error| error.to_string())?,
        _ => report.to_markdown(),
    };
    let outcome = if arguments.get_flag("strict") && !report.findings.is_empty() {
        Outcome::Partial
    } else {
        Outcome::Ok
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered.trim_end() }),
        diagnostics: Vec::new(),
        outcome,
    })
}

/// A refusal: exit status 2 with one diagnostic, and nothing on stdout.
fn refused(code: &str, message: &str) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }
}

/// An invalid verifier result: exit status 3 and no report.
fn invalid(code: &str, message: &str) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Invalid,
    }
}
