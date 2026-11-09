// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use super::super::fixtures::{
    AS_OF, CLAIM_EVIDENCE_REF, argument, build, decision, json, request, request_with_evidence,
    resolved_evidence, with,
};
use super::super::view::{EvidenceIndexEntry, EvidenceRefState, ViewStatus};
use super::build_authored_argument_view;
use crate::argument::parse_assurance_argument;
use ix_trace_rs::trace;

/// Provenance: quoin#445, quoin#447
#[trace("FR-047-AC-1")]
#[test]
fn tc_1131_preserves_the_authored_claim_authority_and_independence() {
    let view = build(argument(), vec![decision()], AS_OF);
    // The expectation is the RETAINED implementation's own output, taken
    // from `dist/` at this revision and written out — not a re-derivation
    // from the code under test.
    let expected: serde_json::Value = serde_json::from_str(
        r#"{"schemaVersion":"authored-assurance-view-v1","argument":{"id":"AA-900","title":"Synthetic widget release decision","status":"active","owner":"release-owner","profile":"ix://example.invalid/widget/AP-900"},"asOf":"2026-08-15T00:00:00.000Z","topClaim":{"id":"CLAIM-900","statement":"The bounded synthetic widget change is acceptable.","subject":"widget revision 0123456789abcdef","status":"supported","reasons":[]},"reasoning":[{"id":"ARG-900","statement":"Argue from the explicitly reviewed clause disposition.","supports":"CLAIM-900","status":"supported","criteria":[{"criterion":"Every binding synthetic clause has a current disposition.","status":"supported","decision":{"reasoningId":"ARG-900","criterion":"Every binding synthetic clause has a current disposition.","state":"satisfied","evidenceRefs":["evidence://discharge/widget-900"],"decidedBy":"reviewer-900","authority":"may accept or reject this synthetic release","decidedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z"}}]}],"assumptions":[{"id":"ASM-900","statement":"The test environment represents the bounded target.","owner":"release-owner","status":"supported","declaredStatus":"accepted","reviewBy":"2026-09-01T00:00:00.000Z"}],"participants":[{"id":"reviewer-900","role":"decision reviewer","authority":"may accept or reject this synthetic release","independence":"did not produce the implementation evidence"}],"challenges":[{"id":"CH-900","target":"CLAIM-900","statement":"A bounded recovery case needed review.","owner":"release-owner","status":"resolved","declaredStatus":"resolved","resolutionRefs":["evidence://experiment/recovery-900"]}],"relationships":[{"target":"ix://example.invalid/widget/AP-900","type":"references"}],"unusedDecisions":[]}"#,
    )
    .expect("the recorded oracle output is JSON");
    assert_eq!(json(&view), expected);

    // The participants are carried through unchanged, authority and
    // independence included — which is the half of AC-1 a `toMatchObject`
    // on the top claim alone would not have seen.
    assert_eq!(json(&view)["participants"], argument()["participants"]);
}

/// No score, anywhere, at any depth.
///
/// Provenance: quoin#445
#[trace("FR-047-AC-1")]
#[test]
fn tc_1131_emits_no_score_at_any_depth() {
    let view = build(argument(), vec![decision()], AS_OF);
    let text = serde_json::to_string(&view).expect("the view serialises");
    assert!(
        !text.contains("score"),
        "the view must promote no evidence result into a number: {text}"
    );
}

/// Provenance: quoin#445
#[trace("FR-047-AC-2")]
#[test]
fn tc_1132_leaves_an_undecided_criterion_open_instead_of_inferring_from_evidence() {
    // The argument DOES carry an evidence reference — the challenge's
    // `resolution_refs` — so an implementation that inferred from evidence
    // has something to infer from here.
    let view = build(argument(), vec![], AS_OF);
    assert_eq!(view.reasoning[0].criteria[0].status, ViewStatus::Open);
    assert_eq!(
        view.reasoning[0].criteria[0].reason.as_deref(),
        Some("no sufficiency decision")
    );
    assert_eq!(view.reasoning[0].criteria[0].decision, None);
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec!["one or more sufficiency criteria are open".to_owned()]
    );
}

/// Provenance: quoin#445
#[trace("FR-047-AC-3")]
#[test]
fn tc_1133_reopens_expired_decisions_and_assumptions_due_for_review() {
    let assumption = with(
        &argument()["assumptions"][0],
        "review_by",
        serde_json::json!("2026-08-10T00:00:00.000Z"),
    );
    let expired = with(
        &decision(),
        "expiresAt",
        serde_json::json!("2026-08-10T00:00:00.000Z"),
    );
    let view = build(
        with(&argument(), "assumptions", serde_json::json!([assumption])),
        vec![expired],
        AS_OF,
    );
    assert_eq!(
        view.reasoning[0].criteria[0].reason.as_deref(),
        Some("decision is expired")
    );
    // The refused decision is still reported, which is how a reader tells
    // an expired decision from an absent one.
    assert!(view.reasoning[0].criteria[0].decision.is_some());
    assert_eq!(
        view.assumptions[0].reason.as_deref(),
        Some("assumption review is due")
    );
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec![
            "one or more sufficiency criteria are open".to_owned(),
            "one or more assumptions are open, invalidated, or due for review".to_owned(),
        ]
    );
}

/// Provenance: quoin#445
#[trace("FR-047-AC-5")]
#[test]
fn tc_1135_validates_the_closed_authored_contract_and_rejects_duplicate_decisions() {
    // The round trip the retained test spelled `toEqual(argument)`: the
    // parse emits the authored document back, unchanged.
    let parsed = parse_assurance_argument(&argument()).expect("the fixture is valid");
    assert_eq!(
        serde_json::to_value(&parsed).expect("the argument serialises"),
        argument()
    );

    let cases: [(serde_json::Value, Vec<serde_json::Value>, &str); 5] = [
        (
            with(&argument(), "invented_score", serde_json::json!(100)),
            vec![decision()],
            "argument has unknown field invented_score",
        ),
        (
            argument(),
            vec![decision(), decision()],
            "duplicate sufficiency decision for ARG-900: Every binding synthetic clause has a current disposition.",
        ),
        (
            argument(),
            vec![with(
                &decision(),
                "authority",
                serde_json::json!("self-declared authority"),
            )],
            "sufficiency decision authority for reviewer-900 does not match the authored participant",
        ),
        (
            argument(),
            vec![with(&decision(), "inventedScore", serde_json::json!(100))],
            "sufficiency decision has unknown field inventedScore",
        ),
        (
            with(
                &argument(),
                "assumptions",
                serde_json::json!([with(
                    &argument()["assumptions"][0],
                    "id",
                    argument()["top_claim"]["id"].clone()
                )]),
            ),
            vec![decision()],
            "top claim, reasoning, and assumption ids must be unique",
        ),
    ];
    for (argument, decisions, message) in cases {
        let error = build_authored_argument_view(&request(argument, decisions, AS_OF))
            .expect_err("the case is a rejection");
        assert_eq!(error.0, message);
    }
}

/// The instant is read as a NUMBER here, and the number has to be right —
/// a validator that only answered yes/no would pass every test above and
/// still order two instants wrongly.
///
/// Provenance: quoin#436, quoin#445
#[trace("FR-047-AC-3", "FR-047-AC-7", "TC-1712")]
#[test]
fn tc_1712_the_offset_and_the_fraction_move_the_comparison() {
    // `2026-08-15T00:00:00.000Z` as an offset instant one minute EARLIER,
    // which must therefore read as due.
    let due = with(
        &argument()["assumptions"][0],
        "review_by",
        serde_json::json!("2026-08-15T05:29:00.000+05:30"),
    );
    let view = build(
        with(&argument(), "assumptions", serde_json::json!([due])),
        vec![decision()],
        AS_OF,
    );
    assert_eq!(
        view.assumptions[0].reason.as_deref(),
        Some("assumption review is due"),
        "05:29+05:30 is 23:59Z the previous day, which is before the asOf"
    );

    // One minute LATER, same offset: not due.
    let current = with(
        &argument()["assumptions"][0],
        "review_by",
        serde_json::json!("2026-08-15T05:31:00.000+05:30"),
    );
    let view = build(
        with(&argument(), "assumptions", serde_json::json!([current])),
        vec![decision()],
        AS_OF,
    );
    assert_eq!(view.assumptions[0].reason, None);

    // One millisecond after the asOf is not due; exactly the asOf is.
    for (review_by, expected) in [
        ("2026-08-15T00:00:00.001Z", None),
        ("2026-08-15T00:00:00.000Z", Some("assumption review is due")),
    ] {
        let assumption = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!(review_by),
        );
        let view = build(
            with(&argument(), "assumptions", serde_json::json!([assumption])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.assumptions[0].reason.as_deref(), expected);
    }
}

/// A decision nobody asked for is reported rather than dropped.
///
/// Provenance: quoin#445
#[trace("FR-047-AC-2")]
#[test]
fn tc_1132_a_decision_matching_no_authored_criterion_is_reported_unused() {
    let view = build(
        argument(),
        vec![with(
            &decision(),
            "criterion",
            serde_json::json!("an unmatched criterion"),
        )],
        AS_OF,
    );
    assert_eq!(view.unused_decisions.len(), 1);
    assert_eq!(view.unused_decisions[0].reasoning_id, "ARG-900");
    assert_eq!(view.unused_decisions[0].criterion, "an unmatched criterion");
    // And the criterion it did not decide is still open.
    assert_eq!(view.reasoning[0].criteria[0].status, ViewStatus::Open);
}

/// A non-active argument opens the top claim on its own.
///
/// Provenance: quoin#445
#[trace("FR-047-AC-1")]
#[test]
fn tc_1131_a_proposed_argument_opens_the_top_claim() {
    let view = build(
        with(&argument(), "status", serde_json::json!("proposed")),
        vec![decision()],
        AS_OF,
    );
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec!["argument status is proposed".to_owned()]
    );
}

/// The control case: a claim citing evidence the store resolves cleanly
/// passes clean, and this is what every OTHER fixture in this file already
/// relies on — `argument()`'s top claim carries one reference and
/// [`request`] resolves it by default.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_a_claim_backed_by_resolved_evidence_passes_clean() {
    let view = build(argument(), vec![decision()], AS_OF);
    assert_eq!(view.top_claim.status, ViewStatus::Supported);
    assert_eq!(view.top_claim.reasons, Vec::<String>::new());
}

/// A claim with no evidence references at all is unbacked.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_a_claim_with_no_evidence_references_is_unbacked() {
    let claim_without_refs = with(
        &argument()["top_claim"],
        "evidence_refs",
        serde_json::json!([]),
    );
    let view = build(
        with(&argument(), "top_claim", claim_without_refs),
        vec![decision()],
        AS_OF,
    );
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec!["the top claim has no evidence references".to_owned()]
    );
}

/// A claim citing a reference the evidence store has never heard of is
/// unbacked, distinctly from citing nothing.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_a_claim_citing_an_unresolved_reference_is_unbacked() {
    let view = build_authored_argument_view(&request_with_evidence(
        argument(),
        vec![decision()],
        AS_OF,
        Vec::new(),
    ))
    .expect("the fixture builds");
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec![format!(
            "the top claim's evidence reference {CLAIM_EVIDENCE_REF} does not resolve in the evidence store"
        )]
    );
}

/// A claim citing evidence the auditor already flagged is unbacked, and
/// the reason names the auditor's own word for each of the two states.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_a_claim_citing_stale_or_vacuous_evidence_is_unbacked() {
    let cases = [
        (EvidenceRefState::Stale, "is stale"),
        (EvidenceRefState::Vacuous, "is vacuous"),
    ];
    for (state, expected_tail) in cases {
        let view = build_authored_argument_view(&request_with_evidence(
            argument(),
            vec![decision()],
            AS_OF,
            vec![EvidenceIndexEntry {
                reference: CLAIM_EVIDENCE_REF.to_owned(),
                state,
            }],
        ))
        .expect("the fixture builds");
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec![format!(
                "the top claim's evidence reference {CLAIM_EVIDENCE_REF} {expected_tail}"
            )]
        );
    }
}

/// Every failing reference is reported, in authored order: an earlier
/// stale reference does not hide a later vacuous one, and a resolved
/// reference between them contributes nothing.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_reports_every_failing_reference_in_authored_order() {
    let claim = with(
        &argument()["top_claim"],
        "evidence_refs",
        serde_json::json!([
            "evidence://widget/stale",
            CLAIM_EVIDENCE_REF,
            "evidence://widget/vacuous",
            "evidence://widget/missing"
        ]),
    );
    let mut evidence = resolved_evidence();
    evidence.push(EvidenceIndexEntry {
        reference: "evidence://widget/stale".to_owned(),
        state: EvidenceRefState::Stale,
    });
    evidence.push(EvidenceIndexEntry {
        reference: "evidence://widget/vacuous".to_owned(),
        state: EvidenceRefState::Vacuous,
    });
    let view = build_authored_argument_view(&request_with_evidence(
        with(&argument(), "top_claim", claim),
        vec![decision()],
        AS_OF,
        evidence,
    ))
    .expect("the fixture builds");
    assert_eq!(view.top_claim.status, ViewStatus::Open);
    assert_eq!(
        view.top_claim.reasons,
        vec![
            "the top claim's evidence reference evidence://widget/stale is stale".to_owned(),
            "the top claim's evidence reference evidence://widget/vacuous is vacuous"
                .to_owned(),
            "the top claim's evidence reference evidence://widget/missing does not resolve in the evidence store"
                .to_owned(),
        ]
    );
}

/// An evidence index naming one reference twice is refused rather than
/// resolved by list order: last-write-wins would let a trailing
/// `resolved` entry mask a `stale` one.
///
/// Provenance: PLAT-966
#[trace("FR-047-AC-8", "TC-1925")]
#[test]
fn tc_1925_a_duplicate_evidence_index_entry_is_refused() {
    let mut evidence = vec![EvidenceIndexEntry {
        reference: CLAIM_EVIDENCE_REF.to_owned(),
        state: EvidenceRefState::Stale,
    }];
    evidence.extend(resolved_evidence());
    let error = build_authored_argument_view(&request_with_evidence(
        argument(),
        vec![decision()],
        AS_OF,
        evidence,
    ))
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("duplicate evidence index entry for {CLAIM_EVIDENCE_REF}")
    );
}
