// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The inputs FR-211 evidence resolution and approval verification take: the
//! clause policy of the installed clause set and the engineering-assurance
//! trust a caller supplies.

use std::path::Path;

use quoin_assurance::{
    ApprovalVerifier, CheckError, DischargeChecks, DischargeReport, DischargeStoreError,
    check_report,
};
use quoin_quire_types::{ClauseSetKey, SetPolicy};
use serde::Deserialize;

use crate::capabilities::{ApprovalSetup, Capabilities, ModuleHost};
use crate::error::{CoreError, CoreErrorCode};

/// The engineering-assurance inputs of FR-188, as a request carries them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalInputs {
    /// The absolute path of the engineering-assurance executable.
    pub engineering_assurance: String,
    /// The absolute path of the allowed-signers file.
    pub allowed_signers: String,
    /// The gitsign identity mapping; all three members or none.
    #[serde(default)]
    pub gitsign: Option<GitsignInputs>,
}

/// The gitsign identity mapping of FR-040.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GitsignInputs {
    /// The certificate identity.
    pub certificate_identity: String,
    /// The certificate OIDC issuer.
    pub certificate_oidc_issuer: String,
    /// The principal the identity maps to.
    pub principal: String,
}

fn bad(op: &'static str, message: impl Into<String>) -> CoreError {
    CoreError::new(CoreErrorCode::BadRequest, message).with_context("op", op)
}

/// Validate the inputs and bind them to the repository.
///
/// # Errors
/// [`CoreErrorCode::BadRequest`] for a relative path, or a gitsign value that
/// is empty, over 256 bytes or holds a control character.
pub(super) fn approval_setup(
    op: &'static str,
    repo: &str,
    inputs: &ApprovalInputs,
) -> Result<ApprovalSetup, CoreError> {
    for (name, path) in [
        ("engineeringAssurance", &inputs.engineering_assurance),
        ("allowedSigners", &inputs.allowed_signers),
    ] {
        if !Path::new(path).is_absolute() {
            return Err(bad(op, format!("{name} must be an absolute path")));
        }
    }
    let gitsign = inputs
        .gitsign
        .as_ref()
        .map(|mapping| {
            let values = [
                mapping.certificate_identity.clone(),
                mapping.certificate_oidc_issuer.clone(),
                mapping.principal.clone(),
            ];
            if values.iter().any(|value| {
                value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
            }) {
                return Err(bad(
                    op,
                    "a gitsign value is empty, too long or holds a control character",
                ));
            }
            Ok(values)
        })
        .transpose()?;
    let repository = std::path::absolute(repo).map_err(|error| {
        bad(
            op,
            format!("the repository path cannot be made absolute: {error}"),
        )
    })?;
    Ok(ApprovalSetup {
        engineering_assurance: inputs.engineering_assurance.clone().into(),
        allowed_signers: inputs.allowed_signers.clone().into(),
        repository,
        gitsign,
    })
}

/// A verifier for the supplied inputs, or none when the caller supplied none.
///
/// # Errors
/// As [`approval_setup`], and [`CoreErrorCode::Io`] when inputs were supplied
/// and this build granted no approval host.
pub(super) fn verifier_for<'a>(
    capabilities: &Capabilities<'a>,
    setup: Option<&'a ApprovalSetup>,
    op: &'static str,
) -> Result<Option<Box<dyn ApprovalVerifier + 'a>>, CoreError> {
    let Some(setup) = setup else {
        return Ok(None);
    };
    let host = capabilities.approval.ok_or_else(|| {
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a discharge operation without granting an approval host",
        )
        .with_context("op", op)
    })?;
    Ok(Some(host.verifier(setup)))
}

/// The policy of every clause of a clause set, from the
/// unique installed module that declares it. `None` when no module host was
/// granted or no installed module declares the set; the clauses of such a set
/// are never discharged.
///
/// # Errors
/// [`CoreErrorCode::Refused`] when a module fails to load or two installed
/// modules declare the set.
pub(crate) fn policy_for_binding(
    modules: Option<&dyn ModuleHost>,
    set_key: &ClauseSetKey,
    op: &'static str,
) -> Result<Option<SetPolicy>, CoreError> {
    let Some(modules) = modules else {
        return Ok(None);
    };
    let refused =
        |detail: String| CoreError::new(CoreErrorCode::Refused, detail).with_context("op", op);
    let installed = modules
        .list(None)
        .map_err(|error| refused(format!("installed modules cannot be listed: {error}")))?;
    let mut owner = None;
    for module in &installed {
        let policies = modules
            .clause_policies(module, &installed)
            .map_err(|error| {
                refused(format!(
                    "module {} failed to load clause policies: {error}",
                    module.name
                ))
            })?;
        for (set, policy) in policies {
            if set.authority.as_str() == set_key.authority
                && set.id.as_str() == set_key.id
                && set.version.as_str() == set_key.version
            {
                if owner.is_some() {
                    return Err(refused(format!(
                        "multiple installed modules declare clause set {}/{}/{}",
                        set_key.authority, set_key.id, set_key.version
                    )));
                }
                owner = Some(policy);
            }
        }
    }
    Ok(owner)
}

/// The refusal name a caller maps to an exit status: the allowed-signers file
/// refusal exits 2, an unavailable verifier exits 3.
pub(super) fn store_error_context(error: &DischargeStoreError) -> Option<&'static str> {
    match error {
        DischargeStoreError::Check(_) => Some("allowed-signers-invalid"),
        DischargeStoreError::FactsFailed(failures)
            if failures
                .iter()
                .any(|failure| failure.reason.starts_with("approval-verifier-unavailable")) =>
        {
            Some("approval-verifier-unavailable")
        }
        DischargeStoreError::FactsFailed(_) => Some("discharge-facts-failed"),
        _ => None,
    }
}

/// The refusal for a verifier that rejected its trust file.
pub(crate) fn check_error(op: &'static str, error: &CheckError) -> CoreError {
    CoreError::new(CoreErrorCode::Refused, error.to_string())
        .with_context("op", op)
        .with_context("refusal", "allowed-signers-invalid")
}

/// Re-run evidence resolution and approval verification over a report the
/// caller already holds (FR-211).
///
/// `repo` names the store and the repository holding the approval tags;
/// without it every direct fact and approval-class clause is open with
/// `no-store`.
///
/// # Errors
/// [`CoreErrorCode::BadRequest`] for approval inputs without a repository or
/// invalid inputs; [`CoreErrorCode::Refused`] when the verifier refuses its
/// trust file or the clause set cannot be resolved to one installed module.
pub fn check_supplied_report(
    op: &'static str,
    report: DischargeReport,
    repo: Option<&str>,
    approval: Option<&ApprovalInputs>,
    capabilities: &Capabilities<'_>,
) -> Result<DischargeReport, CoreError> {
    let policy = policy_for_binding(capabilities.modules, &report.clause_set, op)?;
    let setup = match (approval, repo) {
        (Some(inputs), Some(repo)) => Some(approval_setup(op, repo, inputs)?),
        (Some(_), None) => return Err(bad(op, "approval inputs need a repository")),
        (None, _) => None,
    };
    let verifier = verifier_for(capabilities, setup.as_ref(), op)?;
    let checks = DischargeChecks {
        policy: policy.as_ref(),
        verifier: verifier.as_deref(),
        offline: false,
    };
    let Some(repo) = repo else {
        return check_report(report, None, &checks, None)
            .map(|checked| checked.report)
            .map_err(|error| check_error(op, &error));
    };
    let mut outcome = None;
    super::host(capabilities, op)?.with_store(Path::new(repo), &mut |source| {
        outcome = Some(check_report(report.clone(), Some(&*source), &checks, None));
        Ok(serde_json::Value::Null)
    })?;
    match outcome {
        Some(Ok(checked)) => Ok(checked.report),
        Some(Err(error)) => Err(check_error(op, &error)),
        None => Err(
            CoreError::new(CoreErrorCode::Io, "the evidence host did not run the check")
                .with_context("op", op),
        ),
    }
}
