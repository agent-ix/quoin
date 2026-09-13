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
    let pinned = "verificationStack.toolchains must pin node, rust, and python";
    let toolchains = read::object(value, CODE, pinned)?;
    let read_one = |name: &str| -> Result<NonEmptyText, MeasurementError> {
        read::string(toolchains, name)
            .filter(|text| !text.is_empty())
            .map_or_else(
                || Err(refuse(pinned)),
                |text| NonEmptyText::parse(text, CODE, name),
            )
    };
    Ok(Some(Toolchains {
        node: read_one(TOOLCHAIN_NAMES[0])?,
        rust: read_one(TOOLCHAIN_NAMES[1])?,
        python: read_one(TOOLCHAIN_NAMES[2])?,
    }))
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
