// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One measurement failure, one exit status.
//!
//! The single place a `quoin-measurement` or `quoin-measurement-graph` failure
//! becomes a number on the boundary's exit taxonomy. Split from the operations
//! for the same reason `ops::evidence::taxonomy` is: the operations decide
//! *what to do*, this decides *how a failure is reported*, and keeping the
//! mapping alone in a file is what lets it be asserted against the error
//! catalogues themselves rather than against whichever variants a test double
//! happens to construct.
//!
//! # Three catalogues, one question
//!
//! The domain reports three error families — `MeasurementError`,
//! `InterventionIntakeError` and `GraphAdapterError` — and each is asked the
//! same question: **is the fix in what the caller sent, or in the repository?**
//! The caller's own document is `BadRequest` (3); a repository that declines
//! the request it understood is `Refused` (2).
//!
//! The question is asked of the CODE and not of the route, which has one
//! consequence worth stating: `CollectionInvalid` is `BadRequest` because the
//! route that raises it most is `measurement.record`, where the collection
//! arrives on stdin, and a report route that meets the same code over a
//! collection already on disk reports it as `BadRequest` too. That is the
//! cost of a per-code mapping, and it is paid in preference to a per-route
//! one: a per-route mapping is twelve mappings, and the twelfth disagrees.

use quoin_measurement::operational::github_release::GitHubReleaseError;
use quoin_measurement::{InterventionIntakeError, InterventionRefusalCode, MeasurementError};
use quoin_measurement::{MeasurementErrorCode, MeasurementErrorCode as Code};
use quoin_measurement_graph::{GraphAdapterError, GraphAdapterErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-measurement` refusal onto the boundary's exit taxonomy.
///
/// `MeasurementErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm
/// and the compiler will NOT flag a code added upstream.
/// [`tests::the_measurement_mapping_covers_every_code`] pins
/// `MeasurementErrorCode::ALL.len()`, which a loop over `ALL` could not do —
/// such a loop re-runs this same match and agrees with itself (the quoin#443
/// failure mode).
pub(super) fn map_measurement(error: &MeasurementError, op: &'static str) -> CoreError {
    let code = error.code();
    envelope(
        measurement_code(code),
        &error.to_string(),
        op,
        "measurement_code",
        code.as_str(),
    )
}

/// Map an intake refusal onto the boundary's exit taxonomy.
///
/// `InterventionRefusalCode` is a closed union with no `#[non_exhaustive]`, so
/// [`intake_code`] is an exhaustive match and the COMPILER is the guard: a
/// variant added upstream fails the build here rather than falling through.
pub(super) fn map_intake(error: &InterventionIntakeError, op: &'static str) -> CoreError {
    let code = error.code();
    envelope(
        Some(intake_code(code)),
        &error.to_string(),
        op,
        "intake_code",
        code.as_str(),
    )
}

/// Map a graph-adapter refusal onto the boundary's exit taxonomy.
///
/// `GraphAdapterErrorCode` is `#[non_exhaustive]`; see [`map_measurement`] for
/// why the count is pinned rather than looped over.
pub(super) fn map_graph(error: &GraphAdapterError, op: &'static str) -> CoreError {
    let code = error.code();
    envelope(
        graph_code(code),
        &error.to_string(),
        op,
        "graph_code",
        code.as_str(),
    )
}

/// Map a GitHub-release producer refusal onto the boundary's exit taxonomy.
///
/// The producer has two failures and they are kept apart:
/// [`GitHubReleaseError::Input`] is the definition or the retained exports
/// failing the producer's input contract — the caller's own document, so
/// `BadRequest` — and [`GitHubReleaseError::Intake`] is the store declining the
/// produced pair, which is [`map_intake`]'s question and is answered there
/// rather than a second time here.
pub(super) fn map_github_release(error: &GitHubReleaseError, op: &'static str) -> CoreError {
    match error {
        GitHubReleaseError::Input(detail) => {
            CoreError::new(CoreErrorCode::BadRequest, detail.clone())
                .with_context("op", op)
                .with_context("producer", "github_release")
        }
        GitHubReleaseError::Intake(intake) => map_intake(intake, op),
    }
}

/// The one envelope shape all four mappings raise.
fn envelope(
    mapped: Option<CoreErrorCode>,
    message: &str,
    op: &'static str,
    key: &'static str,
    code: &str,
) -> CoreError {
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), message.to_owned())
        .with_context("op", op)
        .with_context(key, code.to_owned());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `MeasurementErrorCode` maps to, or `None` for a
/// code this build does not know.
const fn measurement_code(code: MeasurementErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        // The caller's own document: a candidate that does not satisfy the
        // stored envelope, an id that does not name a file, a definition
        // version no active plan governs, an unsafe raw-evidence reference or
        // artifact name, a value that is not a date-time, or an observation
        // whose population is below, or does not state, its plan's minimum or
        // repetition count.
        Code::CollectionInvalid
        | Code::CollectionIdUnsafe
        | Code::DefinitionMismatch
        | Code::RawEvidencePathUnsafe
        | Code::ArtifactNameUnsafe
        | Code::DateTimeInvalid
        | Code::PopulationBelowMinimum
        | Code::PopulationUnstated
        | Code::RepetitionsShort => CoreErrorCode::BadRequest,

        // The repository declined: retained bytes that differ, a document that
        // cannot be read back, a plan or profile that is present and
        // unacceptable, no active plan at all, a raw-evidence file that
        // disagrees with its digest or is absent, a named artifact the
        // repository holds in a form that cannot be digested, unreadable YAML,
        // and every filesystem and store refusal. Fixing any of these means
        // changing the repository, not the request.
        Code::CollectionIdCollision
        | Code::CollectionUnreadable
        | Code::RecordUnreadable
        | Code::PlanInvalid
        | Code::ProfileInvalid
        | Code::GoverningPlanAbsent
        | Code::RawEvidenceMismatch
        | Code::RawEvidenceUnavailable
        | Code::ArtifactUnreadable
        | Code::Yaml
        | Code::Io
        | Code::Store => CoreErrorCode::Refused,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on.
        _ => return None,
    })
}

/// The exit-taxonomy code one `InterventionRefusalCode` maps to.
///
/// Total, with no `_` arm: the union is closed, so a variant added upstream is
/// a compile error here.
const fn intake_code(code: InterventionRefusalCode) -> CoreErrorCode {
    match code {
        // The record itself, or the definition version it names.
        InterventionRefusalCode::InvalidRecord | InterventionRefusalCode::DefinitionMismatch => {
            CoreErrorCode::BadRequest
        }
        // The store declined: a raw-evidence file that disagrees, no governing
        // plan, another intake holding the lock, or an identity already taken.
        InterventionRefusalCode::RawEvidenceMismatch
        | InterventionRefusalCode::GoverningPlanAbsent
        | InterventionRefusalCode::IntakeBusy
        | InterventionRefusalCode::RecordIdCollision => CoreErrorCode::Refused,
    }
}

/// The exit-taxonomy code one `GraphAdapterErrorCode` maps to, or `None` for a
/// code this build does not know.
///
/// The split here is narrower than [`measurement_code`]'s on purpose. A graph
/// portfolio request carries repository roots and `<repository>=<value>`
/// mappings and NOTHING ELSE — every export, premise, attestation and
/// observation is read off disk — so only the mapping vocabulary can be the
/// caller's mistake. A malformed export is the repository's.
const fn graph_code(code: GraphAdapterErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        GraphAdapterErrorCode::UnknownAdapter
        | GraphAdapterErrorCode::InvalidRepositoryMapping
        | GraphAdapterErrorCode::DuplicateGraphExport
        | GraphAdapterErrorCode::DuplicateGraphPremises
        | GraphAdapterErrorCode::DuplicateGraphAudit => CoreErrorCode::BadRequest,

        GraphAdapterErrorCode::InvalidPremise
        | GraphAdapterErrorCode::InvalidObservation
        | GraphAdapterErrorCode::InvalidAttestation
        | GraphAdapterErrorCode::AttachmentMissing
        | GraphAdapterErrorCode::AttachmentDigestMismatch
        | GraphAdapterErrorCode::InactivePlan
        | GraphAdapterErrorCode::DuplicatePartition
        | GraphAdapterErrorCode::AttachmentEncodingInvalid
        | GraphAdapterErrorCode::CollectionRefused
        | GraphAdapterErrorCode::Measurement
        | GraphAdapterErrorCode::Store => CoreErrorCode::Refused,

        _ => return None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        Code, CoreErrorCode, GitHubReleaseError, GraphAdapterError, GraphAdapterErrorCode,
        InterventionIntakeError, InterventionRefusalCode, MeasurementError, MeasurementErrorCode,
        graph_code, intake_code, map_github_release, map_graph, map_intake, map_measurement,
        measurement_code,
    };

    /// `MeasurementErrorCode` is `#[non_exhaustive]`, so `measurement_code`'s
    /// `_` arm means the compiler cannot catch a code added upstream. This pins
    /// the COUNT instead, and lists every group by name.
    ///
    /// When this fails: a code was added to `quoin-measurement`. Decide which
    /// group it belongs to, add it to that arm, and raise the count.
    ///
    /// Trace: FR-101-AC-7
    /// Provenance: quoin#478
    #[test]
    fn the_measurement_mapping_covers_every_code() {
        assert_eq!(
            MeasurementErrorCode::ALL.len(),
            21,
            "quoin-measurement gained or lost an error code; map it deliberately"
        );

        let bad_request = [
            Code::CollectionInvalid,
            Code::CollectionIdUnsafe,
            Code::DefinitionMismatch,
            Code::RawEvidencePathUnsafe,
            Code::ArtifactNameUnsafe,
            Code::DateTimeInvalid,
            Code::PopulationBelowMinimum,
            Code::PopulationUnstated,
            Code::RepetitionsShort,
        ];
        let refused = [
            Code::CollectionIdCollision,
            Code::CollectionUnreadable,
            Code::RecordUnreadable,
            Code::PlanInvalid,
            Code::ProfileInvalid,
            Code::GoverningPlanAbsent,
            Code::RawEvidenceMismatch,
            Code::RawEvidenceUnavailable,
            Code::ArtifactUnreadable,
            Code::Yaml,
            Code::Io,
            Code::Store,
        ];
        assert_eq!(
            bad_request.len() + refused.len(),
            MeasurementErrorCode::ALL.len(),
            "a code is in `ALL` but in neither of the two groups"
        );
        for code in bad_request {
            assert_eq!(
                measurement_code(code),
                Some(CoreErrorCode::BadRequest),
                "{code}"
            );
        }
        for code in refused {
            assert_eq!(
                measurement_code(code),
                Some(CoreErrorCode::Refused),
                "{code}"
            );
        }

        let envelope = map_measurement(
            &MeasurementError::new(Code::CollectionIdUnsafe, "a/b"),
            "measurement.record",
        );
        assert_eq!(envelope.code, CoreErrorCode::BadRequest);
        assert_eq!(envelope.outcome().code(), 3);
        assert_eq!(
            envelope.context["measurement_code"],
            Code::CollectionIdUnsafe.as_str()
        );
        assert_eq!(envelope.context["op"], "measurement.record");
    }

    /// The intake union is closed, so the mapping is exhaustive and the
    /// compiler guards it. The count is still asserted, because a variant
    /// REMOVED upstream would silently shrink both the match and this list.
    ///
    /// Trace: FR-101-AC-7
    /// Provenance: quoin#478
    #[test]
    fn the_intake_mapping_covers_every_refusal_code() {
        assert_eq!(
            InterventionRefusalCode::all().len(),
            6,
            "quoin-measurement gained or lost an intake refusal code"
        );

        let bad_request = [
            InterventionRefusalCode::InvalidRecord,
            InterventionRefusalCode::DefinitionMismatch,
        ];
        let refused = [
            InterventionRefusalCode::RawEvidenceMismatch,
            InterventionRefusalCode::GoverningPlanAbsent,
            InterventionRefusalCode::IntakeBusy,
            InterventionRefusalCode::RecordIdCollision,
        ];
        assert_eq!(
            bad_request.len() + refused.len(),
            InterventionRefusalCode::all().len(),
            "a refusal code is in `all()` but in neither of the two groups"
        );
        for code in bad_request {
            assert_eq!(intake_code(code), CoreErrorCode::BadRequest, "{code}");
        }
        for code in refused {
            assert_eq!(intake_code(code), CoreErrorCode::Refused, "{code}");
        }

        let envelope = map_intake(
            &InterventionIntakeError::new(
                InterventionRefusalCode::IntakeBusy,
                vec!["another intake holds the store".to_owned()],
            ),
            "measurement.record",
        );
        assert_eq!(envelope.code, CoreErrorCode::Refused);
        assert_eq!(envelope.outcome().code(), 2);
        assert_eq!(envelope.context["intake_code"], "intake_busy");
    }

    /// `GraphAdapterErrorCode` is `#[non_exhaustive]`; the count is the guard.
    ///
    /// Trace: FR-101-AC-7
    /// Provenance: quoin#478
    #[test]
    fn the_graph_mapping_covers_every_code() {
        assert_eq!(
            GraphAdapterErrorCode::ALL.len(),
            16,
            "quoin-measurement-graph gained or lost an error code; map it deliberately"
        );

        let bad_request = [
            GraphAdapterErrorCode::UnknownAdapter,
            GraphAdapterErrorCode::InvalidRepositoryMapping,
            GraphAdapterErrorCode::DuplicateGraphExport,
            GraphAdapterErrorCode::DuplicateGraphPremises,
            GraphAdapterErrorCode::DuplicateGraphAudit,
        ];
        let refused = [
            GraphAdapterErrorCode::InvalidPremise,
            GraphAdapterErrorCode::InvalidObservation,
            GraphAdapterErrorCode::InvalidAttestation,
            GraphAdapterErrorCode::AttachmentMissing,
            GraphAdapterErrorCode::AttachmentDigestMismatch,
            GraphAdapterErrorCode::InactivePlan,
            GraphAdapterErrorCode::DuplicatePartition,
            GraphAdapterErrorCode::AttachmentEncodingInvalid,
            GraphAdapterErrorCode::CollectionRefused,
            GraphAdapterErrorCode::Measurement,
            GraphAdapterErrorCode::Store,
        ];
        assert_eq!(
            bad_request.len() + refused.len(),
            GraphAdapterErrorCode::ALL.len(),
            "a code is in `ALL` but in neither of the two groups"
        );
        for code in bad_request {
            assert_eq!(graph_code(code), Some(CoreErrorCode::BadRequest), "{code}");
        }
        for code in refused {
            assert_eq!(graph_code(code), Some(CoreErrorCode::Refused), "{code}");
        }

        let envelope = map_graph(
            &GraphAdapterError::new(
                GraphAdapterErrorCode::InvalidRepositoryMapping,
                "alpha=: no path",
            ),
            "measurement.build_graph_portfolio",
        );
        assert_eq!(envelope.code, CoreErrorCode::BadRequest);
        assert_eq!(
            envelope.context["graph_code"],
            GraphAdapterErrorCode::InvalidRepositoryMapping.as_str()
        );
    }

    /// The producer's two failures do not collapse into one status.
    ///
    /// Trace: FR-101-AC-7
    /// Provenance: quoin#478
    #[test]
    fn the_release_producer_keeps_its_two_failures_apart() {
        let op = "measurement.produce_github_release_operational";
        let input = map_github_release(
            &GitHubReleaseError::Input("workflow_path does not match the run".to_owned()),
            op,
        );
        assert_eq!(input.code, CoreErrorCode::BadRequest);
        assert_eq!(input.context["producer"], "github_release");

        let intake = map_github_release(
            &GitHubReleaseError::Intake(InterventionIntakeError::new(
                InterventionRefusalCode::RecordIdCollision,
                vec!["r-1".to_owned()],
            )),
            op,
        );
        assert_eq!(intake.code, CoreErrorCode::Refused);
        assert_eq!(intake.context["intake_code"], "record_id_collision");
        assert!(
            !intake.context.contains_key("producer"),
            "an intake refusal is the store's, not the producer's input contract"
        );
    }
}
