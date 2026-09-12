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

use serde::{Deserialize, Serialize};

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
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::Finding;

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
