// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Raw-evidence accounting, and the definition a record may claim.
//!
//! # Why this module is in Wave 1
//!
//! `rawEvidenceFor`, `verifyRawEvidenceReferences` and
//! `assertGoverningDefinition` live in `src/measurement/intervention.ts` today
//! (lines 115-179 plus the `resolveRawPath` helper at 179-216), and they are
//! the *only* reason the operational layer imports the intervention layer. They
//! do not belong to interventions: they are the evidence store's own file
//! accounting and the plan layer's own governance check. Lifting them here cuts
//! the `operational -> intervention` edge, which is what makes the later waves
//! of Stage 6 orderable at all.
//!
//! # The path guards, kept whole
//!
//! [`RawEvidencePath::parse`] carries the lexical half of `resolveRawPath` and
//! the filesystem half stays on [`MeasurementSource::raw_evidence_file`], where
//! the filesystem is. Splitting them that way is what lets
//! [`crate::source::MemoryMeasurement`] refuse honestly instead of pretending
//! to hold a file.
//!
//! # Two representations of a retained evidence file, and only two
//!
//! A pointer at a retained file exists here at exactly **two** trust levels,
//! and `tests/tc_472_evidence_representations.rs` fails if a third appears:
//!
//! | type | trust |
//! |---|---|
//! | [`crate::common::recorded_evidence::RecordedEvidenceReference`] | **as recorded** — read by serde off a stored record, unchecked |
//! | [`RawEvidenceReference`] | **as verified** — minted by [`raw_evidence_for`] from a path that passed the lexical guards and a digest [`quoin_store`] took itself |
//!
//! quoin#468 landed a third, `RawEvidenceClaim`, holding bare `String`s for
//! the same job the recorded reference does, because quoin#469's serde-read
//! type did not exist yet. This wave is where the records are actually parsed,
//! so the claim is gone and [`verify_raw_evidence_references`] takes the
//! recorded reference directly — one fewer conversion at every intake, and one
//! fewer way to spell the same thing.

use quoin_store::RawFileSha256Digest;

use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::{LifecycleStatus, MeasurementPlan};

/// A store-root-relative path that has passed `resolveRawPath`'s lexical
/// guards.
///
/// Non-empty, relative, `/`-separated, already normalised, with no `..`
/// segment, no backslash, no `.` and no trailing separator. Holding this type
/// is the proof; nothing downstream re-checks.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct RawEvidencePath(String);

impl RawEvidencePath {
    /// Read a raw-evidence path.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::RawEvidencePathUnsafe`] for every spelling
    /// `resolveRawPath`'s first guard rejects.
    pub fn parse(value: &str) -> Result<Self, MeasurementError> {
        // `posix.normalize(path) !== path` rejects exactly the spellings that
        // carry an empty or `.` segment, so the segment scan below is the same
        // predicate without a second path library.
        let normalised = !value.is_empty()
            && !value.starts_with('/')
            && !value.contains('\\')
            && !value.ends_with('/')
            && value
                .split('/')
                .all(|segment| !segment.is_empty() && segment != "." && segment != "..");
        if normalised {
            Ok(Self(value.to_owned()))
        } else {
            Err(MeasurementError::new(
                MeasurementErrorCode::RawEvidencePathUnsafe,
                format!("unsafe relative path {value:?}"),
            ))
        }
    }

    /// The path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RawEvidencePath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One raw-evidence reference, as `rawEvidenceFor` mints it.
///
/// # Two types, not three
///
/// This crate describes a retained evidence file with **exactly two** types,
/// and `tests/tc_471_evidence_reference_types.rs` fails if a third appears.
/// They are the two trust levels the domain actually has:
///
/// | type | who makes it | what holding it proves |
/// | --- | --- | --- |
/// | `RawEvidenceReference` | [`raw_evidence_for`], from the file | the path is safe, the media type is non-empty, the digest is a sha256 the store itself took |
/// | [`RecordedEvidenceReference`] | serde, from a stored record | nothing — it is what the record *claims* |
///
/// quoin#468 landed a third, `RawEvidenceClaim`, holding bare `String`s for the
/// same claim [`RecordedEvidenceReference`] already carried. It is gone:
/// `verify_raw_evidence_references` takes the record's own field, which is what
/// the caller has anyway, and the conversion only runs upward — a minted
/// reference becomes a recorded one through [`From`], and there is no
/// conversion back, because a claim is not evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEvidenceReference {
    /// The store-root-relative path.
    pub path: RawEvidencePath,
    /// The producer's media type for the file.
    pub media_type: NonEmptyText,
    /// The file's size in bytes.
    pub size_bytes: u64,
    /// The file's sha256 digest.
    pub digest: RawFileSha256Digest,
}

impl From<RawEvidenceReference> for RecordedEvidenceReference {
    /// Widen a minted reference to the form a record stores.
    ///
    /// One direction only. Going the other way would mean minting proof from a
    /// claim, which is the whole thing `verify_raw_evidence_references` exists
    /// to refuse.
    fn from(value: RawEvidenceReference) -> Self {
        Self {
            path: crate::common::identity::EvidencePath::from_stored(value.path.as_str()),
            media_type: crate::common::identity::MediaType::from_stored(value.media_type.as_str()),
            size_bytes: value.size_bytes,
            digest: crate::common::identity::Digest::from_stored(value.digest.to_stored()),
        }
    }
}

/// Mint a raw-evidence reference for a retained file.
///
/// # Errors
///
/// [`MeasurementErrorCode::RawEvidencePathUnsafe`] for an unsafe or absent
/// path, [`MeasurementErrorCode::RawEvidenceUnavailable`] from a source that
/// holds no files, and [`MeasurementErrorCode::CollectionInvalid`] for an empty
/// media type.
pub fn raw_evidence_for<S: MeasurementSource + ?Sized>(
    source: &S,
    path: &str,
    media_type: &str,
) -> Result<RawEvidenceReference, MeasurementError> {
    let path = RawEvidencePath::parse(path)?;
    let media_type = NonEmptyText::parse(
        media_type,
        MeasurementErrorCode::CollectionInvalid,
        "media_type",
    )?;
    let file = source.raw_evidence_file(&path)?;
    Ok(RawEvidenceReference {
        path,
        media_type,
        size_bytes: file.size_bytes,
        digest: file.digest,
    })
}

/// Check every claimed reference against the file it names.
///
/// Each claim is checked independently and every disagreement is reported, in
/// the `/raw_evidence/<index>/<member>` pointer spelling of
/// `intervention.ts:162-176`.
///
/// # Errors
///
/// [`MeasurementErrorCode::RawEvidenceMismatch`], carrying one finding per
/// disagreement, when any claim does not hold.
pub fn verify_raw_evidence_references<S: MeasurementSource + ?Sized>(
    source: &S,
    references: &[RecordedEvidenceReference],
) -> Result<(), MeasurementError> {
    let mut findings = Vec::new();
    for (index, reference) in references.iter().enumerate() {
        match check(source, reference) {
            Ok(file) => {
                if file.size_bytes != reference.size_bytes {
                    findings.push(format!(
                        "/raw_evidence/{index}/size_bytes: expected {}, observed {}",
                        reference.size_bytes, file.size_bytes
                    ));
                }
                let observed = file.digest.to_stored();
                if observed != reference.digest.as_str() {
                    findings.push(format!(
                        "/raw_evidence/{index}/digest: expected {}, observed {observed}",
                        reference.digest
                    ));
                }
            }
            // The retained code catches around the whole read, so an unsafe
            // path, an absent file and an unreadable one all land on
            // `/path` as one finding rather than aborting the loop.
            Err(error) => findings.push(format!("/raw_evidence/{index}/path: {error}")),
        }
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(MeasurementError::with_findings(
            MeasurementErrorCode::RawEvidenceMismatch,
            "raw evidence does not match the retained files",
            findings,
        ))
    }
}

fn check<S: MeasurementSource + ?Sized>(
    source: &S,
    reference: &RecordedEvidenceReference,
) -> Result<crate::source::RawEvidenceFile, MeasurementError> {
    source.raw_evidence_file(&RawEvidencePath::parse(reference.path.as_str())?)
}

/// Refuse a definition version no active plan governs.
///
/// # Errors
///
/// [`MeasurementErrorCode::GoverningPlanAbsent`] when no plan is active at all,
/// and [`MeasurementErrorCode::DefinitionMismatch`] when one is but none names
/// `observed`.
pub fn assert_governing_definition(
    plans: &[MeasurementPlan],
    observed: &str,
) -> Result<(), MeasurementError> {
    let active: Vec<&MeasurementPlan> = plans
        .iter()
        .filter(|plan| plan.status == LifecycleStatus::Active)
        .collect();
    if active.is_empty() {
        return Err(MeasurementError::with_findings(
            MeasurementErrorCode::GoverningPlanAbsent,
            "no active MeasurementPlan exists",
            vec![format!(
                "requested definition {observed}; no active MeasurementPlan exists"
            )],
        ));
    }
    if active
        .iter()
        .any(|plan| plan.definition_version.as_str() == observed)
    {
        return Ok(());
    }
    let mut expected: Vec<&str> = active
        .iter()
        .map(|plan| plan.definition_version.as_str())
        .collect();
    expected.sort_unstable();
    Err(MeasurementError::with_findings(
        MeasurementErrorCode::DefinitionMismatch,
        "requested definition is governed by no active MeasurementPlan",
        vec![format!(
            "requested definition {observed}; expected one of {}; observed {observed}",
            expected.join(", ")
        )],
    ))
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{RawEvidencePath, assert_governing_definition};
    use crate::error::MeasurementErrorCode;
    use crate::types::ids::NonEmptyText;
    use crate::types::plan::{LifecycleStatus, MeasurementPlan, MeasurementStage};

    fn text(value: &str) -> NonEmptyText {
        NonEmptyText::parse(value, MeasurementErrorCode::PlanInvalid, "x").unwrap()
    }

    fn plan(status: LifecycleStatus, definition_version: &str) -> MeasurementPlan {
        MeasurementPlan {
            id: text("MP-1"),
            title: text("A plan"),
            status,
            stage: MeasurementStage::Observe,
            metric: text("m"),
            definition_version: text(definition_version),
            path: "spec/assurance/a.md".to_owned(),
            owner: None,
            action: None,
            preregistration: None,
        }
    }

    #[test]
    fn every_spelling_the_lexical_guard_rejects_stays_rejected() {
        for unsafe_path in [
            "",
            "/etc/passwd",
            "a\\b",
            "../a",
            "a/../b",
            ".",
            "./a",
            "a/",
            "a//b",
        ] {
            let error = RawEvidencePath::parse(unsafe_path).unwrap_err();
            assert_eq!(error.code(), MeasurementErrorCode::RawEvidencePathUnsafe);
        }
        assert_eq!(
            RawEvidencePath::parse("raw/2026/run.json")
                .unwrap()
                .as_str(),
            "raw/2026/run.json"
        );
    }

    #[test]
    fn no_active_plan_and_a_wrong_definition_are_different_refusals() {
        let retired = [plan(LifecycleStatus::Retired, "v1")];
        assert_eq!(
            assert_governing_definition(&retired, "v1")
                .unwrap_err()
                .code(),
            MeasurementErrorCode::GoverningPlanAbsent
        );
        let active = [plan(LifecycleStatus::Active, "v1")];
        assert!(assert_governing_definition(&active, "v1").is_ok());
        let error = assert_governing_definition(&active, "v2").unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::DefinitionMismatch);
        assert_eq!(
            error.findings(),
            ["requested definition v2; expected one of v1; observed v2"]
        );
    }
}
