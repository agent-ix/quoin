// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a `verificationStack` attestation.
//!
//! Ports `validateVerificationStack` (`src/measurement/validate.ts:110-166`).
//! Split out of [`crate::validate`] because it is a self-contained document of
//! its own — six required members, each with its own shape — and folding it
//! back in would put the module over the 500-line soft ceiling for no gain.

use std::collections::BTreeMap;

use quoin_store::{JsonObject, JsonValue};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{
    BuildProfile, ResolvedApparatus, VERIFICATION_STACK_SCHEMA_VERSION,
    VerificationStackAttestation, unrecordable_apparatus_path,
};
use crate::types::ids::NonEmptyText;
use crate::validate::read;

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::CollectionInvalid;

fn refuse(message: impl Into<String>) -> MeasurementError {
    MeasurementError::new(CODE, message.into())
}

/// Read a `verificationStack`, or refuse.
///
/// A stack carries no digest (FR-044-AC-13): a stored collection that still
/// states a `lockDigest` or `executableDigest` is read with those members
/// ignored.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`] for every shape
/// `validateVerificationStack` rejects.
pub(crate) fn verification_stack(
    value: Option<&JsonValue>,
) -> Result<VerificationStackAttestation, MeasurementError> {
    let Some(value) = value else {
        return Err(refuse("schemaVersion 2 requires `verificationStack`"));
    };
    let object = read::object(value, CODE, "schemaVersion 2 requires `verificationStack`")?;
    if read::string(object, "schemaVersion") != Some(VERIFICATION_STACK_SCHEMA_VERSION) {
        return Err(refuse("verificationStack has unsupported schemaVersion"));
    }

    // Every member is checked, even after an earlier one fails, and each
    // failing member's own findings (not just its headline) are kept, so a
    // caller sees every problem with the attestation in one refusal rather
    // than fixing them one round trip at a time (PLAT-929).
    let mut findings = Vec::new();
    let build_profile = note(&mut findings, build_profile(object));
    let capabilities = note(&mut findings, capabilities(object));
    let artifacts = note(&mut findings, artifacts(object));
    let unverified_artifacts = note(&mut findings, unverified_artifacts(object));
    let protected_apparatus = note(&mut findings, protected_apparatus(object));
    if !findings.is_empty() {
        return Err(MeasurementError::with_findings(
            CODE,
            "verificationStack is invalid",
            findings,
        ));
    }

    // Every `note` call above returned `Some`, or `findings` would not be
    // empty; assembled through `?` inside an `Option`-returning closure
    // rather than `unwrap`/`expect`, which production code here may not use.
    (|| {
        Some(VerificationStackAttestation {
            build_profile: build_profile?,
            capabilities: capabilities?,
            artifacts: artifacts?,
            unverified_artifacts: unverified_artifacts?,
            protected_apparatus: protected_apparatus?,
        })
    })()
    .ok_or_else(|| refuse("verificationStack is invalid"))
}

/// Record a member check's failure as a finding and return what succeeded.
///
/// Unlike a `?`, this lets every member be checked even after an earlier one
/// fails (PLAT-929): a failing member's own findings are kept when it has
/// any (so an internally-accumulating check does not
/// lose all but one of its own problems), and its
/// subject otherwise.
fn note<T>(findings: &mut Vec<String>, result: Result<T, MeasurementError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            if error.findings().is_empty() {
                findings.push(error.subject().to_owned());
            } else {
                findings.extend(error.findings().iter().cloned());
            }
            None
        }
    }
}

/// `buildProfile` is optional in a stored record and `release`-only in a new
/// one; the `release`-only half is [`crate::validate::measurement_collection`].
fn build_profile(object: &JsonObject) -> Result<Option<BuildProfile>, MeasurementError> {
    match object.get("buildProfile") {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .and_then(BuildProfile::from_wire)
            .map(Some)
            .ok_or_else(|| {
                refuse("verificationStack.buildProfile must be debug, release, or absent")
            }),
    }
}

fn capabilities(object: &JsonObject) -> Result<Vec<NonEmptyText>, MeasurementError> {
    let non_empty = "verificationStack.capabilities must be a non-empty string array";
    let Some(JsonValue::Array(items)) = object.get("capabilities") else {
        return Err(refuse(non_empty));
    };
    if items.is_empty() {
        return Err(refuse(non_empty));
    }

    // Every entry is checked, even after an earlier one fails, so several
    // malformed capabilities are named individually rather than only the
    // first (PLAT-929).
    let mut findings = Vec::new();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item.as_str().filter(|text| !text.is_empty()) {
            Some(text) => match NonEmptyText::parse(text, CODE, "capability") {
                Ok(capability) => out.push(capability),
                Err(error) => findings.push(error.subject().to_owned()),
            },
            None => findings.push(non_empty.to_owned()),
        }
    }
    if findings.is_empty() {
        Ok(out)
    } else {
        Err(MeasurementError::with_findings(
            CODE,
            "verificationStack.capabilities is invalid",
            findings,
        ))
    }
}

fn artifacts(object: &JsonObject) -> Result<Vec<String>, MeasurementError> {
    let non_empty = "verificationStack.artifacts must be a non-empty array of artifact names";
    let Some(JsonValue::Array(items)) = object.get("artifacts") else {
        return Err(refuse(non_empty));
    };
    if items.is_empty() {
        return Err(refuse(non_empty));
    }

    // Every entry is checked, even after an earlier one fails, so several
    // malformed names are named individually rather than only the first
    // (PLAT-929).
    let mut findings = Vec::new();
    let mut out: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        match item.as_str().filter(|text| !text.is_empty()) {
            Some(name) if out.iter().any(|seen| seen == name) => findings.push(format!(
                "verificationStack.artifacts names `{name}` more than once"
            )),
            Some(name) => out.push(name.to_owned()),
            None => findings.push(non_empty.to_owned()),
        }
    }
    if findings.is_empty() {
        Ok(out)
    } else {
        Err(MeasurementError::with_findings(
            CODE,
            "verificationStack.artifacts is invalid",
            findings,
        ))
    }
}

/// `unverifiedArtifacts`, the `artifacts` names this repository has no
/// filesystem entry for at all (PLAT-969's ruling). Absent reads as empty —
/// evidence written before the ruling, or authored by hand, states nothing
/// here and that is not a defect — and a present member must be an array of
/// non-empty strings. Nothing here cross-checks the names against
/// `artifacts`: this function reads what the stored document says, and
/// [`crate::store::publish`] is the only writer that computes the list from
/// the filesystem, so a document this crate did not write can say anything.
fn unverified_artifacts(object: &JsonObject) -> Result<Vec<String>, MeasurementError> {
    match object.get("unverifiedArtifacts") {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            let mut findings = Vec::new();
            for item in items {
                match item.as_str().filter(|text| !text.is_empty()) {
                    Some(text) => out.push(text.to_owned()),
                    None => findings.push(
                        "verificationStack.unverifiedArtifacts entries must be non-empty strings"
                            .to_owned(),
                    ),
                }
            }
            if findings.is_empty() {
                Ok(out)
            } else {
                Err(MeasurementError::with_findings(
                    CODE,
                    "verificationStack.unverifiedArtifacts is invalid",
                    findings,
                ))
            }
        }
        Some(_) => Err(refuse(
            "verificationStack.unverifiedArtifacts must be an array of strings",
        )),
    }
}

/// `protectedApparatus`, each governing plan's resolved protected apparatus
/// keyed by plan id (PLAT-975). Absent reads as empty: a collection no
/// protecting plan governs states nothing. A present member must be an object
/// of non-empty arrays of single-file paths. As with [`unverified_artifacts`],
/// this reads what the stored document says; [`crate::store::publish`] is the
/// only writer that computes it from the repository.
///
/// # Key validation (PLAT-985, quoin#600 review)
///
/// A plan-id key must be non-empty, the same requirement `MeasurementPlan.id`
/// itself carries. A path must pass [`unrecordable_apparatus_path`] — the same
/// check intake's resolver applies before it writes a path — so it parses as a
/// single-file `ApparatusPath` and is not a `<directory>/**` entry: a glob or
/// a path-traversal segment is not a shape intake ever wrote, and a later
/// `git diff` over the paths must not be pointed outside the repository.
fn protected_apparatus(
    object: &JsonObject,
) -> Result<BTreeMap<String, ResolvedApparatus>, MeasurementError> {
    let Some(value) = object.get("protectedApparatus") else {
        return Ok(BTreeMap::new());
    };
    let plans = read::object(
        value,
        CODE,
        "verificationStack.protectedApparatus must be an object keyed by plan id",
    )?;
    let mut findings = Vec::new();
    let mut out = BTreeMap::new();
    for (plan, files) in plans.iter() {
        if plan.is_empty() {
            findings
                .push("verificationStack.protectedApparatus has an empty plan-id key".to_owned());
            continue;
        }
        let files = match files {
            JsonValue::Array(files) if !files.is_empty() => files,
            _ => {
                findings.push(format!(
                    "verificationStack.protectedApparatus.{plan} must be a non-empty array of \
                     file paths"
                ));
                continue;
            }
        };
        let mut resolved = ResolvedApparatus::new();
        for file in files {
            let Some(file) = file.as_str() else {
                findings.push(format!(
                    "verificationStack.protectedApparatus.{plan} must hold only file paths"
                ));
                continue;
            };
            if let Some(reason) = unrecordable_apparatus_path(file) {
                findings.push(format!(
                    "verificationStack.protectedApparatus.{plan} has an invalid path \
                     `{file}`: {reason}"
                ));
                continue;
            }
            resolved.insert(file.to_owned());
        }
        out.insert(plan.clone(), resolved);
    }
    if findings.is_empty() {
        Ok(out)
    } else {
        Err(MeasurementError::with_findings(
            CODE,
            "verificationStack.protectedApparatus is invalid",
            findings,
        ))
    }
}

/// The `verificationStack` members intake computes itself (PLAT-969,
/// PLAT-975).
const COMPUTED_MEMBERS: [&str; 2] = ["unverifiedArtifacts", "protectedApparatus"];

/// One finding per computed member a new candidate states. A caller that
/// states either is buggy or is writing its own answer, and neither is
/// silently overwritten.
pub(crate) fn stated_computed_members(candidate: &JsonValue) -> Vec<String> {
    let Some(JsonValue::Object(stack)) = candidate
        .as_object()
        .ok()
        .and_then(|object| object.get("verificationStack"))
    else {
        return Vec::new();
    };
    COMPUTED_MEMBERS
        .into_iter()
        .filter(|member| stack.contains(member))
        .map(|member| {
            format!(
                "verificationStack.{member} is computed by intake and must not be stated by the \
                 candidate"
            )
        })
        .collect()
}
