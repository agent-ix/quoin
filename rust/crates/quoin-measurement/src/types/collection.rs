// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One producer invocation, and the verification stack that produced it.
//!
//! Ports `MeasurementCollection` and `VerificationStackAttestation` from
//! `src/measurement/types.ts:28-58`.

use std::collections::BTreeMap;

use quoin_store::{JsonObject, JsonValue, RawFileSha256Digest};

use crate::types::ids::{FullGitRevision, NonEmptyText};
use crate::types::observation::MeasurementObservation;

/// The schema version new collections must carry.
pub const MEASUREMENT_SCHEMA_VERSION: u32 = 2;

/// Schema versions that remain readable as historical evidence.
pub const HISTORICAL_MEASUREMENT_SCHEMA_VERSIONS: [u32; 1] = [1];

/// The one `schemaVersion` a verification-stack attestation carries.
pub const VERIFICATION_STACK_SCHEMA_VERSION: &str = "verification-stack-attestation-v1";

/// Which build the measured executable came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum BuildProfile {
    /// A debug build. Readable as history; refused for a new collection.
    Debug,
    /// A release build.
    Release,
}

impl BuildProfile {
    /// Every profile, in declaration order.
    pub const ALL: [Self; 2] = [Self::Debug, Self::Release];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    /// Recover a profile from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// The three language toolchains a collection pins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toolchains {
    /// The Node.js toolchain.
    pub node: NonEmptyText,
    /// The Rust toolchain.
    pub rust: NonEmptyText,
    /// The Python toolchain.
    pub python: NonEmptyText,
}

/// One source repository, pinned at a clean full revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAttestation {
    /// The full git revision the source was at.
    pub revision: FullGitRevision,
    /// Always `clean`: a dirty tree is not an attestation.
    pub source_state: CleanSourceState,
    /// Where the source came from.
    pub remote: NonEmptyText,
}

/// The only source state an attestation may carry.
///
/// A unit type rather than a `bool` or a `String`: `validate.ts:143` accepts
/// exactly `"clean"`, so there is one inhabitant and holding it is the proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CleanSourceState;

impl CleanSourceState {
    /// The wire spelling.
    pub const AS_STR: &'static str = "clean";
}

/// The immutable identity of everything that produced a collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationStackAttestation {
    /// The dependency lock's digest.
    pub lock_digest: RawFileSha256Digest,
    /// The measured executable's digest.
    pub executable_digest: RawFileSha256Digest,
    /// Which build the executable came from, when declared.
    pub build_profile: Option<BuildProfile>,
    /// The pinned toolchains, when declared.
    pub toolchains: Option<Toolchains>,
    /// Every source repository, non-empty.
    pub sources: BTreeMap<String, SourceAttestation>,
    /// The capabilities the producer exercised, non-empty.
    pub capabilities: Vec<NonEmptyText>,
    /// Every produced artifact and its digest, non-empty.
    pub artifacts: BTreeMap<String, RawFileSha256Digest>,
}

/// One producer invocation. All observations land atomically as this unit.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementCollection {
    /// `1` for historical evidence, [`MEASUREMENT_SCHEMA_VERSION`] for new.
    pub schema_version: u32,
    /// This invocation's identity, which is also its file name.
    pub collection_id: NonEmptyText,
    /// What was measured.
    pub subject: NonEmptyText,
    /// The producer-defined scope, opaque to this crate.
    pub scope: JsonValue,
    /// Which tool produced it.
    pub tool_identity: NonEmptyText,
    /// Which version of that tool.
    pub tool_version: NonEmptyText,
    /// The digest of the configuration it ran under.
    pub config_digest: NonEmptyText,
    /// When it ran.
    ///
    /// Kept as text: `validate.ts:76-84` requires a non-empty string and
    /// nothing more, and narrowing it to an RFC 3339 value here would refuse
    /// retained evidence this port must keep reading.
    pub timestamp: NonEmptyText,
    /// The revision of the measured source.
    pub source_revision: NonEmptyText,
    /// The revision of the measured corpus, when there is one.
    pub corpus_revision: Option<String>,
    /// The environment the producer reported.
    ///
    /// `types.ts:37` declares `Record<string, string>` and `validate.ts:86`
    /// checks only object-ness, so the port checks object-ness.
    pub environment: JsonObject,
    /// Required in v2: the immutable identity of every input.
    pub verification_stack: Option<VerificationStackAttestation>,
    /// The observations, non-empty.
    pub observations: Vec<MeasurementObservation>,
    /// Complete producer output; report views derive rather than transcribe.
    pub raw_evidence: JsonValue,
}
