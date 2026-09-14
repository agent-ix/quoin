// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One auditor finding (quoin#384, widened for quoin#383).

use serde::{Deserialize, Serialize};

use crate::OtherMembers;
use crate::severity::Severity;

/// One thing wrong with the evidence for one obligation.
///
/// # Three fields are read by the assurance view; twelve are written
///
/// This type began (quoin#384) as the assurance view's *reader*: `build_case`
/// keys findings by `obligation` and renders `` `{kind}: {summary}` ``, and
/// nothing else. quoin#383 ports the producer into the same workspace, and the
/// producer writes all twelve. One home, not two — a second `Finding` would be
/// two declarations of one wire shape, and the pair would drift.
///
/// The nine that only the producer writes are [`Option`] with
/// `skip_serializing_if`, so a finding the view constructs still serialises to
/// the same three keys it always did. The retained source spreads
/// `...(located.path ? { path: located.path } : {})` at every construction
/// site; a `null` on the wire would be a byte the store has never held.
///
/// # Unknown fields are accepted, and must be
///
/// There is no `deny_unknown_fields`. The caller hands this type the auditor's
/// own output verbatim, and a future field is not a parse error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
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
    /// `quoin_assurance::requirement_of`: writing the type from what the
    /// declaration *says* rather than from what the code *does*. There the
    /// enumeration dropped every `-M-` obligation; here it would drop every
    /// finding kind added after this file was written. The difftest case
    /// `assurance/build-case-unknown-finding-kind` holds the property.
    ///
    /// The twelve the auditor mints today are named by
    /// [`FindingKind`](crate::FindingKind), for the reader's benefit and not
    /// as a constraint.
    pub kind: String,
    /// The obligation the finding is against. The view's join key.
    pub obligation: String,
    /// How serious it is, when the producer said.
    ///
    /// Optional because the assurance corpus holds findings without one: four
    /// of the six findings in `quoin-assurance/tests/golden/cases.json` omit
    /// the key entirely, and the view has always rendered them. The auditor
    /// itself always writes one — [`Finding::new`] takes it by value, so a
    /// finding minted through the constructor cannot lack it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    /// The finding's one-line summary, rendered verbatim into `because`.
    pub summary: String,
    /// Repo-relative source locus, when the producer names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The line within [`Finding::path`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// The symbol the finding is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// What the structured action is about.
    ///
    /// The JSON finding contract requires a target and exactly one remedy or
    /// safe diagnostic step whenever this is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Where to make the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_target: Option<String>,
    /// The fix, when one is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
    /// The safe next step, when no fix is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_diagnostic_step: Option<String>,
    /// Every member the producer wrote that this type does not declare.
    ///
    /// Empty for every finding the auditor mints today — it writes exactly the
    /// twelve above. The map is here because `src/graph-analysis/` copies a
    /// finding it was handed **verbatim** into its own report, so a thirteenth
    /// member added upstream must survive the round trip rather than vanish.
    #[serde(flatten)]
    pub other: OtherMembers,
}

impl Finding {
    /// The four fields every auditor finding carries.
    ///
    /// Severity is taken **by value**, not as an [`Option`]: the retained
    /// auditor sets it at all twelve construction sites, and a producer path
    /// that could omit it would be a difference this port introduced.
    pub fn new(
        kind: impl Into<String>,
        obligation: impl Into<String>,
        severity: Severity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            obligation: obligation.into(),
            severity: Some(severity),
            summary: summary.into(),
            path: None,
            line: None,
            symbol: None,
            subject: None,
            change_target: None,
            remedy: None,
            next_diagnostic_step: None,
            other: OtherMembers::new(),
        }
    }

    /// The baseline key for one finding: `` `{kind}:{obligation}` ``.
    ///
    /// Here rather than in the auditor because the ratchet baseline on disk is
    /// a list of these strings, and a reader that wanted to ask "is this
    /// finding accepted?" would otherwise re-spell the key.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.obligation)
    }
}

/// The twelve kinds the auditor mints, as a reader's note.
///
/// Not a type [`Finding::kind`] is constrained to — see the doc there. This is
/// a list, and its only job is to let the producer spell each kind once.
pub struct FindingKind;

impl FindingKind {
    /// The statement changed after the evidence was bound.
    pub const SUSPECT_LINK: &'static str = "suspect-link";
    /// The evidence is missing, or older than HEAD.
    pub const STALE_EVIDENCE: &'static str = "stale-evidence";
    /// The evidence exists and verified nothing.
    pub const VACUOUS_EVIDENCE: &'static str = "vacuous-evidence";
    /// A declared configuration space is not covered.
    pub const COMBINATORIAL_GAP: &'static str = "combinatorial-gap";
    /// Nothing is bound to the obligation.
    pub const UNDISCHARGED: &'static str = "undischarged";
    /// The evidence is of a kind the declared method does not produce.
    pub const METHOD_CONFORMANCE: &'static str = "method-conformance";
    /// The declared method is in no catalog.
    pub const UNKNOWN_METHOD: &'static str = "unknown-method";
    /// A profile-selected separation requirement was not met.
    pub const INSUFFICIENT_INDEPENDENCE: &'static str = "insufficient-independence";
    /// Criticality demands two independent methods and one was found.
    pub const INSUFFICIENT_MULTIPLICITY: &'static str = "insufficient-multiplicity";
    /// The measured mutation score is below the demanded floor.
    pub const INSUFFICIENT_MUTATION_SCORE: &'static str = "insufficient-mutation-score";
    /// A mutation floor is demanded and nothing measured one.
    pub const UNMEASURED_MUTATION_SCORE: &'static str = "unmeasured-mutation-score";
    /// Every test discharging the obligation injects what it verifies.
    pub const MOCKED_CONFIRMATION: &'static str = "mocked-confirmation";

    /// All twelve, in the order `src/auditor/audit.ts:38-50` declares them.
    pub const ALL: [&'static str; 12] = [
        Self::SUSPECT_LINK,
        Self::STALE_EVIDENCE,
        Self::VACUOUS_EVIDENCE,
        Self::COMBINATORIAL_GAP,
        Self::UNDISCHARGED,
        Self::METHOD_CONFORMANCE,
        Self::UNKNOWN_METHOD,
        Self::INSUFFICIENT_INDEPENDENCE,
        Self::INSUFFICIENT_MULTIPLICITY,
        Self::INSUFFICIENT_MUTATION_SCORE,
        Self::UNMEASURED_MUTATION_SCORE,
        Self::MOCKED_CONFIRMATION,
    ];
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Finding, FindingKind};
    use crate::severity::Severity;

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
        assert_eq!(
            finding.severity.as_ref().map(Severity::as_str),
            Some("error")
        );
        assert_eq!(finding.change_target.as_deref(), Some("tests/cli.test.ts"));
        assert_eq!(
            finding.next_diagnostic_step.as_deref(),
            Some("run the suite")
        );
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
        assert!(finding.severity.is_none());
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

    #[test]
    fn a_minted_finding_writes_four_keys_and_never_a_null() {
        // `...(x ? {x} : {})` in the retained source, at every site.
        let finding = Finding::new("undischarged", "FR-001-AC-1", Severity::medium(), "s");
        assert_eq!(
            serde_json::to_string(&finding).unwrap(),
            r#"{"kind":"undischarged","obligation":"FR-001-AC-1","severity":"medium","summary":"s"}"#
        );
    }

    #[test]
    fn the_baseline_key_is_kind_then_obligation() {
        let finding = Finding::new("suspect-link", "FR-001-AC-1", Severity::high(), "s");
        assert_eq!(finding.key(), "suspect-link:FR-001-AC-1");
    }

    /// A finding round-trips through JSON with every member the producer
    /// wrote, because `src/graph-analysis/` renders the whole finding into its
    /// own report and a dropped member is a changed byte. Carried from
    /// quoin#385, which held the property when nine of these were undeclared.
    #[test]
    fn carries_undeclared_members_back_out_again() {
        let source = r#"{"obligation":"FR-1","kind":"k","summary":"s","severity":"low","line":42,"deep":{"b":[1,null,true],"a":"x"}}"#;
        let finding: Finding = serde_json::from_str(source).expect("reads");
        assert_eq!(finding.other.len(), 1, "{:?}", finding.other);
        let back = serde_json::to_value(&finding).expect("writes");
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(source).expect("reads"),
            "a member the auditor wrote must survive the round trip"
        );
    }

    #[test]
    fn the_twelve_kinds_are_twelve_distinct_spellings() {
        // Anti-vacuity: a list that silently lost an entry to a copy-paste
        // would still be "all of it".
        let distinct: BTreeSet<&str> = FindingKind::ALL.into_iter().collect();
        assert_eq!(distinct.len(), 12);
        assert!(distinct.contains(FindingKind::MOCKED_CONFIRMATION));
    }
}
