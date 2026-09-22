// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a `verificationStack` attestation.
//!
//! Ports `validateVerificationStack` (`src/measurement/validate.ts:110-166`).
//! Split out of [`crate::validate`] because it is a self-contained document of
//! its own — six required members, each with its own shape — and folding it
//! back in would put the module over the 500-line soft ceiling for no gain.

use std::collections::BTreeMap;

use quoin_store::{JsonObject, JsonValue, RawFileSha256Digest};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{
    BuildProfile, CleanSourceState, SourceAttestation, Toolchains,
    VERIFICATION_STACK_SCHEMA_VERSION, VerificationStackAttestation,
};
use crate::types::ids::{FullGitRevision, NonEmptyText};
use crate::validate::read;

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::CollectionInvalid;

/// The three toolchains a collection pins, in the order the oracle names them.
pub(crate) const TOOLCHAIN_NAMES: [&str; 3] = ["node", "rust", "python"];

fn refuse(message: impl Into<String>) -> MeasurementError {
    MeasurementError::new(CODE, message.into())
}

/// Read a `verificationStack`, or refuse.
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

    // Every member is checked, even after an earlier one fails, so a caller
    // sees every problem with the attestation in one refusal rather than
    // fixing them one round trip at a time (PLAT-929).
    let mut findings = Vec::new();
    note(&mut findings, digest(object, "lockDigest"));
    note(&mut findings, digest(object, "executableDigest"));
    note(&mut findings, build_profile(object));
    note(&mut findings, toolchains(object));
    note(&mut findings, sources(object));
    note(&mut findings, capabilities(object));
    note(&mut findings, artifacts(object));
    if !findings.is_empty() {
        return Err(MeasurementError::with_findings(
            CODE,
            "verificationStack is invalid",
            findings,
        ));
    }

    // Every member above is now known individually valid, so re-reading each
    // to assemble the struct cannot fail here.
    Ok(VerificationStackAttestation {
        lock_digest: digest(object, "lockDigest")?,
        executable_digest: digest(object, "executableDigest")?,
        build_profile: build_profile(object)?,
        toolchains: toolchains(object)?,
        sources: sources(object)?,
        capabilities: capabilities(object)?,
        artifacts: artifacts(object)?,
    })
}

/// Record a member check's failure as a finding; the checked value itself is
/// discarded either way, since the first pass over `verificationStack` exists
/// only to decide whether every member is individually valid.
fn note<T>(findings: &mut Vec<String>, result: Result<T, MeasurementError>) {
    if let Err(error) = result {
        findings.push(error.subject().to_owned());
    }
}

/// One `sha256:<64 hex>` member, read as `quoin-store`'s own digest type.
fn digest(object: &JsonObject, name: &str) -> Result<RawFileSha256Digest, MeasurementError> {
    read::string(object, name)
        .and_then(|text| RawFileSha256Digest::parse_stored(text).ok())
        .ok_or_else(|| {
            refuse(format!(
                "verificationStack.{name} must be a full sha256 digest"
            ))
        })
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

fn toolchains(object: &JsonObject) -> Result<Option<Toolchains>, MeasurementError> {
    let Some(value) = object.get("toolchains") else {
        return Ok(None);
    };
    let toolchains = read::object(
        value,
        CODE,
        "verificationStack.toolchains must be an object",
    )?;
    Ok(Some(Toolchains {
        node: toolchain_identity(toolchains, TOOLCHAIN_NAMES[0])?,
        rust: toolchain_identity(toolchains, TOOLCHAIN_NAMES[1])?,
        python: toolchain_identity(toolchains, TOOLCHAIN_NAMES[2])?,
    }))
}

/// One toolchain identity (PLAT-930): absent or explicit `null` means "not
/// applicable" for this language, and anything else must be a non-empty
/// string. Only the malformed shapes refuse — an empty string, or a value
/// that is neither a string nor `null`.
fn toolchain_identity(
    toolchains: &JsonObject,
    name: &str,
) -> Result<Option<NonEmptyText>, MeasurementError> {
    match toolchains.get(name) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(text)) if !text.is_empty() => {
            NonEmptyText::parse(text, CODE, name).map(Some)
        }
        _ => Err(refuse(format!(
            "verificationStack.toolchains.{name} must be a non-empty string, null, or absent"
        ))),
    }
}

fn sources(object: &JsonObject) -> Result<BTreeMap<String, SourceAttestation>, MeasurementError> {
    let Some(JsonValue::Object(sources)) = object.get("sources") else {
        return Err(refuse(
            "verificationStack.sources must be a non-empty object",
        ));
    };
    if sources.is_empty() {
        return Err(refuse(
            "verificationStack.sources must be a non-empty object",
        ));
    }
    let mut out = BTreeMap::new();
    for (name, value) in sources.iter() {
        let unclean = format!("verificationStack.sources.{name} is not a clean full-SHA source");
        let source = read::object(value, CODE, &unclean)?;
        let revision = read::string(source, "revision").map_or_else(
            || Err(refuse(unclean.clone())),
            |text| FullGitRevision::parse(text, CODE, &unclean),
        )?;
        if read::string(source, "sourceState") != Some(CleanSourceState::AS_STR) {
            return Err(refuse(unclean));
        }
        let remote = read::string(source, "remote")
            .filter(|text| !text.is_empty())
            .map_or_else(
                || Err(refuse(unclean)),
                |text| NonEmptyText::parse(text, CODE, "remote"),
            )?;
        out.insert(
            name.clone(),
            SourceAttestation {
                revision,
                source_state: CleanSourceState,
                remote,
            },
        );
    }
    Ok(out)
}

fn capabilities(object: &JsonObject) -> Result<Vec<NonEmptyText>, MeasurementError> {
    let non_empty = "verificationStack.capabilities must be a non-empty string array";
    let Some(JsonValue::Array(items)) = object.get("capabilities") else {
        return Err(refuse(non_empty));
    };
    if items.is_empty() {
        return Err(refuse(non_empty));
    }
    items
        .iter()
        .map(|item| {
            item.as_str().filter(|text| !text.is_empty()).map_or_else(
                || Err(refuse(non_empty)),
                |text| NonEmptyText::parse(text, CODE, "capability"),
            )
        })
        .collect()
}

fn artifacts(
    object: &JsonObject,
) -> Result<BTreeMap<String, RawFileSha256Digest>, MeasurementError> {
    let Some(JsonValue::Object(artifacts)) = object.get("artifacts") else {
        return Err(refuse(
            "verificationStack.artifacts must be a non-empty object",
        ));
    };
    if artifacts.is_empty() {
        return Err(refuse(
            "verificationStack.artifacts must be a non-empty object",
        ));
    }
    artifacts
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .and_then(|text| RawFileSha256Digest::parse_stored(text).ok())
                .map(|digest| (name.clone(), digest))
                .ok_or_else(|| {
                    refuse(format!(
                        "verificationStack.artifacts.{name} must be a full sha256 digest"
                    ))
                })
        })
        .collect()
}
