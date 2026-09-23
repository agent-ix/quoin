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

/// The three language toolchains a collection may pin.
///
/// A collection rarely touches every language: a measurement that only ran
/// Rust has nothing true to say about `node` or `python`. Each member is
/// therefore `Option` rather than a placeholder string — `None` means "not
/// applicable" (PLAT-930). There is deliberately no second, sentinel spelling
/// of the same absence (a literal `"not-applicable"` string); that would just
/// give a caller two ways to say one thing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toolchains {
    /// The Node.js toolchain, when this measurement touched it.
    pub node: Option<NonEmptyText>,
    /// The Rust toolchain, when this measurement touched it.
    pub rust: Option<NonEmptyText>,
    /// The Python toolchain, when this measurement touched it.
    pub python: Option<NonEmptyText>,
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
    /// Names from `artifacts` this repository holds no filesystem entry for,
    /// sorted.
    ///
    /// The owner's ruling on PLAT-969: a `verificationStack.artifacts` name
    /// with no local entry stays admitted as a label — it is not, and never
    /// was, this crate's place to require every producer's declared artifact
    /// to be locally reachable — but admission must never be silent. Intake
    /// ([`crate::store::publish`]) computes this list itself at write time and
    /// merges it into the stored bytes; a stored collection this crate did not
    /// write (historical evidence, or one authored directly) carries whatever
    /// it happened to state here, or nothing.
    pub unverified_artifacts: Vec<String>,
    /// Each governing plan's resolved protected apparatus, keyed by plan id
    /// (PLAT-975, engineering-assurance FR-024).
    ///
    /// Intake ([`crate::store::publish`]) resolves every `protected_apparatus`
    /// entry of every plan governing an observation, digests each file, and
    /// merges this member into the stored bytes, so a later comparison reads
    /// the apparatus as it was when the collection was written rather than as
    /// the disk holds it today. A plan that protects nothing has no entry, and
    /// a collection no such plan governs states nothing here. Like
    /// `unverifiedArtifacts`, a stored collection this crate did not write
    /// carries whatever it happened to state.
    pub protected_apparatus: BTreeMap<String, ResolvedApparatus>,
}

/// One plan's resolved protected apparatus: every file its entries named,
/// repository-relative and `/`-separated, and that file's digest when the
/// collection was written.
///
/// What an apparatus comparison compares is this set of (path, digest)
/// pairs, so a file added under or removed from a `<directory>/**` entry is a
/// change exactly as an edited file is (engineering-assurance FR-024).
pub type ResolvedApparatus = BTreeMap<String, RawFileSha256Digest>;

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

impl MeasurementCollection {
    /// The resolved protected apparatus this collection recorded for
    /// `plan_id`, or `None` when it recorded none (PLAT-975).
    #[must_use]
    pub fn protected_apparatus_of(&self, plan_id: &str) -> Option<&ResolvedApparatus> {
        self.verification_stack
            .as_ref()
            .and_then(|stack| stack.protected_apparatus.get(plan_id))
    }
}
