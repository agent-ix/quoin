// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The auditor's [`Finding`], as the assurance view observes it (quoin#384).
//!
//! # Why this crate holds one type and not the auditor's four
//!
//! quoin#425 sized the remaining burn-down stages from the import graph. That
//! is the wrong instrument, and this crate is the first place it shows. The
//! retained `src/assurance/graph.ts` imports five types across four
//! directories, so the import graph says "four crates". The **field subset**
//! `buildCase` can actually observe says something else: twelve fields across
//! five types, and two of the five never read at all.
//!
//! `Finding` is one of the two that is genuinely read, so it gets a real type.
//! The auditor's other three types are not on `build_case`'s path, and putting
//! them here because they share a directory with this one would be sizing from
//! the module boundary again — the same mistake in the opposite direction.
//!
//! # What quoin#385 added, and why it did not add a fourth crate
//!
//! `src/graph-analysis/` is a **second** view of the auditor's output, and it
//! reads more than `build_case` does: [`AuditReport`] as a whole, and every
//! [`UnevaluatedCheck`] in it. quoin#385 recorded the ruling once — those
//! types live here, beside [`Finding`], rather than in the view crate that
//! happens to need them first. Two homes for one producer's shape is how two
//! readers end up disagreeing about it.
//!
//! # `severity` is declared by neither reader, and that is the boundary
//!
//! `src/auditor/audit.ts:34` types it `"low" | "medium" | "high"`, and the
//! graph view's input contract refuses a fourth spelling
//! (`src/graph-analysis/input.ts:52`). Neither reader *uses* the value:
//! `build_case` renders `kind` and `summary`, and the graph view copies the
//! finding out again unchanged. So `severity` is carried in `other` like every
//! other undeclared member, and the closed vocabulary is stated once, at the
//! boundary that states it — `quoin-graph-analysis`'s audit contract.
//!
//! Declaring it here as a closed enum instead would make this crate refuse a
//! finding `build_case` has always accepted: TypeScript's types are not
//! runtime checks, and `quoin-assurance`'s captured corpus carries a
//! `severity` of `"error"` that the retained implementation read without
//! complaint. A reader stricter than the one it replaces is a port defect.
//!
//! # Members nobody here reads are preserved, not declared
//!
//! The graph view copies the auditor's findings and unevaluated checks
//! **verbatim** into its own report — `src/graph-analysis/input.ts` validates
//! the join surface with a zod `.passthrough()` and preserves the rest. A type
//! that dropped the other members would change those bytes. So every type in
//! this crate carries a flattened `other` map: the members are carried, and
//! still not declared. The original argument stands — a *declared* field
//! nothing observes is untested surface — and it is a different thing from a
//! member kept because its producer wrote it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Members the producer wrote that no reader here declares.
///
/// A `BTreeMap` rather than a `serde_json::Map`: the map is only ever written
/// back out through a canonical serializer, so a stable order here costs
/// nothing and makes equality on these types independent of input order.
pub type OtherMembers = BTreeMap<String, Value>;

/// One auditor finding, restricted to the fields the assurance view reads.
///
/// # The subset is deliberate, and it is three of twelve
///
/// The retained `Finding` (`src/auditor/audit.ts`) carries twelve fields.
/// `buildCase` reads exactly three: it keys findings by `obligation`, and
/// renders `because` as `` `{kind}: {summary}` ``. It never reads `severity`,
/// `path`, `line`, `symbol` or any of the structured action fields — not even
/// to order by, because the auditor has already ordered its output and the
/// view takes the first finding per obligation on that strength.
///
/// Declaring the other nine here would put fields in the payload that no
/// behaviour depends on, and a difftest comparing canonical JSON bytes could
/// not tell a correct one from an incorrect one. A field nothing observes is
/// not covered by anything.
///
/// # Unknown fields are accepted, and must be
///
/// There is no `deny_unknown_fields`. The real auditor emits all twelve, and
/// the caller hands this view the auditor's own output verbatim. Refusing the
/// nine we do not read would refuse every real input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// The obligation the finding is against. The view's join key.
    pub obligation: String,
    /// What kind of finding it is.
    ///
    /// # Why this is a `String` and not an enum
    ///
    /// The retained type declares twelve spellings in a closed union, and it
    /// is tempting to mirror that here — the spellings *are* observable,
    /// because `because` interpolates this value into the payload.
    ///
    /// But a TypeScript union is erased at run time. `buildCase` performs no
    /// validation: it interpolates whatever string arrives. A closed Rust enum
    /// would therefore **refuse input the retained implementation accepts**,
    /// turning a payload into a `BadRequest` — a behavioural difference, in
    /// the direction of the port being stricter than the thing it replaces.
    ///
    /// This is the same defect class as the suffix enumeration in
    /// [`quoin_assurance::requirement_of`]: writing the type from what the
    /// declaration *says* rather than from what the code *does*. There the
    /// enumeration dropped every `-M-` obligation; here it would drop every
    /// finding kind added after this file was written. The difftest case
    /// `assurance/build-case-unknown-finding-kind` holds the property.
    ///
    /// The twelve the auditor mints today, for the reader's benefit and not as
    /// a constraint: `suspect-link`, `stale-evidence`, `vacuous-evidence`,
    /// `combinatorial-gap`, `undischarged`, `method-conformance`,
    /// `unknown-method`, `insufficient-independence`,
    /// `insufficient-multiplicity`, `insufficient-mutation-score`,
    /// `unmeasured-mutation-score`, `mocked-confirmation`.
    pub kind: String,
    /// The finding's one-line summary, rendered verbatim into `because`.
    pub summary: String,
    /// Every other member the auditor wrote, carried unchanged.
    #[serde(flatten)]
    pub other: OtherMembers,
}

/// One check the auditor could not run, as a view observes it.
///
/// Separate from both findings and clean results: `src/auditor/audit.ts:146`
/// exists because "the check did not run" and "the check passed" are different
/// answers, and folding them together is how an unrun check reads as healthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnevaluatedCheck {
    /// Which check did not run.
    ///
    /// A `String`, for [`Finding::kind`]'s reason: the retained type declares
    /// one spelling, `mocked-confirmation`, and the graph view's input
    /// contract accepts any non-empty string.
    pub check: String,
    /// The obligation it would have been run against. The view's join key.
    pub obligation: String,
    /// The suites it would have covered.
    pub suites: Vec<String>,
    /// Why it could not run.
    pub reason: String,
    /// Every other member the auditor wrote, carried unchanged.
    #[serde(flatten)]
    pub other: OtherMembers,
}

/// The audit result, as a view observes it.
///
/// The three collections are the whole join surface. `independence` — present
/// only when a profile supplied requirements — is one of the members carried
/// in [`AuditReport::other`] rather than declared: no view reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    /// Everything wrong with the evidence.
    pub findings: Vec<Finding>,
    /// Obligations with a binding whose hash still matches.
    pub healthy: Vec<String>,
    /// Checks that could not run.
    pub unevaluated: Vec<UnevaluatedCheck>,
    /// Every other member the auditor wrote, carried unchanged.
    #[serde(flatten)]
    pub other: OtherMembers,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{AuditReport, Finding};

    #[test]
    fn reads_a_finding_carrying_the_auditor_s_other_nine_fields() {
        // The shape the real auditor emits. If this ever fails, the view has
        // started refusing its own producer's output.
        let finding: Finding = serde_json::from_str(
            r#"{
                "kind": "undischarged",
                "obligation": "FR-001-AC-1",
                "severity": "error",
                "summary": "no evidence binds this criterion",
                "path": "src/cli.ts",
                "line": 42,
                "symbol": "parseArgs",
                "subject": "the criterion",
                "changeTarget": "tests/cli.test.ts",
                "remedy": "bind a test",
                "nextDiagnosticStep": "run the suite"
            }"#,
        )
        .expect("the auditor's own finding shape must deserialize");
        assert_eq!(finding.obligation, "FR-001-AC-1");
        assert_eq!(finding.kind, "undischarged");
        assert_eq!(finding.summary, "no evidence binds this criterion");
        // Every member the graph view copies verbatim, severity included.
        assert_eq!(finding.other.len(), 8, "{:?}", finding.other);
    }

    /// A finding round-trips through JSON with every member the producer
    /// wrote, because `src/graph-analysis/` renders the whole finding into its
    /// own report and a dropped member is a changed byte.
    #[test]
    fn carries_undeclared_members_back_out_again() {
        let source = r#"{"obligation":"FR-1","kind":"k","summary":"s","severity":"low","line":42,"deep":{"b":[1,null,true],"a":"x"}}"#;
        let finding: Finding = serde_json::from_str(source).expect("reads");
        let back = serde_json::to_value(&finding).expect("writes");
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(source).expect("reads"),
            "a member the auditor wrote must survive the round trip"
        );
    }

    /// The report and its unevaluated checks carry their undeclared members
    /// too — `independence` is the one the real auditor writes.
    #[test]
    fn a_report_carries_independence_without_declaring_it() {
        let report: AuditReport = serde_json::from_str(
            r#"{"findings":[],"healthy":["FR-1-AC-1"],"unevaluated":[{"check":"mocked-confirmation","obligation":"FR-1-AC-1","suites":["unit"],"reason":"r","inspectedAt":null}],"independence":[{"obligation":"FR-1-AC-1"}]}"#,
        )
        .expect("the auditor's own report shape must deserialize");
        assert!(report.other.contains_key("independence"));
        assert_eq!(report.unevaluated[0].other.len(), 1);
        assert_eq!(report.unevaluated[0].check, "mocked-confirmation");
    }

    #[test]
    fn accepts_a_kind_no_closed_enum_would_admit() {
        // The property the doc comment above argues for. A thirteenth kind is
        // not this view's business to reject; it renders it and moves on.
        let finding: Finding = serde_json::from_str(
            r#"{"kind":"kind-invented-tomorrow","obligation":"FR-002-AC-1","summary":"s"}"#,
        )
        .expect("an unknown kind is not a parse error");
        assert_eq!(finding.kind, "kind-invented-tomorrow");
    }

    #[test]
    fn a_missing_read_field_is_a_hard_error() {
        // All three are read unconditionally, so absence must fail loudly
        // rather than default to empty and render a blank `because`.
        for text in [
            r#"{"kind":"undischarged","summary":"s"}"#,
            r#"{"obligation":"FR-001-AC-1","summary":"s"}"#,
            r#"{"obligation":"FR-001-AC-1","kind":"undischarged"}"#,
        ] {
            assert!(
                serde_json::from_str::<Finding>(text).is_err(),
                "a finding missing a field the view reads must not deserialize: {text}"
            );
        }
    }
}
