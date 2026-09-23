// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verification, checked against the receipts the oracle produced (FR-065).
//!
//! Every assertion here compares against `tests/fixtures/oracle.json`, which
//! the retained TypeScript wrote. No test in this file asserts against a value
//! this crate produced: the fixture is the oracle's word, and a Rust receipt
//! that differs from it anywhere — one reason, one ordering, one digest — is a
//! failure.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use quoin_change_assurance::model::outcome::Reason;
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{member, oracle, section, text, verification_input};

/// The captured scenarios, as (name, input JSON) pairs.
fn scenarios() -> Vec<JsonValue> {
    section(&oracle(), "verifications")
}

/// One replayed scenario: its name, the receipt the oracle produced, the
/// refusal it produced instead, and the receipt this crate produced.
type Replay = (String, Option<JsonValue>, Option<String>, Option<JsonValue>);

/// Every scenario in the capture, verified, paired with what the oracle said.
fn replayed() -> Vec<Replay> {
    scenarios()
        .iter()
        .map(|scenario| {
            let name = text(scenario, "name").to_owned();
            let expected = match member(scenario, "receipt") {
                JsonValue::Null => None,
                receipt => Some(receipt.clone()),
            };
            let refusal = match member(scenario, "refusal") {
                JsonValue::Null => None,
                message => Some(message.as_str().unwrap().to_owned()),
            };
            let input = verification_input(member(scenario, "input"));
            let produced = verify_change_assurance(&input)
                .ok()
                .map(|receipt| receipt.to_json().unwrap());
            (name, expected, refusal, produced)
        })
        .collect()
}

/// Trace: FR-065-AC-1
///
/// The receipt the oracle emitted and the receipt this crate emits are the
/// same document — every member, including the decision event, the chain tail,
/// the parent chain and all four named checks.
#[test]
fn tc_455_every_captured_receipt_is_reproduced_member_for_member() {
    let replayed = replayed();
    assert!(
        replayed.len() >= 30,
        "anti-vacuity floor: the capture must carry at least 30 scenarios, found {}",
        replayed.len()
    );
    let mut compared = 0_usize;
    for (name, expected, _, produced) in &replayed {
        let Some(expected) = expected else { continue };
        let produced = produced.as_ref().unwrap_or_else(|| {
            panic!("{name}: the oracle produced a receipt and this crate did not")
        });
        assert_eq!(
            canonical_bytes(produced).unwrap(),
            canonical_bytes(expected).unwrap(),
            "{name}: receipt differs from the oracle's"
        );
        compared += 1;
    }
    assert!(
        compared >= 30,
        "anti-vacuity floor: at least 30 receipts must be compared, compared {compared}"
    );
}

/// Trace: FR-065-AC-10
///
/// A receipt's digest is the digest of its own canonical bytes, in every
/// captured scenario — so a receipt read back from anywhere verifies against
/// itself without the verification being re-run.
#[test]
fn tc_455_every_receipt_digest_names_its_own_canonical_bytes() {
    let mut checked = 0_usize;
    for (name, expected, _, _) in &replayed() {
        let Some(expected) = expected else { continue };
        let receipt = quoin_change_assurance::verify::verify_receipt(expected)
            .unwrap_or_else(|error| panic!("{name}: the oracle's own receipt is refused: {error}"));
        assert_eq!(receipt.digest.as_hex(), text(expected, "digest"));
        checked += 1;
    }
    assert!(
        checked >= 30,
        "anti-vacuity floor: checked {checked} receipts"
    );
}

/// Trace: FR-065-AC-2
///
/// Outcome precedence: any invalid reason dominates, a wholly incomplete reason
/// set is incomplete, and no reason at all is valid. Asserted over the oracle's
/// own receipts rather than over this crate's table, so the table cannot be
/// wrong in the same direction as the check.
#[test]
fn tc_455_outcome_precedence_matches_the_oracle_on_every_receipt() {
    let mut seen_valid = false;
    let mut seen_invalid = false;
    let mut seen_incomplete = false;
    for (name, expected, _, _) in &replayed() {
        let Some(expected) = expected else { continue };
        let JsonValue::Array(reasons) = member(expected, "reasons") else {
            panic!("{name}: reasons is not an array");
        };
        let reasons: Vec<Reason> = reasons
            .iter()
            .map(|reason| Reason::parse(reason.as_str().unwrap()).unwrap())
            .collect();
        let outcome = quoin_change_assurance::model::outcome::outcome_for_reasons(&reasons);
        assert_eq!(
            outcome.as_str(),
            text(expected, "outcome"),
            "{name}: outcome disagrees with the oracle"
        );
        match outcome.as_str() {
            "valid" => seen_valid = true,
            "invalid" => seen_invalid = true,
            _ => seen_incomplete = true,
        }
    }
    assert!(
        seen_valid && seen_invalid && seen_incomplete,
        "anti-vacuity floor: all three outcomes must occur in the capture"
    );
}

/// Trace: FR-065-AC-3
///
/// One receipt per reviewed proof, in `proof_id` order; a selection naming a
/// proof the record does not declare is refused as a mismatch rather than
/// silently judged.
#[test]
fn tc_455_one_verdict_per_proof_and_an_unknown_selection_is_a_mismatch() {
    let mut saw_unknown_selection = false;
    for (name, expected, _, _) in &replayed() {
        let Some(expected) = expected else { continue };
        let JsonValue::Array(proofs) = member(expected, "proofs") else {
            panic!("{name}: proofs is not an array");
        };
        let ids: Vec<&str> = proofs.iter().map(|proof| text(proof, "proof_id")).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "{name}: proofs are not in proof_id order");
        if name == "selection-names-an-unknown-proof" {
            saw_unknown_selection = true;
            let JsonValue::Array(reasons) = member(expected, "reasons") else {
                panic!("reasons");
            };
            assert!(
                reasons
                    .iter()
                    .any(|reason| reason.as_str() == Some("proof_id_mismatch")),
                "an unknown selection must be named"
            );
        }
    }
    assert!(
        saw_unknown_selection,
        "anti-vacuity floor: the scenario must exist"
    );
}

/// Trace: FR-065-AC-4
///
/// Each of the six bindings between a proof obligation and an attestation —
/// record digest, candidate revision, proof id, argv/cwd, tool identity,
/// configuration — invalidates the selection on its own.
#[test]
fn tc_455_each_binding_mismatch_is_named_on_its_own() {
    let expected_reasons = [
        ("bound-to-another-record", "record_binding_mismatch"),
        ("another-candidate-revision", "candidate_revision_mismatch"),
        ("another-command", "command_mismatch"),
        ("another-tool", "tool_identity_mismatch"),
        ("another-configuration", "configuration_mismatch"),
    ];
    let replayed = replayed();
    for (scenario, reason) in expected_reasons {
        let found = replayed
            .iter()
            .find(|(name, _, _, _)| name == scenario)
            .unwrap_or_else(|| panic!("the capture must carry {scenario}"));
        let receipt = found.1.as_ref().unwrap();
        let JsonValue::Array(reasons) = member(receipt, "reasons") else {
            panic!("reasons");
        };
        assert!(
            reasons.iter().any(|entry| entry.as_str() == Some(reason)),
            "{scenario}: expected {reason}, got {reasons:?}"
        );
    }
}

/// Trace: FR-065-AC-5
///
/// Passed evidence discharges only when the exact retained bytes verify and the
/// audit calls every owning obligation healthy.
#[test]
fn tc_455_only_intact_bytes_and_a_healthy_audit_discharge_a_proof() {
    let replayed = replayed();
    let by_name = |wanted: &str| {
        replayed
            .iter()
            .find(|(name, _, _, _)| name == wanted)
            .and_then(|(_, expected, _, _)| expected.clone())
            .unwrap_or_else(|| panic!("the capture must carry {wanted}"))
    };
    let good = by_name("everything-agrees");
    let JsonValue::Array(proofs) = member(&good, "proofs") else {
        panic!("proofs");
    };
    assert_eq!(text(&proofs[0], "outcome"), "valid");
    for scenario in [
        "output-bytes-differ",
        "audit-found-stale-evidence",
        "audit-found-a-suspect-link",
        "audit-found-vacuous-evidence",
        "audit-found-it-undischarged",
    ] {
        let receipt = by_name(scenario);
        assert_ne!(
            text(&receipt, "outcome"),
            "valid",
            "{scenario} must not verify"
        );
    }
}

/// Trace: FR-065-AC-6
///
/// Absent evidence and a producer that could not answer are *incomplete*, never
/// valid and never invalid: the difference between "this is wrong" and "this is
/// not finished" is the whole point of the third outcome.
#[test]
fn tc_455_absent_evidence_is_incomplete_rather_than_invalid() {
    let replayed = replayed();
    for scenario in [
        "attestation-not-selected",
        "attestation-not-retained",
        "output-missing",
        "producer-unavailable",
        "producer-did-not-compute",
        "audit-left-it-unevaluated",
        "no-audit",
    ] {
        let receipt = replayed
            .iter()
            .find(|(name, _, _, _)| name == scenario)
            .and_then(|(_, expected, _, _)| expected.clone())
            .unwrap_or_else(|| panic!("the capture must carry {scenario}"));
        assert_eq!(
            text(&receipt, "outcome"),
            "incomplete",
            "{scenario} must be incomplete"
        );
    }
}

/// Trace: FR-065-AC-7
///
/// Exactly one intact human `approved` event permits review validity. A missing
/// history is incomplete; a broken chain, an agent actor, a rejection and a
/// revision request are each their own refusal.
#[test]
fn tc_455_review_validity_needs_one_intact_human_approval() {
    let replayed = replayed();
    let review_reasons = |wanted: &str| -> Vec<String> {
        let receipt = replayed
            .iter()
            .find(|(name, _, _, _)| name == wanted)
            .and_then(|(_, expected, _, _)| expected.clone())
            .unwrap_or_else(|| panic!("the capture must carry {wanted}"));
        let review = member(member(&receipt, "checks"), "review");
        let JsonValue::Array(reasons) = member(review, "reasons") else {
            panic!("reasons");
        };
        reasons
            .iter()
            .map(|reason| reason.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(review_reasons("everything-agrees"), Vec::<String>::new());
    assert_eq!(
        review_reasons("no-decision-history"),
        ["event_chain_missing"]
    );
    assert_eq!(
        review_reasons("decision-chain-broken"),
        ["event_chain_invalid"]
    );
    assert_eq!(review_reasons("decision-rejected"), ["review_rejected"]);
    assert_eq!(
        review_reasons("decision-revise"),
        ["review_revision_requested"]
    );
    // An agent's decision is not a review decision: the candidate stops being a
    // match at all, so the history holds a decision event that decides nothing.
    assert_eq!(
        review_reasons("decision-by-an-agent"),
        ["decision_mismatch"]
    );
}

/// Trace: FR-065-AC-8
///
/// Incomplete or truncated impact evidence, and every unknown that is not
/// resolved, stay named in the receipt and force an incomplete verification.
#[test]
fn tc_455_impact_gaps_and_open_unknowns_are_named_and_force_incomplete() {
    let replayed = replayed();
    let receipt = replayed
        .iter()
        .find(|(name, _, _, _)| name == "impact-truncated-and-unknown-open")
        .and_then(|(_, expected, _, _)| expected.clone())
        .unwrap();
    let impact = member(member(&receipt, "checks"), "impact");
    assert_eq!(text(impact, "outcome"), "incomplete");
    let JsonValue::Array(reasons) = member(impact, "reasons") else {
        panic!("reasons");
    };
    let reasons: Vec<&str> = reasons.iter().map(|r| r.as_str().unwrap()).collect();
    assert_eq!(reasons, ["impact_truncated", "unresolved_unknown"]);
    let JsonValue::Array(unknowns) = member(&receipt, "unknowns") else {
        panic!("unknowns");
    };
    assert_eq!(unknowns.len(), 1, "the unknown must still be named");
    assert_eq!(text(&unknowns[0], "disposition"), "accepted");
}

/// Trace: FR-065-AC-9
///
/// An auditor's finding kind is retained byte-for-byte beside the reason it
/// maps to — including a kind this crate has never heard of, which maps to
/// `audit_finding` and is still reported under its own spelling.
#[test]
fn tc_455_audit_findings_are_retained_verbatim_beside_their_mapped_reason() {
    let replayed = replayed();
    let receipt = replayed
        .iter()
        .find(|(name, _, _, _)| name == "audit-found-something-unnamed")
        .and_then(|(_, expected, _, _)| expected.clone())
        .unwrap();
    let JsonValue::Array(proofs) = member(&receipt, "proofs") else {
        panic!("proofs");
    };
    let JsonValue::Array(findings) = member(&proofs[0], "audit_findings") else {
        panic!("audit_findings");
    };
    assert_eq!(findings.len(), 1);
    assert_eq!(text(&findings[0], "kind"), "invented-kind");
    assert_eq!(text(&findings[0], "obligation_id"), "FR-063-AC-1");
    let JsonValue::Array(reasons) = member(&proofs[0], "reasons") else {
        panic!("reasons");
    };
    assert!(
        reasons
            .iter()
            .any(|reason| reason.as_str() == Some("audit_finding"))
    );
}

/// Trace: FR-065-AC-11
///
/// A verification refuses in exactly the places the oracle refuses. The capture
/// holds one scenario the oracle could not answer at all — a genesis record
/// offered a parent, whose `parent_missing` reason makes the lineage check
/// declare itself `invalid` while its own reason implies `incomplete` — and
/// this crate reproduces that refusal rather than inventing a receipt the
/// retained implementation would never have written.
#[test]
fn tc_455_refusals_happen_where_the_oracle_refuses() {
    let replayed = replayed();
    let refusals: Vec<&String> = replayed
        .iter()
        .filter_map(|(name, _, refusal, _)| refusal.as_ref().map(|_| name))
        .collect();
    assert!(
        !refusals.is_empty(),
        "anti-vacuity floor: the capture must hold at least one refusal"
    );
    for (name, _, refusal, produced) in &replayed {
        assert_eq!(
            refusal.is_some(),
            produced.is_none(),
            "{name}: the oracle refused={}, this crate refused={}",
            refusal.is_some(),
            produced.is_none()
        );
    }
}

/// Trace: FR-100-AC-3
///
/// The refusal *classification* is the same on both sides. The oracle reads its
/// own thrown messages with three regexes; this crate reads the structure of
/// the refusal instead. Every captured scenario that reaches a classification
/// lands on the same reason in both, which is the only evidence that the
/// rewrite preserved the reading.
#[test]
fn tc_455_refusal_classification_agrees_with_the_oracle_everywhere() {
    let replayed = replayed();
    let mut classified = 0_usize;
    for (name, expected, _, produced) in &replayed {
        let (Some(expected), Some(produced)) = (expected, produced) else {
            continue;
        };
        for check in ["record", "lineage", "review", "impact"] {
            let left = member(member(expected, "checks"), check);
            let right = member(member(produced, "checks"), check);
            assert_eq!(
                canonical_bytes(left).unwrap(),
                canonical_bytes(right).unwrap(),
                "{name}: the {check} check classifies differently"
            );
            if !matches!(member(left, "reasons"), JsonValue::Array(reasons) if reasons.is_empty()) {
                classified += 1;
            }
        }
    }
    // 19 is the census of the committed capture, not a round number: every
    // check that names a reason in any captured receipt is counted, so a
    // scenario that stops reaching a classification drops the count and fails
    // here rather than passing vacuously.
    assert!(
        classified >= 19,
        "anti-vacuity floor: at least 19 non-empty classifications, saw {classified}"
    );
}

/// Trace: FR-065-AC-12
///
/// The receipt's own vocabulary. Every reason this family can record, and
/// every word appearing in any captured receipt, is checked against the claims
/// a receipt is not allowed to make: a receipt says what the evidence shows,
/// and never that anybody was authenticated, authorized or signed anything. An
/// actor that appears in a decision event is there as recorded attribution.
#[test]
fn tc_455_no_receipt_claims_more_than_integrity_and_attribution() {
    let claims = [
        "authentication",
        "authenticity",
        "authorization",
        "authorized",
        "non_repudiation",
        "nonrepudiation",
        "signature",
        "signed",
    ];
    for reason in Reason::ALL {
        assert!(
            !claims.contains(&reason.as_str()),
            "`{}` claims more than a verification can know",
            reason.as_str()
        );
    }

    let mut scanned = 0_usize;
    for (name, expected, _, _) in replayed() {
        let Some(expected) = expected else { continue };
        let text = String::from_utf8(canonical_bytes(&expected).unwrap()).unwrap();
        for claim in claims {
            assert!(
                !text.contains(claim),
                "{name}: a receipt names `{claim}`, which a receipt cannot establish"
            );
        }
        scanned += 1;
    }
    assert!(
        scanned >= 30,
        "anti-vacuity floor: at least 30 receipts scanned, saw {scanned}"
    );
}

/// Trace: FR-065-AC-2
///
/// The reason census. 34 of the 38 reasons the vocabulary declares are reached
/// by the captured scenarios, and every one of them is reached in a receipt
/// this crate produced rather than only in the oracle's. So this asserts the
/// exact figure rather than a floor, and a port that quietly stopped emitting
/// a reason fails here.
///
/// `parent_missing` is not reached because it is unreachable in any receipt
/// at all — not merely absent from this capture.
/// `tc_455_parent_missing_is_structurally_unreachable_in_this_crate` states
/// why over this crate's own code, so the figure stays falsifiable now that the
/// TypeScript it was originally explained by is gone (quoin#457).
///
/// `apparatus_touched`, `negative_control_uncaught` and `diff_missing`
/// (PLAT-964) are not
/// reached because the oracle capture predates them: no TypeScript ever
/// emitted them, so there is nothing here to replay. `tests/tc_964_apparatus.rs`
/// is the hand-built suite that reaches all three.
#[test]
fn tc_455_thirty_four_of_the_thirty_eight_reasons_are_reached() {
    let mut reached: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
    for (_, _, _, produced) in replayed() {
        let Some(produced) = produced else { continue };
        let mut collect = |value: &JsonValue| {
            if let JsonValue::Array(entries) = value {
                for entry in entries {
                    if let Some(spelling) = entry.as_str()
                        && let Some(reason) =
                            Reason::ALL.iter().find(|known| known.as_str() == spelling)
                    {
                        reached.insert(reason.as_str());
                    }
                }
            }
        };
        collect(member(&produced, "reasons"));
        for check in ["record", "lineage", "review", "impact"] {
            collect(member(
                member(member(&produced, "checks"), check),
                "reasons",
            ));
        }
        if let JsonValue::Array(proofs) = member(&produced, "proofs") {
            for proof in proofs {
                collect(member(proof, "reasons"));
            }
        }
    }
    assert_eq!(Reason::ALL.len(), 38, "the vocabulary is closed at 38");
    let unreached: Vec<&str> = Reason::ALL
        .iter()
        .map(|reason| reason.as_str())
        .filter(|spelling| !reached.contains(spelling))
        .collect();
    assert_eq!(
        unreached,
        vec![
            "parent_missing",
            "apparatus_touched",
            "negative_control_uncaught",
            "diff_missing"
        ],
        "exactly four reasons are unreached here, each for its own recorded reason"
    );
}

/// Trace: FR-065-AC-2
///
/// Why `parent_missing` is unreachable, stated over THIS implementation.
///
/// The census above records that one of the thirty-five reasons never appears
/// in a receipt. Until quoin#457 the recorded explanation was a defect in the
/// retained TypeScript (`DIVERGENCE.md` §4) — an explanation that stops being
/// checkable the moment that tree is deleted, which would leave "34 of 35" as
/// an assertion nothing could falsify.
///
/// The port inherited the defect faithfully, so it can be restated as a
/// property of this crate and checked here. Two facts in this crate contradict
/// each other:
///
/// - `Reason::ParentMissing` declares its precedence class `incomplete`;
/// - `verify::verify_change_assurance` builds the lineage check with
///   `Check::from_reasons(.., Outcome::Invalid)`, so ANY lineage reason makes
///   that check `invalid`.
///
/// A receipt carrying it therefore disagrees with its own precedence rule and
/// is refused by the self-consistency check before it is returned — for every
/// input that reaches the reason, not only for the one the capture holds. Both
/// halves are asserted, so a change to either one fails here and forces the
/// census figure and `DIVERGENCE.md` to be revisited together.
#[test]
fn tc_455_parent_missing_is_structurally_unreachable_in_this_crate() {
    // Half one: the reason's precedence class.
    assert!(
        Reason::ParentMissing.is_incomplete(),
        "parent_missing is an incompleteness, not an invalidity"
    );
    // The lineage reasons that are NOT incompletenesses do reach receipts, so
    // this is a statement about one reason and not about the whole check.
    for reachable in [
        Reason::ParentInvalid,
        Reason::ParentMismatch,
        Reason::RevisionGap,
    ] {
        assert!(!reachable.is_incomplete(), "{}", reachable.as_str());
    }

    // Half two: an input that reaches the reason is refused rather than
    // answered. A parent missing a required member is an absence, which is what
    // `verify::lineage::lineage_reason` maps to `parent_missing`.
    let scenario = scenarios()
        .into_iter()
        .find(|scenario| text(scenario, "name") == "full-chain")
        .expect("the capture carries the full-chain scenario");
    let mut input = verification_input(member(&scenario, "input"));
    assert!(
        !input.parents.is_empty(),
        "anti-vacuity floor: the scenario must offer a parent to damage"
    );
    let parent = input.parents.last_mut().expect("a parent");
    let JsonValue::Object(members) = parent else {
        panic!("a parent is an object")
    };
    assert!(
        members.remove("revision").is_some(),
        "the parent carried the member this test removes"
    );

    let refusal = verify_change_assurance(&input)
        .expect_err("a receipt carrying parent_missing contradicts its own precedence rule");
    assert!(
        format!("{refusal:?}").contains("outcome disagrees with reason precedence"),
        "refused for the recorded reason, not some other one: {refusal:?}"
    );
}
