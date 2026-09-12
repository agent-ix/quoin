// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The engine edge — what `src/quire/contract.ts` was for, and why almost none
//! of it survives the move (quoin#379, EPIC #373 FR-097).
//!
//! ## What `contract.ts` guarded
//!
//! Three distinct things, which the subprocess boundary forced into one file:
//!
//! 1. **Payload shape.** quoin vendored five JSON Schemas out of quire-rs and
//!    validated every payload against them, because a subprocess hands back
//!    text and TypeScript erases at runtime. The vendored copy could drift from
//!    what quire actually emits, so the file recorded a source revision and a
//!    SHA-256 per schema and asserted them on every test run.
//! 2. **A version premise.** `minimumCli: "0.21.0"` — the oldest CLI that emits
//!    the described shapes — checked by parsing `quire --version`.
//! 3. **A capability premise, badly.** Its own comment records that
//!    `minimumCli` is "a CONTRACT floor, not a CAPABILITY floor, and the two
//!    have already drifted once": a consumer on 0.22.0 silently gets no FR-059
//!    vocabulary coverage, the payload parses, and simply contains less.
//!
//! ## What the Cargo edge does with each
//!
//! 1. **Dissolved.** The types a payload is read into are the engine's own
//!    (`quire_rs::CoverageReport`, `quire_rs::ClauseBindingReport`,
//!    `quire_rs::AssuranceExport`). There is no second declaration to drift
//!    from, so there is nothing for a hash to pin. The concrete drift the hash
//!    could *not* catch is on record: the `implements` field was in the
//!    published schema from CR-080 and missing from `src/quire/types.ts` until
//!    CR-083 — the vendored contract and its types had drifted in the one
//!    direction the contract test does not check. A shared type cannot drift in
//!    either direction.
//! 2. **Compile time.** The minimum engine is not a number to compare; it is
//!    the `rev` in `Cargo.toml`. An engine without a surface this crate calls
//!    does not link.
//! 3. **Compile time, honestly.** [`CAPABILITIES`] is asserted by
//!    [`capability_witnesses`], each of which names the engine item behind one
//!    token. The pattern is `quire-cli`'s (`src/engine.rs`, agent-ix/
//!    quire-cli#68) and it is borrowed on purpose: it is the half a version
//!    comparison could never give.
//!
//! ## What survives
//!
//! Reading a **stored** artifact. An evidence-store payload was produced by
//! some other build, possibly months ago, and its `engine` block is the only
//! statement of which. [`check_premise`] is the narrowed remnant of
//! `checkVersionPremise`, and it applies to exactly that case.

use serde::{Deserialize, Serialize};

/// This crate's own version.
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The linked `quire-rs` version, read from this crate's manifest by
/// `build.rs` rather than restated as a constant.
pub const ENGINE_VERSION: &str = env!("QUOIN_QUIRE_ENGINE_VERSION");

/// The exact `quire-rs` object id this build links.
pub const ENGINE_REVISION: &str = env!("QUOIN_QUIRE_ENGINE_REVISION");

/// The oldest engine whose payloads this build will read back off disk.
///
/// Equal to [`ENGINE_VERSION`] deliberately, and that is the whole change in
/// posture: `contract.ts` had to accept a *range* of producers because the
/// producer was whatever binary was on `PATH`. This build has exactly one
/// engine, so anything older is an artifact from a different instrument and is
/// re-measured rather than reinterpreted.
pub const MINIMUM_STORED_ENGINE: &str = ENGINE_VERSION;

/// What this build can emit, as tokens.
///
/// Open vocabulary: adding a token must not break a consumer written against
/// an older list. A consumer asserts it needs `binding_census`; it must never
/// assert `engine >= 0.43.0`, because a version comparison in a consumer is a
/// second place the contract lives and it goes stale.
pub const CAPABILITIES: &[&str] = &[
    // `read_assurance_export` — the engine's own fail-closed assurance-v1
    // reader, which replaces quoin's vendored assurance schema outright.
    "assurance_export.v1",
    // `CoverageReport.binding_census` (quire-rs FR-050-AC-27).
    "binding_census",
    // Rights-aware module clause sets and three-valued applicability
    // (quire-rs FR-067).
    "clause_sets",
    // `CoverageReport.metrics` (quire-rs FR-063).
    "metrics_envelope",
    // `CoverageReport.minted_targets` (quire-rs FR-050-AC-38).
    "minted_targets",
    // `AcClassification` spans and the safe-refusal signal trail (#241).
    "property_spans.safe_refusal",
    // `AcClassification::property.is_specific()` (quire-rs CR-095).
    "specific_shaped",
    // `CoverageReport.suspicions` (quire-rs FR-064).
    "suspicions",
    // `CoverageReport.unmatched_tags` (quire-rs FR-050-AC-39).
    "unmatched_tags",
    // `CoverageReport.vocabulary_coverage` (quire-rs FR-059-AC-9) — the exact
    // capability `contract.ts` recorded itself as unable to express.
    "vocabulary_coverage",
];

/// Which instrument produced a payload.
///
/// Field names are `cli` / `engine` / `capabilities` to match the block
/// `quire-cli` already appends and `src/quire/types.ts` already reads
/// (`EngineProvenance`). During the staged coexistence of EPIC #373 an
/// artifact written here must stay readable by the retained TypeScript, so the
/// key names are a contract, not a preference. `cli` carries this adapter's
/// identity because from the artifact's point of view this crate *is* the
/// instrument's outer half.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The producing surface — `quoin-quire <version>`.
    pub cli: String,
    /// The linked engine version, verbatim.
    pub engine: String,
    /// Capability tokens; see [`CAPABILITIES`].
    pub capabilities: Vec<String>,
}

impl Provenance {
    /// This build's provenance.
    #[must_use]
    pub fn current() -> Self {
        Self {
            cli: format!("quoin-quire {ADAPTER_VERSION}"),
            engine: ENGINE_VERSION.to_string(),
            capabilities: CAPABILITIES
                .iter()
                .map(|token| (*token).to_string())
                .collect(),
        }
    }
}

impl Default for Provenance {
    fn default() -> Self {
        Self::current()
    }
}

/// A three-part numeric version, compared numerically.
///
/// The successor to `compareVersions`, which split on `.` and compared three
/// `parseInt`s. Same semantics, plus a type that cannot be handed a string
/// that never parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// The first `N.N.N` in a string, as `parseCliVersion` took it.
    ///
    /// Tolerant on purpose: an engine version can carry a `-123-gabcdef`
    /// describe suffix, and the three numbers in front of it are the comparable
    /// part.
    ///
    /// Only the first [`Self::SCAN_LIMIT`] bytes are scanned. The argument
    /// comes off a stored artifact, so it is untrusted input of unbounded
    /// length, and a backtracking scan over all of it is quadratic
    /// (rust-review §11). No version string is 256 bytes long.
    #[must_use]
    pub fn parse_first(text: &str) -> Option<Self> {
        let scanned = bounded(text, Self::SCAN_LIMIT);
        (0..scanned.len()).find_map(|start| Self::at(scanned, start))
    }

    /// How much of a supplied string is scanned for a version.
    pub const SCAN_LIMIT: usize = 256;

    /// A version starting exactly at `from`, if one does.
    fn at(text: &str, from: usize) -> Option<Self> {
        let (major, cursor) = number(text, from)?;
        let cursor = dot(text, cursor)?;
        let (minor, cursor) = number(text, cursor)?;
        let cursor = dot(text, cursor)?;
        let (patch, _) = number(text, cursor)?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

/// The longest prefix of `text` no longer than `limit`, cut on a character
/// boundary.
fn bounded(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or("")
}

/// A decimal run at `from`, and the offset just past it.
///
/// `None` for no digits, and for a run too long to be a `u64` — a 30-digit
/// major version is not a version, and silently truncating it would invent one.
fn number(text: &str, from: usize) -> Option<(u64, usize)> {
    let rest = text.get(from..)?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let slice = rest.get(..digits)?;
    let value = slice.parse::<u64>().ok()?;
    Some((value, from + digits))
}

/// The offset just past a `.` at `from`.
fn dot(text: &str, from: usize) -> Option<usize> {
    text.get(from..)?
        .starts_with('.')
        .then_some(from.saturating_add(1))
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What an artifact's `engine.engine` string actually names.
///
/// **Measured, not assumed.** `quire-cli` resolves that field from its own
/// lockfile, and a lockfile entry for a `rev`-pinned git dependency carries the
/// object id — so a real `coverage --json` written by quire-cli 0.32.0 says
/// `"engine": "a874fb641cb70da83c8c8b23f9fea0a44255b88a"`, not `"0.46.0"`.
/// `src/quire/contract.ts` never met this, because it read
/// `quire --version` (where the first `N.N.N` is the **CLI** version) and never
/// looked at a stored payload's provenance block at all.
///
/// Reading it as "a version that failed to parse" would report every real
/// artifact as unknown provenance, which is the opposite of the truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentId {
    /// A three-part version, comparable.
    Version(Version),
    /// A 40-character object id. Comparable only by equality: a revision is
    /// not ordered, and guessing an order from one is how a payload from a
    /// side branch reads as "newer".
    Revision(String),
    /// Neither — including a missing provenance block.
    Unknown(String),
}

/// Classify an artifact's `engine.engine` string.
#[must_use]
pub fn identify(found: Option<&str>) -> InstrumentId {
    let Some(found) = found else {
        return InstrumentId::Unknown("unknown".to_string());
    };
    if found.len() == 40 && found.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return InstrumentId::Revision(found.to_string());
    }
    Version::parse_first(found).map_or_else(
        || InstrumentId::Unknown(found.to_string()),
        InstrumentId::Version,
    )
}

/// Check a **stored** payload's `engine` block against this build.
///
/// `found` is the `engine.engine` string an artifact carries, or `None` for an
/// artifact written before provenance existed.
///
/// # Errors
/// [`crate::Error::EngineRevisionMismatch`] when the artifact names a different
/// engine object id — which may be older *or* newer, and is why it is not the
/// same condition as an old version. [`crate::Error::EnginePremise`] when it
/// names a version below [`MINIMUM_STORED_ENGINE`], or names nothing readable.
pub fn check_premise(subject: &str, found: Option<&str>) -> crate::Result<()> {
    let required = Version::parse_first(MINIMUM_STORED_ENGINE).unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
    });
    match identify(found) {
        InstrumentId::Version(version) if version >= required => Ok(()),
        InstrumentId::Revision(revision) if revision == ENGINE_REVISION => Ok(()),
        InstrumentId::Revision(revision) => Err(crate::Error::EngineRevisionMismatch {
            subject: subject.to_string(),
            found: revision,
            required: ENGINE_REVISION.to_string(),
        }),
        InstrumentId::Version(version) => Err(crate::Error::EnginePremise {
            subject: subject.to_string(),
            found: version.to_string(),
            required: MINIMUM_STORED_ENGINE.to_string(),
        }),
        InstrumentId::Unknown(raw) => Err(crate::Error::EnginePremise {
            subject: subject.to_string(),
            found: raw,
            required: MINIMUM_STORED_ENGINE.to_string(),
        }),
    }
}

/// Compile-time witnesses for [`CAPABILITIES`].
///
/// Each `const _` names the engine surface one token claims. The item is
/// evaluated at compile time, so an engine revision that dropped the field, the
/// type or the method fails the build here — naming the capability that went
/// missing — rather than shipping a payload that advertises it.
///
/// This is what replaces `QUIRE_CONTRACT.minimumCli`. A number said "the
/// producer is new enough" and was wrong twice; these say "the producer has
/// this exact surface" and cannot be wrong at all.
mod capability_witnesses {
    use quire_rs::CoverageReport;

    // `assurance_export.v1`
    const _: fn(
        &[u8],
        &quire_rs::AcceptedAssurancePremises,
    ) -> Result<quire_rs::AssuranceExport, quire_rs::AssuranceError> =
        quire_rs::read_assurance_export;
    // `binding_census`
    const _: fn(&CoverageReport) -> &[quire_rs::symbols::trace::BindingCensus] =
        |report| &report.binding_census;
    // `clause_sets`
    const _: fn(
        &quire_rs::ClauseSet,
        &std::collections::BTreeMap<String, String>,
    ) -> quire_rs::ClauseBindingReport = quire_rs::ClauseSet::evaluate;
    // `metrics_envelope`
    const _: fn(&CoverageReport) -> &[quire_rs::metric::Metric] = |report| &report.metrics;
    // `minted_targets`
    const _: fn(&CoverageReport) -> usize = |report| report.minted_targets.len();
    // `property_spans.safe_refusal`
    const _: fn(&quire_rs::AcClassification) -> bool = |record| {
        record.domain.is_some()
            || record
                .signals
                .iter()
                .any(|s| s.starts_with("span:refused-"))
    };
    // `specific_shaped`
    const _: fn(&quire_rs::AcClassification) -> bool = |record| record.property.is_specific();
    // `suspicions`
    const _: fn(&CoverageReport) -> usize = |report| report.suspicions.len();
    // `unmatched_tags`
    const _: fn(&CoverageReport) -> &[quire_rs::symbols::trace::UnmatchedTag] =
        |report| &report.unmatched_tags;
    // `vocabulary_coverage`
    const _: fn(&CoverageReport) -> usize = |report| report.vocabulary_coverage.len();
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: FR-097
    #[test]
    fn tc_379_020_the_pin_is_read_from_the_manifest_not_restated() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            manifest.contains(&format!("rev = \"{ENGINE_REVISION}\"")),
            "the compiled-in revision disagrees with the manifest"
        );
        assert!(
            manifest.contains(&format!("version = \"={ENGINE_VERSION}\"")),
            "the compiled-in engine version disagrees with the manifest"
        );
        assert_eq!(ENGINE_REVISION.len(), 40);
    }

    /// Trace: FR-097
    #[test]
    fn tc_379_021_capability_tokens_are_sorted_unique_and_non_empty() {
        let mut sorted = CAPABILITIES.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "duplicate capability token");
        assert_eq!(CAPABILITIES, sorted.as_slice(), "tokens must be sorted");
        assert!(CAPABILITIES.iter().all(|token| !token.is_empty()));
    }

    /// Trace: FR-097
    #[test]
    fn tc_379_022_versions_compare_numerically_not_lexically() {
        let ten = Version::parse_first("0.10.0").expect("parses");
        let nine = Version::parse_first("0.9.0").expect("parses");
        assert!(ten > nine, "0.10.0 must outrank 0.9.0");
        assert_eq!(
            Version::parse_first("quire 0.46.0\n"),
            Version::parse_first("0.46.0")
        );
        // The describe suffix an engine pin can carry.
        assert_eq!(
            Version::parse_first("0.45.0-123-g85dfe9d"),
            Version::parse_first("0.45.0")
        );
        assert_eq!(Version::parse_first("no version here"), None);
        assert_eq!(Version::parse_first("1.2"), None);
    }

    /// Trace: FR-097
    #[test]
    fn tc_379_023_a_stored_artifact_from_an_older_engine_is_refused_by_name() {
        assert!(check_premise("coverage artifact", Some(ENGINE_VERSION)).is_ok());

        let error = check_premise("coverage artifact", Some("0.21.0")).expect_err("must refuse");
        assert_eq!(error.code(), crate::ErrorCode::EnginePremise);
        assert!(error.to_string().contains("0.21.0"));

        // "no provenance at all" is its own reportable state, not a pass.
        let missing = check_premise("coverage artifact", None).expect_err("must refuse");
        assert_eq!(missing.code(), crate::ErrorCode::EnginePremise);
        assert!(missing.to_string().contains("unknown"));
    }

    /// Trace: FR-097
    #[test]
    fn tc_379_024_provenance_uses_the_key_names_the_retained_typescript_reads() {
        let value = serde_json::to_value(Provenance::current()).expect("serializes");
        let object = value.as_object().expect("object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["capabilities", "cli", "engine"]);
        assert_eq!(object["engine"], ENGINE_VERSION);
        assert!(
            object["cli"]
                .as_str()
                .expect("string")
                .contains("quoin-quire")
        );
    }
}
