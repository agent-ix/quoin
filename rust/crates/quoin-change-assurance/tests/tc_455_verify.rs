// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verification of retained evidence (FR-063-AC-10, FR-065).
//!
//! Each case starts from the scenario in which everything agrees
//! (`common::base_input`) and varies one premise, then asserts the outcome and
//! the exact reasons the receipt names. A receipt carries no digest, so
//! nothing here recomputes one: the receipt is judged by what it says.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::path::{Path, PathBuf};

use quoin_change_assurance::model::outcome::{Outcome, Reason, outcome_for_reasons};
use quoin_change_assurance::model::receipt::VerificationReceipt;
use quoin_change_assurance::verify::input::{
    AuditReport, ReportFinding, ReportUnevaluated, RetainedAttestation, RetainedAudit, Selection,
    VerificationInput,
};
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{
    OUTPUT, attestation_text, base_input, history, json, member, record_text, replacing,
};

const KIND: &str = "change_assurance.review_decided";

fn verify(input: &VerificationInput) -> VerificationReceipt {
    verify_change_assurance(input).expect("a verification always produces a receipt")
}

fn reasons_of(receipt: &VerificationReceipt) -> Vec<&'static str> {
    receipt
        .reasons
        .iter()
        .map(|reason| reason.as_str())
        .collect()
}

/// An ix-flow event with every premise spelled out.
fn event(
    sequence: u64,
    actor_kind: &str,
    kind: &str,
    record_id: &str,
    revision: u64,
    decision: &str,
) -> String {
    format!(
        r#"{{
      "id": "event-{sequence}", "ts": "2026-08-31T00:00:00.000Z", "sequence": {sequence},
      "actor": {{"id": "reviewer-1", "kind": "{actor_kind}"}},
      "kind": "{kind}",
      "payload": {{"schema_version": 1, "record_id": "{record_id}", "revision": {revision}, "decision": "{decision}"}}
    }}"#
    )
}

/// The scenario with one decision event, as given.
fn with_decision(events: &[String]) -> VerificationInput {
    let mut input = base_input();
    input.decision_history = Some(history(events));
    input
}

/// The scenario's attestation with one top-level member replaced.
fn attestation_with(name: &str, value: JsonValue) -> VerificationInput {
    let mut input = base_input();
    input.attestations[0].attestation = replacing(&json(&attestation_text()), name, value);
    input
}

/// The scenario's attestation with one member of a nested object replaced.
fn attestation_nested(outer: &str, name: &str, value: JsonValue) -> VerificationInput {
    let attestation = json(&attestation_text());
    let nested = replacing(member(&attestation, outer), name, value);
    attestation_with(outer, nested)
}

/// The scenario extended with a second proof, `proof-0`, fully attested and
/// audited, so that two proofs are judged in `proof_id` order.
fn two_proofs() -> VerificationInput {
    let mut input = base_input();
    let record = json(&record_text());
    let definition = member(&record, "definition").clone();
    let JsonValue::Array(mut obligations) = member(&definition, "proof_obligations").clone() else {
        panic!("proof_obligations is not an array");
    };
    let mut second = obligations[0].as_object().unwrap().clone();
    second.set("proof_id", JsonValue::string("proof-0"));
    second.set(
        "obligation_ids",
        JsonValue::Array(vec![JsonValue::string("FR-063-AC-2")]),
    );
    // The record's collections are in identity order: `proof-0` precedes `proof-1`.
    obligations.insert(0, JsonValue::Object(second));
    let definition = replacing(
        &definition,
        "proof_obligations",
        JsonValue::Array(obligations),
    );
    input.record = replacing(&record, "definition", definition);

    let attestation = replacing(
        &replacing(
            &json(&attestation_text()),
            "attestation_id",
            JsonValue::string("att-2"),
        ),
        "proof_id",
        JsonValue::string("proof-0"),
    );
    input.attestations.push(RetainedAttestation {
        filed_as: "att-2".to_owned(),
        attestation,
        output: Some(OUTPUT.to_vec()),
    });
    input.selections.push(Selection {
        proof_id: "proof-0".to_owned(),
        attestation_id: "att-2".to_owned(),
    });
    input.audits.push(RetainedAudit {
        proof_id: "proof-0".to_owned(),
        report: AuditReport {
            findings: Vec::new(),
            healthy: vec!["FR-063-AC-2".to_owned()],
            unevaluated: Vec::new(),
        },
    });
    input
}

/// Trace: FR-065-AC-2
///
/// The three outcomes stay distinct and the precedence is fixed: any invalid
/// reason dominates incomplete, any incomplete reason dominates valid, and
/// only an empty reason set is valid. Stated over every pair of reasons, and
/// then through real receipts.
#[test]
fn tc_455_outcome_precedence_holds_over_every_reason_pair() {
    assert_eq!(outcome_for_reasons(&[]), Outcome::Valid);
    for left in Reason::ALL {
        let alone = outcome_for_reasons(&[*left]);
        assert_eq!(
            alone,
            if left.is_incomplete() {
                Outcome::Incomplete
            } else {
                Outcome::Invalid
            },
            "{left} alone"
        );
        for right in Reason::ALL {
            let both = outcome_for_reasons(&[*left, *right]);
            let expected = if left.is_incomplete() && right.is_incomplete() {
                Outcome::Incomplete
            } else {
                Outcome::Invalid
            };
            assert_eq!(both, expected, "{left} with {right}");
        }
    }

    let valid = verify(&base_input());
    assert_eq!((valid.outcome, valid.reasons.len()), (Outcome::Valid, 0));

    let mut truncated = base_input();
    truncated.record = replacing(
        &json(&record_text()),
        "impact_snapshot",
        replacing(
            member(&json(&record_text()), "impact_snapshot"),
            "truncated",
            JsonValue::Bool(true),
        ),
    );
    let incomplete = verify(&truncated);
    assert_eq!(incomplete.outcome, Outcome::Incomplete);

    let mut both = truncated.clone();
    both.decision_history = Some(history(&[event(
        1, "human", KIND, "change-1", 1, "rejected",
    )]));
    let invalid = verify(&both);
    assert_eq!(invalid.outcome, Outcome::Invalid);
    assert!(invalid.reasons.contains(&Reason::ImpactTruncated));
    assert!(invalid.reasons.contains(&Reason::ReviewRejected));

    // A receipt whose stated outcome disagrees with its own reasons is refused.
    let mut forged = incomplete.to_json().unwrap().as_object().unwrap().clone();
    forged.set("outcome", JsonValue::string("valid"));
    assert!(VerificationReceipt::from_json(&JsonValue::Object(forged)).is_err());
}

/// Trace: FR-065-AC-3
///
/// Exactly one proof receipt per reviewed proof, in `proof_id` order. A
/// missing selection is incomplete, a duplicate or unknown-proof selection is
/// invalid, and an unselected stored attestation changes nothing.
#[test]
fn tc_455_one_verdict_per_proof_and_an_unknown_selection_is_a_mismatch() {
    let both = two_proofs();
    let receipt = verify(&both);
    let ids: Vec<&str> = receipt
        .proofs
        .iter()
        .map(|proof| proof.proof_id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["proof-0", "proof-1"],
        "one verdict per proof, in id order"
    );
    assert_eq!(
        receipt.outcome,
        Outcome::Valid,
        "{:?}",
        reasons_of(&receipt)
    );

    let mut missing = two_proofs();
    missing
        .selections
        .retain(|selection| selection.proof_id != "proof-0");
    let receipt = verify(&missing);
    assert_eq!(receipt.outcome, Outcome::Incomplete);
    assert_eq!(receipt.proofs.len(), 2);
    assert_eq!(receipt.proofs[0].reasons, vec![Reason::AttestationMissing]);

    let mut duplicated = base_input();
    duplicated.selections.push(duplicated.selections[0].clone());
    let receipt = verify(&duplicated);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert!(receipt.reasons.contains(&Reason::AttestationSchemaInvalid));

    let mut unknown = base_input();
    unknown.selections.push(Selection {
        proof_id: "proof-9".to_owned(),
        attestation_id: "att-1".to_owned(),
    });
    let receipt = verify(&unknown);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert!(
        receipt.reasons.contains(&Reason::ProofIdMismatch),
        "an unknown selection must be named, got {:?}",
        reasons_of(&receipt)
    );
    assert_eq!(
        receipt.proofs.len(),
        1,
        "no verdict for a proof nobody reviewed"
    );

    let mut stray = base_input();
    stray.attestations.push(RetainedAttestation {
        filed_as: "att-9".to_owned(),
        attestation: replacing(
            &json(&attestation_text()),
            "attestation_id",
            JsonValue::string("att-9"),
        ),
        output: None,
    });
    assert_eq!(verify(&stray), verify(&base_input()));
}

/// Trace: FR-065-AC-4
///
/// Each binding between a proof obligation and an attestation - record id,
/// record revision, candidate revision, proof id, argv, working directory,
/// tool identity, configuration reference - invalidates the selection on its
/// own, and the receipt names the premise that failed.
#[test]
fn tc_455_each_binding_mismatch_is_named_on_its_own() {
    let cases: Vec<(&str, VerificationInput, Reason)> = vec![
        (
            "record id",
            attestation_with("record_id", JsonValue::string("change-2")),
            Reason::RecordBindingMismatch,
        ),
        (
            "record revision",
            attestation_with("record_revision", JsonValue::number(2.0).unwrap()),
            Reason::RecordBindingMismatch,
        ),
        (
            "candidate revision",
            attestation_with("candidate_revision", JsonValue::string("candidate-2")),
            Reason::CandidateRevisionMismatch,
        ),
        (
            "proof id",
            attestation_with("proof_id", JsonValue::string("proof-0")),
            Reason::ProofIdMismatch,
        ),
        (
            "argv",
            attestation_nested(
                "command",
                "argv",
                JsonValue::Array(vec![JsonValue::string("cargo"), JsonValue::string("check")]),
            ),
            Reason::CommandMismatch,
        ),
        (
            "working directory",
            attestation_nested("command", "working_directory", JsonValue::string("crates")),
            Reason::CommandMismatch,
        ),
        (
            "tool identity",
            attestation_nested("tool", "identity", JsonValue::string("rustc")),
            Reason::ToolIdentityMismatch,
        ),
        (
            "configuration",
            attestation_nested(
                "tool",
                "configuration_ref",
                JsonValue::string("config/other.toml"),
            ),
            Reason::ConfigurationMismatch,
        ),
    ];
    for (name, input, expected) in cases {
        let receipt = verify(&input);
        assert_eq!(receipt.outcome, Outcome::Invalid, "{name}");
        assert_eq!(reasons_of(&receipt), [expected.as_str()], "{name}");
    }
}

/// Trace: FR-065-AC-5
///
/// Passed evidence discharges a proof only when its retained output has the
/// declared size and the audit calls every owning obligation healthy. A
/// failed result, a wrong-sized output, and each auditor defect are invalid.
#[test]
fn tc_455_only_intact_output_and_a_healthy_audit_discharge_a_proof() {
    let good = verify(&base_input());
    assert_eq!(good.proofs[0].outcome, Outcome::Valid);

    let mut resized = base_input();
    resized.attestations[0].output = Some(b"short".to_vec());
    let receipt = verify(&resized);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert_eq!(reasons_of(&receipt), ["output_size_mismatch"]);

    let failed = attestation_with("result", JsonValue::string("failed"));
    let receipt = verify(&failed);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert_eq!(reasons_of(&receipt), ["result_failed"]);

    for (kind, reason) in [
        ("stale-evidence", "evidence_stale"),
        ("suspect-link", "evidence_suspect"),
        ("vacuous-evidence", "evidence_vacuous"),
        ("undischarged", "evidence_unrelated"),
    ] {
        let mut audited = base_input();
        audited.audits[0].report = AuditReport {
            findings: vec![ReportFinding {
                obligation: "FR-063-AC-1".to_owned(),
                kind: kind.to_owned(),
            }],
            healthy: Vec::new(),
            unevaluated: Vec::new(),
        };
        let receipt = verify(&audited);
        assert_eq!(receipt.outcome, Outcome::Invalid, "{kind} must not verify");
        assert!(
            receipt.reasons.iter().any(|found| found.as_str() == reason),
            "{kind}: expected {reason}, got {:?}",
            reasons_of(&receipt)
        );
    }

    let mut silent = base_input();
    silent.audits[0].report.healthy.clear();
    let receipt = verify(&silent);
    assert_eq!(
        receipt.outcome,
        Outcome::Invalid,
        "an obligation the audit does not call healthy is not discharged"
    );
}

/// Trace: FR-065-AC-6
///
/// Absent evidence, a producer that could not answer, and an audit that did
/// not evaluate are *incomplete*: never valid, and never invalid. The
/// difference between "this is wrong" and "this is not finished" is the point
/// of the third outcome.
#[test]
fn tc_455_absent_evidence_is_incomplete_rather_than_invalid() {
    let mut not_selected = base_input();
    not_selected.selections.clear();

    let mut not_retained = base_input();
    not_retained.attestations.clear();

    let mut output_missing = base_input();
    output_missing.attestations[0].output = None;

    let mut unevaluated = base_input();
    unevaluated.audits[0].report = AuditReport {
        findings: Vec::new(),
        healthy: Vec::new(),
        unevaluated: vec![ReportUnevaluated {
            obligation: "FR-063-AC-1".to_owned(),
        }],
    };

    let mut no_audit = base_input();
    no_audit.audits.clear();

    let cases: Vec<(&str, VerificationInput, &str)> = vec![
        (
            "attestation not selected",
            not_selected,
            "attestation_missing",
        ),
        (
            "attestation not retained",
            not_retained,
            "attestation_missing",
        ),
        ("output missing", output_missing, "output_missing"),
        (
            "producer unavailable",
            attestation_with("result", JsonValue::string("unavailable")),
            "result_unavailable",
        ),
        (
            "producer did not compute",
            attestation_with("result", JsonValue::string("not_computed")),
            "result_not_computed",
        ),
        (
            "audit left it unevaluated",
            unevaluated,
            "audit_not_evaluated",
        ),
        ("no audit", no_audit, "audit_not_evaluated"),
    ];
    for (name, input, reason) in cases {
        let receipt = verify(&input);
        assert_eq!(receipt.outcome, Outcome::Incomplete, "{name}");
        assert_eq!(reasons_of(&receipt), [reason], "{name}");
    }
}

/// Trace: FR-065-AC-7
/// Trace: FR-063-AC-10
///
/// Exactly one intact matching human `approved` event permits review
/// validity. A missing history is incomplete; a sequence gap, a repeat, a
/// mismatched run, kind, actor kind, record id or revision, a rejection and a
/// revision request each fail on their own.
#[test]
fn tc_455_review_validity_needs_one_intact_human_approval() {
    let review = |input: &VerificationInput| {
        let receipt = verify(input);
        (
            receipt.checks.review.outcome,
            receipt
                .checks
                .review
                .reasons
                .iter()
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>(),
        )
    };
    let approved = event(1, "human", KIND, "change-1", 1, "approved");
    assert_eq!(
        review(&with_decision(std::slice::from_ref(&approved))),
        (Outcome::Valid, vec![])
    );

    let mut no_history = base_input();
    no_history.decision_history = None;
    assert_eq!(
        review(&no_history),
        (Outcome::Incomplete, vec!["event_chain_missing"])
    );

    assert_eq!(
        review(&with_decision(&[])),
        (Outcome::Incomplete, vec!["decision_missing"])
    );
    assert_eq!(
        review(&with_decision(&[event(
            1, "human", "other", "change-1", 1, "approved"
        )])),
        (Outcome::Incomplete, vec!["decision_missing"]),
        "an event of another kind is not a decision"
    );

    assert_eq!(
        review(&with_decision(&[event(
            2, "human", KIND, "change-1", 1, "approved"
        )])),
        (Outcome::Invalid, vec!["event_chain_invalid"]),
        "a history that does not start at sequence one has a gap"
    );
    assert_eq!(
        review(&with_decision(&[
            approved.clone(),
            event(1, "human", "other", "change-1", 1, "approved"),
        ])),
        (Outcome::Invalid, vec!["event_chain_invalid"]),
        "a repeated sequence is not a chain"
    );

    assert_eq!(
        review(&with_decision(&[event(
            1, "human", KIND, "change-1", 1, "rejected"
        )])),
        (Outcome::Invalid, vec!["review_rejected"])
    );
    assert_eq!(
        review(&with_decision(&[event(
            1, "human", KIND, "change-1", 1, "revise"
        )])),
        (Outcome::Invalid, vec!["review_revision_requested"])
    );

    // An agent's decision is not a review decision, and a decision about
    // another record or revision identifies nothing here.
    for (why, mismatched) in [
        (
            "agent actor",
            event(1, "agent", KIND, "change-1", 1, "approved"),
        ),
        (
            "service actor",
            event(1, "service", KIND, "change-1", 1, "approved"),
        ),
        (
            "another record",
            event(1, "human", KIND, "change-2", 1, "approved"),
        ),
        (
            "another revision",
            event(1, "human", KIND, "change-1", 2, "approved"),
        ),
    ] {
        assert_eq!(
            review(&with_decision(&[mismatched])),
            (Outcome::Invalid, vec!["decision_mismatch"]),
            "{why} cannot approve"
        );
    }

    let mut other_run = base_input();
    other_run.decision_history = Some(json(&format!(
        r#"{{"run_id": "run-2", "events": [{approved}]}}"#
    )));
    assert_eq!(
        review(&other_run),
        (Outcome::Invalid, vec!["decision_mismatch"]),
        "a decision from another run cannot approve"
    );

    assert_eq!(
        review(&with_decision(&[
            approved.clone(),
            event(2, "human", KIND, "change-1", 1, "approved"),
        ])),
        (Outcome::Invalid, vec!["decision_mismatch"]),
        "two matching decisions are an ambiguity, not an approval"
    );
}

/// Trace: FR-065-AC-8
///
/// Incomplete or truncated impact evidence, and every unknown that is not
/// resolved, stay named in the receipt and force an incomplete verification.
#[test]
fn tc_455_impact_gaps_and_open_unknowns_are_named_and_force_incomplete() {
    let record = json(&record_text());
    let snapshot = replacing(
        &replacing(
            member(&record, "impact_snapshot"),
            "completeness",
            JsonValue::string("incomplete"),
        ),
        "truncated",
        JsonValue::Bool(true),
    );
    let mut unknown = quoin_store::JsonObject::new();
    unknown.set("id", JsonValue::string("unknown-1"));
    unknown.set("statement", JsonValue::string("what about it"));
    unknown.set("disposition", JsonValue::string("accepted"));
    unknown.set("owner", JsonValue::string("reviewer-1"));
    let definition = replacing(
        member(&record, "definition"),
        "unknowns",
        JsonValue::Array(vec![JsonValue::Object(unknown)]),
    );

    let mut input = base_input();
    input.record = replacing(
        &replacing(&record, "impact_snapshot", snapshot),
        "definition",
        definition,
    );
    let receipt = verify(&input);
    assert_eq!(receipt.checks.impact.outcome, Outcome::Incomplete);
    assert_eq!(
        receipt
            .checks
            .impact
            .reasons
            .iter()
            .map(|reason| reason.as_str())
            .collect::<Vec<_>>(),
        [
            "impact_incomplete",
            "impact_truncated",
            "unresolved_unknown"
        ]
    );
    assert_eq!(receipt.outcome, Outcome::Incomplete);
    assert_eq!(receipt.unknowns.len(), 1, "the unknown must still be named");
    assert_eq!(receipt.unknowns[0].id.as_str(), "unknown-1");
}

/// Trace: FR-065-AC-9
///
/// An auditor's finding kind and obligation id are retained byte for byte
/// beside the reason they map to - including a kind this crate has never
/// heard of, which maps to `audit_finding` and is still reported under its
/// own spelling. The source report is not changed.
#[test]
fn tc_455_audit_findings_are_retained_verbatim_beside_their_mapped_reason() {
    let mut input = base_input();
    input.audits[0].report = AuditReport {
        findings: vec![ReportFinding {
            obligation: "FR-063-AC-1".to_owned(),
            kind: "invented-kind".to_owned(),
        }],
        healthy: vec!["FR-063-AC-1".to_owned()],
        unevaluated: Vec::new(),
    };
    let before = input.clone();
    let receipt = verify(&input);
    assert_eq!(
        input, before,
        "verification must not change the auditor's result"
    );

    let proof = &receipt.proofs[0];
    assert_eq!(proof.audit_findings.len(), 1);
    assert_eq!(proof.audit_findings[0].kind.as_str(), "invented-kind");
    assert_eq!(
        proof.audit_findings[0].obligation_id.as_str(),
        "FR-063-AC-1"
    );
    assert!(proof.reasons.contains(&Reason::AuditFinding));
}

/// Trace: FR-065-AC-10
///
/// Input and selection permutations emit byte-identical canonical receipts.
#[test]
fn tc_455_permuted_inputs_emit_byte_identical_receipts() {
    let canonical =
        |input: &VerificationInput| canonical_bytes(&verify(input).to_json().unwrap()).unwrap();
    let original = two_proofs();
    let expected = canonical(&original);

    let mut selections = original.clone();
    selections.selections.reverse();
    assert_eq!(canonical(&selections), expected, "selection order");

    let mut attestations = original.clone();
    attestations.attestations.reverse();
    assert_eq!(canonical(&attestations), expected, "attestation order");

    let mut audits = original.clone();
    audits.audits.reverse();
    assert_eq!(canonical(&audits), expected, "audit order");

    let mut everything = original.clone();
    everything.selections.reverse();
    everything.attestations.reverse();
    everything.audits.reverse();
    assert_eq!(canonical(&everything), expected, "all orders together");

    // And the same input is the same bytes twice.
    assert_eq!(canonical(&original), expected);
}

/// Every `.rs` file under `src/verify/`, as `(file name, contents)`.
fn verify_sources() -> Vec<(String, String)> {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("verify");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&root).expect("src/verify is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("a source file"),
            ));
        }
    }
    files.sort();
    files
}

/// Trace: FR-065-AC-11
///
/// Verification runs nothing and writes nothing: no module under `verify/`
/// names a process, a file, the environment or a store write, and verifying
/// leaves its input exactly as it was.
#[test]
fn tc_455_verification_runs_nothing_and_writes_nothing() {
    let files = verify_sources();
    assert!(
        files.len() >= 6,
        "anti-vacuity floor: at least 6 verify modules, saw {}",
        files.len()
    );
    for (name, contents) in &files {
        for forbidden in [
            "std::fs",
            "std::process",
            "std::env",
            "Command::new",
            "write_change_record",
            "publish_record",
            "publish_attestation",
            "intake_attestation",
        ] {
            assert!(
                !contents.contains(forbidden),
                "verify/{name} names `{forbidden}`; verification runs and writes nothing"
            );
        }
    }

    let input = two_proofs();
    let before = input.clone();
    let _ = verify(&input);
    assert_eq!(input, before);
}
