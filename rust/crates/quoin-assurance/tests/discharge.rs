// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-046's acceptance criteria, against the public API (quoin#384).
//!
//! # What this file is carrying forward
//!
//! `tests/discharge.test.ts` is deleted by this cutover. Five of its six
//! criteria are `src/assurance/discharge.ts`'s and are restated here.
//!
//! The sixth, **FR-046-AC-1**, is not: both of its assertions call
//! `parseClauseBinding`, which lives in `src/quire/` and is out of this port's
//! scope. It is named here so that nobody reads a five-of-six count as a
//! complete carry-over. `quoin-quire-types`'s
//! `a_clause_binding_report_round_trips_with_quires_own_field_spellings`
//! covers the READER half of that criterion — that quoin can read the pinned
//! wire shape — and covers none of the validation half.
//!
//! # Why the fixture is written here and the expectations are not derived
//!
//! FR-101 names the self-fixture tautology. The input below is a hand-written
//! `clause-binding-v1` document, which is what the retained test used too —
//! quoin cannot capture one from a live quire in CI, for the reason
//! `quoin-quire-types/tests/coverage_reader.rs` records. What makes these
//! tests non-tautological is the other half: every expectation is an
//! independently stated literal — the partition membership, the exact reason
//! strings, and the complete rendered Markdown byte for byte — never a second
//! call to the code under test.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::discharge::{
    BuildDischargeRequest, DischargeState, DispositionDecision, FactKind, UnusedFactReason,
    build_discharge_report, render_discharge_report,
};
use quoin_quire_types::ClauseBindingReport;

/// The emitted `clause-discharge-v1` document for [`every_state_report`].
///
/// Written out rather than re-derived: a test that serialises the report a
/// second time agrees with itself no matter what the wire spellings say.
const EVERY_STATE_DOCUMENT: &str = r#"{"schemaVersion":"clause-discharge-v1","clauseSet":{"authority":"example.invalid","id":"synthetic-widget-rules","version":"1.0.0"},"clauseSetDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","context":{"deployment":"test","product":"widget"},"asOf":"2026-08-15T00:00:00.000Z","binding":{"direct":[{"clauseId":"SYN-001","force":"mandatory","state":"direct","expectedOutputs":["test-result"],"fact":{"kind":"direct","clauseId":"SYN-001","evidenceRefs":["evidence://run/one"],"attestation":{"attestedBy":"reviewer-1","authority":"quality-lead","attestedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z","sourceRevision":"0123456789abcdef","evidenceDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}}],"dispositions":[{"clauseId":"SYN-002","force":"recommended","state":"disposition","expectedOutputs":["review-record"],"fact":{"kind":"disposition","clauseId":"SYN-002","decision":"temporary_exception","rationale":"Synthetic decision for the bounded test window.","approvalRef":"decision://synthetic/temporary_exception","attestation":{"attestedBy":"reviewer-1","authority":"quality-lead","attestedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z","sourceRevision":"0123456789abcdef","evidenceDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}},{"clauseId":"SYN-003","force":"mandatory","state":"disposition","expectedOutputs":["decision-record"],"fact":{"kind":"disposition","clauseId":"SYN-003","decision":"accepted_risk","rationale":"Synthetic decision for the bounded test window.","approvalRef":"decision://synthetic/accepted_risk","attestation":{"attestedBy":"reviewer-1","authority":"quality-lead","attestedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z","sourceRevision":"0123456789abcdef","evidenceDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}},{"clauseId":"SYN-006","force":"permitted","state":"disposition","expectedOutputs":["delegation-record"],"fact":{"kind":"disposition","clauseId":"SYN-006","decision":"delegated","rationale":"Synthetic decision for the bounded test window.","approvalRef":"decision://synthetic/delegated","attestation":{"attestedBy":"reviewer-1","authority":"quality-lead","attestedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z","sourceRevision":"0123456789abcdef","evidenceDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}}],"open":[{"clauseId":"SYN-007","force":"mandatory","state":"open","expectedOutputs":["open-record"],"reason":"no discharge fact"}]},"unresolved":[{"clauseId":"SYN-004","force":"mandatory","state":"unresolved","expectedOutputs":["environment-record"],"reason":"environment is not known"}],"notBinding":[{"clauseId":"SYN-005","force":"permitted","state":"not_binding","expectedOutputs":[]}],"unusedFacts":[{"clauseId":"SYN-004","kind":"direct","reason":"unresolved"},{"clauseId":"SYN-005","kind":"direct","reason":"not_binding"},{"clauseId":"SYN-999","kind":"direct","reason":"unknown_clause"}]}"#;

fn digest() -> String {
    format!("sha256:{}", "a".repeat(64))
}

fn binding() -> ClauseBindingReport {
    serde_json::from_value(serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {
            "authority": "example.invalid",
            "id": "synthetic-widget-rules",
            "version": "1.0.0"
        },
        "clauseSetDigest": digest(),
        "context": { "product": "widget", "deployment": "test" },
        "clauses": [
            {"clauseId": "SYN-001", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["test-result"]},
            {"clauseId": "SYN-002", "force": "recommended", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["review-record"]},
            {"clauseId": "SYN-003", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["decision-record"]},
            {"clauseId": "SYN-004", "force": "mandatory", "outcome": "unresolved",
             "reasons": [{"code": "missing-context", "dimension": "environment",
                          "message": "environment is not known"}],
             "expectedOutputs": ["environment-record"]},
            {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
             "reasons": [], "expectedOutputs": []}
        ]
    }))
    .expect("the fixture matches the pinned wire shape")
}

fn attestation() -> serde_json::Value {
    serde_json::json!({
        "attestedBy": "reviewer-1",
        "authority": "quality-lead",
        "attestedAt": "2026-08-01T00:00:00.000Z",
        "expiresAt": "2026-09-01T00:00:00.000Z",
        "sourceRevision": "0123456789abcdef",
        "evidenceDigest": digest()
    })
}

fn direct() -> serde_json::Value {
    serde_json::json!({
        "kind": "direct",
        "clauseId": "SYN-001",
        "evidenceRefs": ["evidence://run/one"],
        "attestation": attestation()
    })
}

fn disposition() -> serde_json::Value {
    serde_json::json!({
        "kind": "disposition",
        "clauseId": "SYN-002",
        "decision": "temporary_exception",
        "rationale": "Synthetic exception for the bounded test window.",
        "approvalRef": "decision://synthetic/one",
        "attestation": attestation()
    })
}

fn request(facts: Vec<serde_json::Value>) -> BuildDischargeRequest {
    BuildDischargeRequest {
        binding: binding(),
        facts,
        as_of: "2026-08-15T00:00:00.000Z".to_owned(),
    }
}

fn ids(entries: &[quoin_assurance::discharge::ClauseDischarge]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.clause_id.as_str())
        .collect()
}

/// Trace: FR-046-AC-2
/// Provenance: quoin#384, tests/discharge.test.ts
#[test]
fn tc_1126_every_binding_clause_lands_in_exactly_one_population_and_no_score_is_emitted() {
    let report = build_discharge_report(&request(vec![direct(), disposition()]))
        .expect("both facts are well formed");

    assert_eq!(ids(&report.binding.direct), ["SYN-001"]);
    assert_eq!(ids(&report.binding.dispositions), ["SYN-002"]);
    assert_eq!(ids(&report.binding.open), ["SYN-003"]);

    // "exactly once" stated as a census rather than as three separate lists:
    // the three binding clauses appear three times in total across the whole
    // report's binding partition.
    let mut placed: Vec<&str> = Vec::new();
    placed.extend(ids(&report.binding.direct));
    placed.extend(ids(&report.binding.dispositions));
    placed.extend(ids(&report.binding.open));
    placed.sort_unstable();
    assert_eq!(placed, ["SYN-001", "SYN-002", "SYN-003"]);

    // No score, anywhere. The retained test asserts the absence of one
    // property name on the top-level object; this walks the whole serialised
    // document, which is the assertion the criterion actually makes.
    let value = serde_json::to_value(&report).expect("it serialises");
    let text = serde_json::to_string(&value).expect("it serialises");
    assert!(!text.to_lowercase().contains("score"), "{text}");
    assert!(value.get("score").is_none());
}

/// Trace: FR-046-AC-3
/// Provenance: quoin#384, tests/discharge.test.ts
#[test]
fn tc_1127_unresolved_and_not_binding_clauses_stay_outside_the_partition_and_spend_no_fact() {
    let mut unresolved_fact = direct();
    unresolved_fact["clauseId"] = serde_json::json!("SYN-004");
    let mut not_binding_fact = direct();
    not_binding_fact["clauseId"] = serde_json::json!("SYN-005");
    let mut unknown_fact = direct();
    unknown_fact["clauseId"] = serde_json::json!("SYN-999");

    let report = build_discharge_report(&request(vec![
        unknown_fact,
        unresolved_fact,
        not_binding_fact,
    ]))
    .expect("all three facts are well formed");

    assert_eq!(ids(&report.unresolved), ["SYN-004"]);
    assert_eq!(report.unresolved[0].state, DischargeState::Unresolved);
    assert_eq!(
        report.unresolved[0].reason.as_deref(),
        Some("environment is not known")
    );
    assert_eq!(ids(&report.not_binding), ["SYN-005"]);
    assert_eq!(report.not_binding[0].state, DischargeState::NotBinding);

    // Neither is in the binding denominator.
    assert!(!ids(&report.binding.direct).contains(&"SYN-004"));
    assert!(!ids(&report.binding.open).contains(&"SYN-004"));
    assert!(!ids(&report.binding.open).contains(&"SYN-005"));

    // Every unspent fact is reported, with its reason — and the ORDER is the
    // clause-ordered pass before the fact-ordered one, which is why the
    // unknown clause is last despite being supplied first.
    let listed: Vec<(&str, FactKind, UnusedFactReason)> = report
        .unused_facts
        .iter()
        .map(|fact| (fact.clause_id.as_str(), fact.kind, fact.reason))
        .collect();
    assert_eq!(
        listed,
        [
            ("SYN-004", FactKind::Direct, UnusedFactReason::Unresolved),
            ("SYN-005", FactKind::Direct, UnusedFactReason::NotBinding),
            ("SYN-999", FactKind::Direct, UnusedFactReason::UnknownClause),
        ]
    );
}

/// Trace: FR-046-AC-4
/// Provenance: quoin#384, tests/discharge.test.ts
#[test]
fn tc_1128_expired_future_dated_and_incoherent_attestations_leave_the_clause_open() {
    let reopened = |mutate: fn(&mut serde_json::Value)| {
        let mut fact = direct();
        mutate(&mut fact);
        let report = build_discharge_report(&request(vec![fact])).expect("the fact parses");
        assert!(
            report.binding.direct.is_empty(),
            "an attestation that is not current must not discharge"
        );
        let open = report.binding.open;
        assert_eq!(open[0].clause_id, "SYN-001");
        assert_eq!(open[0].state, DischargeState::Open);
        // The fact is carried on the entry, so the reader can see WHICH
        // attestation failed rather than only that one did.
        assert!(open[0].fact.is_some());
        open[0].reason.clone().expect("the reason is visible")
    };

    assert_eq!(
        reopened(
            |fact| fact["attestation"]["expiresAt"] = serde_json::json!("2026-08-10T00:00:00.000Z")
        ),
        "discharge fact is expired"
    );
    assert_eq!(
        reopened(|fact| {
            fact["attestation"]["attestedAt"] = serde_json::json!("2026-08-20T00:00:00.000Z");
            fact["attestation"]["expiresAt"] = serde_json::json!("2026-09-20T00:00:00.000Z");
        }),
        "attestation is in the future"
    );
    assert_eq!(
        reopened(
            |fact| fact["attestation"]["expiresAt"] = serde_json::json!("2026-08-01T00:00:00.000Z")
        ),
        "attestation expiry is not after attestation"
    );

    // A binding clause with no fact at all is open for a different, stated
    // reason — the criterion is that the reason is VISIBLE, not merely that
    // the clause is open.
    let report = build_discharge_report(&request(vec![])).expect("no facts is valid");
    assert_eq!(
        report.binding.open[0].reason.as_deref(),
        Some("no discharge fact")
    );
    assert!(report.binding.open[0].fact.is_none());
}

/// Trace: FR-046-AC-5
/// Provenance: quoin#384, tests/discharge.test.ts
#[test]
fn tc_1129_duplicate_facts_and_incomplete_attestations_are_rejected() {
    let refused = |facts: Vec<serde_json::Value>| {
        build_discharge_report(&request(facts))
            .expect_err("the request must be refused")
            .0
    };

    // Not resolved by ordering: neither copy wins.
    assert_eq!(
        refused(vec![direct(), direct()]),
        "duplicate discharge fact for clause SYN-001"
    );

    let mut empty_authority = direct();
    empty_authority["attestation"]["authority"] = serde_json::json!("");
    assert_eq!(
        refused(vec![empty_authority]),
        "authority must not be empty"
    );

    let mut invented_kind = direct();
    invented_kind["kind"] = serde_json::json!("invented");
    assert_eq!(
        refused(vec![invented_kind]),
        "kind must be one of direct, disposition"
    );

    let mut unknown_field = direct();
    unknown_field["inventedScore"] = serde_json::json!(100);
    assert_eq!(
        refused(vec![unknown_field]),
        "direct discharge fact has unknown field inventedScore"
    );

    // The rest of `parseAttestation`, which the retained test names as
    // "incomplete attestations" and reaches only one of.
    let mut missing_field = direct();
    missing_field["attestation"]
        .as_object_mut()
        .unwrap()
        .remove("sourceRevision");
    assert_eq!(
        refused(vec![missing_field]),
        "attestation is missing sourceRevision"
    );

    let mut bad_digest = direct();
    bad_digest["attestation"]["evidenceDigest"] = serde_json::json!("not-a-digest");
    assert_eq!(
        refused(vec![bad_digest]),
        "attestation evidenceDigest must be sha256:<64 lowercase hex>"
    );

    let mut rolled_day = direct();
    // quoin#436: `Date.parse` ROLLED this to March 2 rather than refusing it.
    rolled_day["attestation"]["expiresAt"] = serde_json::json!("2026-02-30T00:00:00.000Z");
    assert_eq!(
        refused(vec![rolled_day]),
        "expiresAt must be an ISO-8601 instant"
    );

    let mut empty_evidence = direct();
    empty_evidence["evidenceRefs"] = serde_json::json!([]);
    assert_eq!(
        refused(vec![empty_evidence]),
        "evidenceRefs must contain non-empty values"
    );

    let mut repeated_evidence = direct();
    repeated_evidence["evidenceRefs"] =
        serde_json::json!(["evidence://run/one", "evidence://run/one"]);
    assert_eq!(
        refused(vec![repeated_evidence]),
        "evidenceRefs must contain unique values"
    );

    // `asOf` is validated before anything else is even read.
    let mut bad_as_of = request(vec![direct()]);
    bad_as_of.as_of = "yesterday".to_owned();
    assert_eq!(
        build_discharge_report(&bad_as_of)
            .expect_err("not an instant")
            .0,
        "asOf must be an ISO-8601 instant"
    );
}

/// Trace: FR-046-AC-6
/// Provenance: quoin#384, tests/discharge.test.ts
#[test]
fn tc_1130_every_population_renders_deterministically_and_without_a_score() {
    let report = build_discharge_report(&request(vec![direct(), disposition()]))
        .expect("both facts are well formed");
    let rendered = render_discharge_report(&report);

    // The literal, not a re-derivation: every heading, every row, the blank
    // lines between them, and the single trailing newline.
    let expected = format!(
        "# Clause discharge: example.invalid/synthetic-widget-rules@1.0.0\n\
         \n\
         - Clause-set digest: `{}`\n\
         - Evaluated as of: `2026-08-15T00:00:00.000Z`\n\
         \n\
         ## Direct evidence\n\
         \n\
         - `SYN-001` — mandatory; `test-result`\n\
         \n\
         ## Approved dispositions\n\
         \n\
         - `SYN-002` — recommended; `review-record`\n\
         \n\
         ## Open binding clauses\n\
         \n\
         - `SYN-003` — mandatory; `decision-record`; no discharge fact\n\
         \n\
         ## Unresolved applicability\n\
         \n\
         - `SYN-004` — mandatory; `environment-record`; environment is not known\n\
         \n\
         ## Not binding\n\
         \n\
         - `SYN-005` — permitted; no declared outputs\n",
        digest()
    );
    assert_eq!(rendered, expected);
    assert!(!rendered.to_lowercase().contains("score"));

    // Deterministic: the same report renders the same bytes, and a rebuilt
    // report from the same request does too.
    assert_eq!(render_discharge_report(&report), rendered);
    let again = build_discharge_report(&request(vec![direct(), disposition()])).expect("valid");
    assert_eq!(render_discharge_report(&again), rendered);

    // An empty population is STATED, not omitted — the whole point of a
    // report that exposes every input population.
    let empty = build_discharge_report(&BuildDischargeRequest {
        binding: serde_json::from_value(serde_json::json!({
            "schemaVersion": "clause-binding-v1",
            "clauseSet": {"authority": "example.invalid", "id": "empty", "version": "1.0.0"},
            "clauseSetDigest": digest(),
            "context": {},
            "clauses": []
        }))
        .expect("an empty clause set is a valid report"),
        facts: vec![],
        as_of: "2026-08-15T00:00:00.000Z".to_owned(),
    })
    .expect("valid");
    let rendered_empty = render_discharge_report(&empty);
    assert_eq!(rendered_empty.matches("_None._").count(), 5);
    assert!(rendered_empty.ends_with("## Not binding\n\n_None._\n"));
    assert!(!rendered_empty.ends_with("_None._\n\n"));

    // The unused-facts section appears only when there is something in it.
    assert!(!rendered.contains("## Unused facts"));
    let mut unknown = direct();
    unknown["clauseId"] = serde_json::json!("SYN-999");
    let with_unused = build_discharge_report(&request(vec![unknown])).expect("valid");
    assert!(
        render_discharge_report(&with_unused).contains("- `SYN-999` (direct): unknown_clause\n")
    );
}

/// A report the port actually computed still reads back.
///
/// The refusal below is only worth having if the validating door is the one a
/// legitimate `build_discharge` → `render_discharge` hop goes through, which
/// is the hop `quoin-core` makes over two invocations.
///
/// Trace: FR-046-AC-5, FR-046-AC-6
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_451_a_computed_discharge_report_round_trips_through_the_validating_door() {
    let report = build_discharge_report(&request(vec![direct(), disposition()]))
        .expect("both facts are well formed");
    let payload = serde_json::to_value(&report).expect("it serialises");
    let read_back: quoin_assurance::DischargeReport =
        serde_json::from_value(payload).expect("the emitted document reads back");
    assert_eq!(read_back, report);
}

/// Accept and emit spell every closed vocabulary the same way.
///
/// `parse_fact`'s membership tables are built from `as_str()`, and this pins
/// `as_str()` against what `#[serde(rename_all)]` writes — in BOTH directions,
/// so neither half can move alone. Flipping `DischargeState` to `camelCase`
/// passed every test in this crate before quoin#447; it fails here.
///
/// Trace: FR-046-AC-5, FR-046-AC-6
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_452_the_wire_spelling_of_every_closed_vocabulary_has_one_source() {
    for kind in FactKind::all() {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            serde_json::json!(kind.as_str())
        );
        assert_eq!(
            serde_json::from_value::<FactKind>(serde_json::json!(kind.as_str())).unwrap(),
            *kind
        );
    }
    for decision in DispositionDecision::all() {
        assert_eq!(
            serde_json::to_value(decision).unwrap(),
            serde_json::json!(decision.as_str())
        );
        assert_eq!(
            serde_json::from_value::<DispositionDecision>(serde_json::json!(decision.as_str()))
                .unwrap(),
            *decision
        );
    }
    for state in DischargeState::all() {
        assert_eq!(
            serde_json::to_value(state).unwrap(),
            serde_json::json!(state.as_str())
        );
        assert_eq!(
            serde_json::from_value::<DischargeState>(serde_json::json!(state.as_str())).unwrap(),
            *state
        );
    }
    for reason in UnusedFactReason::all() {
        assert_eq!(
            serde_json::to_value(reason).unwrap(),
            serde_json::json!(reason.as_str())
        );
        assert_eq!(
            serde_json::from_value::<UnusedFactReason>(serde_json::json!(reason.as_str())).unwrap(),
            *reason
        );
    }
    // The set is closed as well as spelled: the accept side names exactly the
    // variants `all()` lists, in that order.
    let mut renamed = disposition();
    renamed["decision"] = serde_json::json!("temporaryException");
    assert_eq!(
        build_discharge_report(&request(vec![renamed]))
            .expect_err("an unlisted decision is refused")
            .0,
        "decision must be one of accepted_risk, temporary_exception, delegated"
    );
}

/// A clause set that reaches every state, every decision and every unused
/// reason — including `accepted_risk` and `delegated`, which no other fixture
/// in this crate constructs.
fn every_state_binding() -> ClauseBindingReport {
    serde_json::from_value(serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {
            "authority": "example.invalid",
            "id": "synthetic-widget-rules",
            "version": "1.0.0"
        },
        "clauseSetDigest": digest(),
        "context": { "product": "widget", "deployment": "test" },
        "clauses": [
            {"clauseId": "SYN-001", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["test-result"]},
            {"clauseId": "SYN-002", "force": "recommended", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["review-record"]},
            {"clauseId": "SYN-003", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["decision-record"]},
            {"clauseId": "SYN-004", "force": "mandatory", "outcome": "unresolved",
             "reasons": [{"code": "missing-context", "dimension": "environment",
                          "message": "environment is not known"}],
             "expectedOutputs": ["environment-record"]},
            {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-006", "force": "permitted", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["delegation-record"]},
            {"clauseId": "SYN-007", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["open-record"]}
        ]
    }))
    .expect("the fixture matches the pinned wire shape")
}

/// One disposition fact, for a named decision.
fn disposition_of(clause_id: &str, decision: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "disposition",
        "clauseId": clause_id,
        "decision": decision,
        "rationale": "Synthetic decision for the bounded test window.",
        "approvalRef": format!("decision://synthetic/{decision}"),
        "attestation": attestation()
    })
}

/// The report every state, decision and unused reason appears in.
fn every_state_report() -> quoin_assurance::DischargeReport {
    let mut unresolved_fact = direct();
    unresolved_fact["clauseId"] = serde_json::json!("SYN-004");
    let mut not_binding_fact = direct();
    not_binding_fact["clauseId"] = serde_json::json!("SYN-005");
    let mut unknown_fact = direct();
    unknown_fact["clauseId"] = serde_json::json!("SYN-999");

    build_discharge_report(&BuildDischargeRequest {
        binding: every_state_binding(),
        facts: vec![
            direct(),
            disposition_of("SYN-002", "temporary_exception"),
            disposition_of("SYN-003", "accepted_risk"),
            disposition_of("SYN-006", "delegated"),
            unresolved_fact,
            not_binding_fact,
            unknown_fact,
        ],
        as_of: "2026-08-15T00:00:00.000Z".to_owned(),
    })
    .expect("every fact is well formed")
}

/// The whole emitted `clause-discharge-v1` document, pinned as one literal.
///
/// The Markdown render has been pinned byte for byte since quoin#384; the JSON
/// was asserted only structurally — key presence and enum equality — so a
/// changed `#[serde(rename_all)]` moved the emitted document without moving a
/// test, and a consumer matching `not_binding` would have seen zero
/// not-binding clauses. Every state, every disposition decision and every
/// unused reason appears here, so the pin covers the whole vocabulary rather
/// than the two spellings the other fixtures happen to reach.
///
/// Trace: FR-046-AC-2, FR-046-AC-6
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_453_the_emitted_json_document_is_pinned_byte_for_byte() {
    let report = every_state_report();
    assert_eq!(
        serde_json::to_string(&report).expect("it serialises"),
        EVERY_STATE_DOCUMENT
    );
    // The pinned bytes are also what the validating door accepts back, so the
    // literal is a statement about the document a consumer receives and not
    // only about this process's output.
    let read_back: quoin_assurance::DischargeReport =
        serde_json::from_str(EVERY_STATE_DOCUMENT).expect("the pinned document reads back");
    assert_eq!(read_back, report);
}

/// Every reader in the emitted document refuses a field it does not know, at
/// every depth. `assurance.render_discharge` reads a whole
/// `clause-discharge-v1` document off untrusted stdin, so a key silently
/// dropped here is a key the caller believes quoin honoured.
///
/// The unknown key is injected where it is REACHABLE rather than each struct
/// being read standalone: an isolated struct refusing it says nothing about
/// the path a caller actually reaches it by.
///
/// Trace: FR-046-AC-1, FR-046-AC-5
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_442_every_reachable_discharge_type_refuses_an_unknown_field() {
    let document: serde_json::Value =
        serde_json::from_str(EVERY_STATE_DOCUMENT).expect("the pinned document parses as JSON");

    for pointer in [
        "",
        "/clauseSet",
        "/binding",
        "/binding/direct/0",
        "/binding/direct/0/fact",
        "/binding/direct/0/fact/attestation",
        "/binding/dispositions/0",
        "/binding/dispositions/0/fact",
        "/binding/open/0",
        "/unresolved/0",
        "/notBinding/0",
        "/unusedFacts/0",
    ] {
        let mut forged = document.clone();
        let target = if pointer.is_empty() {
            &mut forged
        } else {
            forged
                .pointer_mut(pointer)
                .unwrap_or_else(|| panic!("{pointer} is a path this document has"))
        };
        target
            .as_object_mut()
            .unwrap_or_else(|| panic!("{pointer} names an object"))
            .insert("inventedField".to_owned(), serde_json::json!(1));

        assert!(
            serde_json::from_value::<quoin_assurance::DischargeReport>(forged).is_err(),
            "an unknown field at {pointer} must be refused"
        );
    }
}

/// The forged discharge report quoin#447's review demonstrated, replayed.
///
/// Trace: FR-046-AC-5
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_450_a_forged_discharge_report_is_refused_by_the_deserialiser() {
    assert!(
        serde_json::from_value::<quoin_assurance::DischargeReport>(forged_report()).is_err(),
        "a report whose fact never went through parseFact must not deserialise"
    );
}

/// A `clause-discharge-v1` document nothing computed: the attestation carries
/// an empty `authority`, an `attestedAt` that is not an instant, an expiry
/// before it, an `evidenceDigest` that is not one, and no evidence at all.
fn forged_report() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "clause-discharge-v1",
        "clauseSet": {"authority": "example.invalid", "id": "synthetic-widget-rules",
                      "version": "1.0.0"},
        "clauseSetDigest": digest(),
        "context": {},
        "asOf": "2026-08-15T00:00:00.000Z",
        "binding": {
            "direct": [{
                "clauseId": "SYN-001",
                "force": "mandatory",
                "state": "direct",
                "expectedOutputs": ["test-result"],
                "fact": {
                    "kind": "direct",
                    "clauseId": "SYN-001",
                    "evidenceRefs": [],
                    "attestation": {
                        "attestedBy": "nobody",
                        "authority": "",
                        "attestedAt": "nope",
                        "expiresAt": "1999-01-01T00:00:00.000Z",
                        "sourceRevision": "0123456789abcdef",
                        "evidenceDigest": "not-a-digest"
                    }
                }
            }],
            "dispositions": [],
            "open": []
        },
        "unresolved": [],
        "notBinding": [],
        "unusedFacts": []
    })
}
