// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a `verificationStack` attestation.
//!
//! Ports `validateVerificationStack` (`src/measurement/validate.ts:110-166`).
//! Split out of [`crate::validate`] because it is a self-contained document of
//! its own — six required members, each with its own shape — and folding it
//! back in would put the module over the 500-line soft ceiling for no gain.

use std::collections::BTreeMap;

use engineering_assurance::measurement::ApparatusPath;
use quoin_store::{JsonObject, JsonValue, RawFileSha256Digest};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{
    BuildProfile, CleanSourceState, ResolvedApparatus, SourceAttestation, Toolchains,
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
/// `config_digest` is the collection envelope's own `configDigest`, already
/// read (and confirmed non-empty) by
/// [`crate::validate::parse_stored_measurement_collection`] before this runs
/// — it is not a member of the `verificationStack` object itself. It is
/// checked here anyway: `verify_local_artifacts`'s doc comment
/// (`crate::store::publish`) already treats `lockDigest`, `executableDigest`
/// and `configDigest` as the three digests the stack attests, and until this
/// (PLAT-939) `configDigest` was the one of the three accepted on any
/// non-empty string, never confirmed to even be shaped like a sha256 digest.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`] for every shape
/// `validateVerificationStack` rejects, plus a `configDigest` that is not a
/// full sha256 digest.
pub(crate) fn verification_stack(
    value: Option<&JsonValue>,
    config_digest: &str,
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
    // than fixing them one round trip at a time (PLAT-929). `configDigest`'s
    // shape joins the same accumulation rather than short-circuiting, for the
    // same reason (PLAT-939).
    let mut findings = Vec::new();
    let lock_digest = note(&mut findings, digest(object, "lockDigest"));
    let executable_digest = note(&mut findings, digest(object, "executableDigest"));
    if let Err(error) = config_digest_shape(config_digest) {
        findings.push(error.subject().to_owned());
    }
    let build_profile = note(&mut findings, build_profile(object));
    let toolchains = note(&mut findings, toolchains(object));
    let sources = note(&mut findings, sources(object));
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
            lock_digest: lock_digest?,
            executable_digest: executable_digest?,
            build_profile: build_profile?,
            toolchains: toolchains?,
            sources: sources?,
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
/// any (so an internally-accumulating check like [`toolchains`] or
/// [`sources`] does not lose all but one of its own problems), and its
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

/// Confirm `configDigest` is shaped like a sha256 digest.
///
/// Unlike [`digest`], the caller already has the text in hand — `configDigest`
/// lives on the collection envelope, not inside a `verificationStack` member
/// object — so this only checks shape and discards the parsed value; the
/// envelope keeps carrying `configDigest` as [`crate::types::ids::NonEmptyText`]
/// (schemaVersion 1 collections still accept any non-empty string here, since
/// this check only runs for schemaVersion 2, same as `lockDigest` and
/// `executableDigest`).
fn config_digest_shape(text: &str) -> Result<(), MeasurementError> {
    RawFileSha256Digest::parse_stored(text)
        .map(|_| ())
        .map_err(|_| refuse("collection.configDigest must be a full sha256 digest"))
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

    // Every language is checked, even after an earlier one fails, so several
    // malformed toolchain identities are named individually rather than only
    // the first (PLAT-929).
    let mut findings = Vec::new();
    let node = note(
        &mut findings,
        toolchain_identity(toolchains, TOOLCHAIN_NAMES[0]),
    );
    let rust = note(
        &mut findings,
        toolchain_identity(toolchains, TOOLCHAIN_NAMES[1]),
    );
    let python = note(
        &mut findings,
        toolchain_identity(toolchains, TOOLCHAIN_NAMES[2]),
    );
    if !findings.is_empty() {
        return Err(MeasurementError::with_findings(
            CODE,
            "verificationStack.toolchains is invalid",
            findings,
        ));
    }
    let (node, rust, python) = (|| Some((node?, rust?, python?)))()
        .ok_or_else(|| refuse("verificationStack.toolchains is invalid"))?;

    // PLAT-930 made each language optional, but a stack that pins none of
    // them names nothing: an empty `{}` must not satisfy "toolchains was
    // supplied" any more than an absent member would. Require at least one.
    if node.is_none() && rust.is_none() && python.is_none() {
        return Err(refuse(
            "verificationStack.toolchains must name at least one language; mark a language not \
             used as absent or null rather than omitting all three",
        ));
    }

    Ok(Some(Toolchains { node, rust, python }))
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
    let non_empty = "verificationStack.sources must be a non-empty object";
    let Some(JsonValue::Object(sources)) = object.get("sources") else {
        return Err(refuse(non_empty));
    };
    if sources.is_empty() {
        return Err(refuse(non_empty));
    }

    // Every named source is checked, even after an earlier one fails, so
    // several malformed sources are named individually rather than only the
    // first (PLAT-929).
    let mut findings = Vec::new();
    let mut out = BTreeMap::new();
    for (name, value) in sources.iter() {
        match source_attestation(name, value) {
            Ok(source) => {
                out.insert(name.clone(), source);
            }
            Err(error) => findings.push(error.subject().to_owned()),
        }
    }
    if findings.is_empty() {
        Ok(out)
    } else {
        Err(MeasurementError::with_findings(
            CODE,
            "verificationStack.sources is invalid",
            findings,
        ))
    }
}

/// One named entry of `verificationStack.sources`.
fn source_attestation(
    name: &str,
    value: &JsonValue,
) -> Result<SourceAttestation, MeasurementError> {
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
    Ok(SourceAttestation {
        revision,
        source_state: CleanSourceState,
        remote,
    })
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

fn artifacts(
    object: &JsonObject,
) -> Result<BTreeMap<String, RawFileSha256Digest>, MeasurementError> {
    let non_empty = "verificationStack.artifacts must be a non-empty object";
    let Some(JsonValue::Object(artifacts)) = object.get("artifacts") else {
        return Err(refuse(non_empty));
    };
    if artifacts.is_empty() {
        return Err(refuse(non_empty));
    }

    // Every named artifact is checked, even after an earlier one fails, so
    // several malformed digests are named individually rather than only the
    // first (PLAT-929).
    let mut findings = Vec::new();
    let mut out = BTreeMap::new();
    for (name, value) in artifacts.iter() {
        match value
            .as_str()
            .and_then(|text| RawFileSha256Digest::parse_stored(text).ok())
        {
            Some(digest) => {
                out.insert(name.clone(), digest);
            }
            None => findings.push(format!(
                "verificationStack.artifacts.{name} must be a full sha256 digest"
            )),
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
/// protecting plan governs states nothing, and neither does evidence written
/// before PLAT-975. A present member must be an object of non-empty objects
/// whose members are full sha256 digests. As with [`unverified_artifacts`],
/// this reads what the stored document says; [`crate::store::publish`] is the
/// only writer that computes it from the repository.
///
/// # Key validation (PLAT-985, quoin#600 review)
///
/// Both levels of key are validated, not just the digests they lead to. A
/// plan-id key must be non-empty, the same requirement `MeasurementPlan.id`
/// itself carries — an empty key can never name a real plan, and reading one
/// as a match for `""` would be a bug, not a feature. A path key must parse as
/// [`ApparatusPath`] and must not be a `<directory>/**` entry: a resolved
/// record names one concrete file per member, at the digest intake computed
/// for it, so a glob or a path-traversal segment here is not a shape intake
/// ever wrote and is refused rather than carried through as an opaque string
/// a later consumer — a `git show <sourceRevision>:<path>` check among them —
/// could be pointed at outside the repository.
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
            JsonValue::Object(files) if !files.is_empty() => files,
            _ => {
                findings.push(format!(
                    "verificationStack.protectedApparatus.{plan} must be a non-empty object of \
                     file digests"
                ));
                continue;
            }
        };
        let mut resolved = ResolvedApparatus::new();
        for (file, digest) in files.iter() {
            match ApparatusPath::new(file.as_str()) {
                Ok(path) if path.directory().is_some() => {
                    findings.push(format!(
                        "verificationStack.protectedApparatus.{plan} names `{file}`, a \
                         `<directory>/**` entry; a resolved record names one concrete file per \
                         member"
                    ));
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    findings.push(format!(
                        "verificationStack.protectedApparatus.{plan} has an invalid path key \
                         `{file}`: {error}"
                    ));
                    continue;
                }
            }
            match digest
                .as_str()
                .and_then(|text| RawFileSha256Digest::parse_stored(text).ok())
            {
                Some(digest) => {
                    resolved.insert(file.clone(), digest);
                }
                None => findings.push(format!(
                    "verificationStack.protectedApparatus.{plan}.{file} must be a full sha256 \
                     digest"
                )),
            }
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
