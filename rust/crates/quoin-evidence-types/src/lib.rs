// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Producer-reliance and independence assessments, as the assurance renderer
//! observes them (quoin#384).
//!
//! # This crate exists because an import list lied in the other direction
//!
//! quoin#425 found that sizing a crate from the import graph **overstates**:
//! `build_case` names five types and reads twelve fields, and two of the five
//! it never inspects at all — it sorts them by one key and re-emits them
//! unchanged. Those two were correctly carried as opaque values.
//!
//! `render_case` is the same instrument failing the other way. It imports one
//! line:
//!
//! ```ignore
//! import type { AssuranceCase, CaseNode } from "./graph.js";
//! ```
//!
//! Both of those already existed, so by the import graph `render_case` was
//! free. It reads **fourteen fields across three types it never names**,
//! reaching through `AssuranceCase`'s own fields:
//!
//! | type | `build_case` | `render_case` |
//! |---|---|---|
//! | [`TrustAssessment`] | one sort key | 5 of 8 fields |
//! | [`IndependenceAssessment`] | one sort key | 6 of 7 fields |
//! | [`IndependenceDimensionAssessment`] | never reached | 3 of 3 fields |
//!
//! Every one of those is interpolated into the rendered markdown, so every one
//! is observable in a byte comparison — which is exactly the test that decides
//! whether a type is real or opaque. The opaque-value decision that was right
//! for `build_case` is wrong here, and the same type lands in different classes
//! for different operations. That is the part that is easy to miss.
//!
//! # These are readers, restricted to what the renderer emits
//!
//! `src/evidence/types.ts` owns the full shapes. Only the fields the renderer
//! puts on the page are declared here, for the reason above: a field nothing
//! observes cannot be covered by anything, so declaring it would add surface a
//! byte comparison could not check.
//!
//! There is no `deny_unknown_fields` anywhere in this crate. The caller hands
//! the renderer whatever `build_case` emitted, which carries every field the
//! evidence store wrote.

use serde::{Deserialize, Serialize};

/// Use-specific producer reliance, shown as context and never as claim support.
///
/// Reads 5 of the 8 fields `src/evidence/types.ts` declares. `producer`,
/// `permittedDecisions` and `owner` are not rendered, so they are not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustAssessment {
    /// The assessment's own id. Also `build_case`'s sort key.
    pub id: String,
    /// The bounded use this reliance decision was made for.
    pub use_id: String,
    /// The decision, interpolated verbatim.
    ///
    /// A `String` and not an enum, for the reason set out on
    /// [`quoin_finding_types::Finding::kind`]: the retained union is erased at
    /// run time and the renderer interpolates whatever arrives, so a closed
    /// Rust enum would refuse input the retained implementation accepts.
    ///
    /// The five the store writes today, as a reader's note and not a
    /// constraint: `accepted`, `accepted-with-limitations`, `invalidated`,
    /// `not-accepted`, `unobserved`.
    pub status: String,
    /// What would require this reliance to be revalidated.
    ///
    /// A list of strings rather than of objects: `TrustTrigger` is a string
    /// union in the retained source, and the renderer joins it with `", "`.
    /// Had it been an object, that join would produce `[object Object]` — a
    /// real defect, which is why this was worth checking rather than assuming.
    pub triggered_by: Vec<String>,
    /// Stated limitations, joined with `"; "` when non-empty.
    pub limitations: Vec<String>,
}

/// One dimension of a profile-selected separation check.
///
/// All three fields are rendered, so all three are here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndependenceDimensionAssessment {
    /// Which dimension was checked, interpolated verbatim.
    pub dimension: String,
    /// The values observed. Rendered as `no stated values` when empty.
    pub values: Vec<String>,
    /// Suites the dimension could not be read from.
    pub missing_suites: Vec<String>,
}

/// A profile-selected separation result for one obligation.
///
/// Reads 6 of the 7 fields the retained type declares; `satisfiedBy` is the one
/// the renderer does not put on the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndependenceAssessment {
    /// The profile that selected the check.
    pub profile: String,
    /// The requirement the obligation belongs to.
    pub requirement: String,
    /// The obligation assessed. Also `build_case`'s sort key.
    pub obligation: String,
    /// `satisfied` or `insufficient` today; a `String` for the same reason as
    /// [`TrustAssessment::status`].
    pub status: String,
    /// One line explaining the result.
    pub summary: String,
    /// The per-dimension detail.
    pub dimensions: Vec<IndependenceDimensionAssessment>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{IndependenceAssessment, TrustAssessment};

    #[test]
    fn reads_a_trust_assessment_carrying_the_three_unrendered_fields() {
        // The shape the evidence store actually writes. If this fails, the
        // renderer has started refusing its own producer's output.
        let assessment: TrustAssessment = serde_json::from_str(
            r#"{
                "id": "T-001",
                "useId": "U-1",
                "producer": "vitest",
                "status": "accepted-with-limitations",
                "permittedDecisions": ["release"],
                "limitations": ["linux only"],
                "triggeredBy": ["producer-version", "configuration"],
                "owner": "qa"
            }"#,
        )
        .expect("the store's own trust shape must deserialize");
        assert_eq!(assessment.use_id, "U-1");
        assert_eq!(
            assessment.triggered_by,
            ["producer-version", "configuration"]
        );
    }

    #[test]
    fn reads_an_independence_assessment_ignoring_satisfied_by() {
        let assessment: IndependenceAssessment = serde_json::from_str(
            r#"{
                "profile": "P",
                "requirement": "FR-001",
                "obligation": "FR-001-AC-1",
                "status": "satisfied",
                "dimensions": [
                    {"dimension": "author", "values": ["a", "b"], "missingSuites": []}
                ],
                "satisfiedBy": ["unit", "integration"],
                "summary": "two authors"
            }"#,
        )
        .expect("the store's own independence shape must deserialize");
        assert_eq!(assessment.dimensions.len(), 1);
        assert_eq!(assessment.dimensions[0].missing_suites, [] as [String; 0]);
    }

    #[test]
    fn accepts_a_status_no_closed_enum_would_admit() {
        // Same property as `Finding::kind`. The renderer interpolates; it does
        // not validate, and neither may this.
        let assessment: TrustAssessment = serde_json::from_str(
            r#"{"id":"T","useId":"U","status":"status-invented-tomorrow","triggeredBy":[],"limitations":[]}"#,
        )
        .expect("an unknown status is not a parse error");
        assert_eq!(assessment.status, "status-invented-tomorrow");
    }

    #[test]
    fn a_missing_rendered_field_is_a_hard_error() {
        // Every field here is rendered unconditionally, so absence must fail
        // loudly rather than default to empty and print a blank row.
        for text in [
            r#"{"useId":"U","status":"s","triggeredBy":[],"limitations":[]}"#,
            r#"{"id":"T","status":"s","triggeredBy":[],"limitations":[]}"#,
            r#"{"id":"T","useId":"U","triggeredBy":[],"limitations":[]}"#,
            r#"{"id":"T","useId":"U","status":"s","limitations":[]}"#,
            r#"{"id":"T","useId":"U","status":"s","triggeredBy":[]}"#,
        ] {
            assert!(
                serde_json::from_str::<TrustAssessment>(text).is_err(),
                "a trust assessment missing a rendered field must not deserialize: {text}"
            );
        }
    }
}
