// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A change touching a plan's protected apparatus is refused credit toward
//! that plan's objective, and a negative control the checker cannot evaluate
//! is reported rather than silently ignored (PLAT-964, quoin FR-111).
//!
//! There is no TypeScript oracle for this behaviour — `quoin-change-assurance`
//! never carried a diff or a plan link before this ticket — so every scenario
//! here is hand-built rather than replayed. The base record, parents,
//! selections, attestations, decision history and audits are lifted verbatim
//! from the oracle's own `everything-agrees` scenario
//! (`tests/fixtures/oracle.json`), which verifies `valid` with no reasons at
//! all: only [`VerificationInput::diff_paths`] and
//! [`VerificationInput::governing_plan`] vary between the tests below, so any
//! reason that appears is one this ticket's code produced.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use engineering_assurance::measurement::{
    ApparatusPath, NegativeControl, NegativeControlKind, NegativeControls, ProtectedApparatus,
};
use quoin_change_assurance::model::outcome::{Outcome, Reason};
use quoin_change_assurance::model::receipt::UnsealedReceipt;
use quoin_change_assurance::verify::input::GoverningPlan;
use quoin_change_assurance::verify::verify_change_assurance;

use crate::common::{member, oracle, section, text, verification_input};

/// The `everything-agrees` scenario's input, unlinked from any plan and
/// with no diff retained.
fn base_input() -> quoin_change_assurance::verify::input::VerificationInput {
    let captured = section(&oracle(), "verifications")
        .into_iter()
        .find(|entry| text(entry, "name") == "everything-agrees")
        .expect("the oracle capture still carries `everything-agrees`");
    verification_input(member(&captured, "input"))
}

fn apparatus(entries: &[&str]) -> ProtectedApparatus {
    ProtectedApparatus::new(
        entries
            .iter()
            .map(|entry| ApparatusPath::new(*entry).unwrap()),
    )
    .unwrap()
}

fn apparatus_edit_control() -> NegativeControls {
    NegativeControls::new([NegativeControl::new(
        NegativeControlKind::ApparatusEdit,
        "answer key edits",
    )
    .unwrap()])
    .unwrap()
}

fn plan_protecting(entries: &[&str]) -> GoverningPlan {
    GoverningPlan {
        protected_apparatus: Some(apparatus(entries)),
        negative_controls: None,
    }
}

/// Trace: FR-111-AC-1
///
/// A change whose diff touches the harness, the labels, the population or
/// the checker configuration is refused credit with `apparatus_touched`,
/// whichever of the four the plan protects and the diff reaches, and whether
/// or not the plan declares the `apparatus-edit` control.
#[test]
fn tc_964_001_a_diff_touching_any_protected_category_is_refused() {
    let categories: [(&str, &str, &str); 4] = [
        ("harness", "tests/harness/**", "tests/harness/run.rs"),
        ("labels", "eval/labels/**", "eval/labels/gold.jsonl"),
        ("population", "eval/population.json", "eval/population.json"),
        (
            "checker configuration",
            "checker/config.toml",
            "checker/config.toml",
        ),
    ];
    for (label, protected_entry, touched_path) in categories {
        for controls in [None, Some(apparatus_edit_control())] {
            let declared = controls.is_some();
            let mut input = base_input();
            input.diff_paths = Some(vec!["src/lib.rs".to_owned(), touched_path.to_owned()]);
            input.governing_plan = Some(GoverningPlan {
                protected_apparatus: Some(apparatus(&[protected_entry])),
                negative_controls: controls,
            });
            let receipt = verify_change_assurance(&input)
                .unwrap_or_else(|error| panic!("{label}: verification failed: {error}"));
            assert_eq!(
                receipt.body.reasons,
                vec![Reason::ApparatusTouched],
                "{label} (apparatus-edit declared: {declared})"
            );
            assert_eq!(
                receipt.body.outcome,
                Outcome::Invalid,
                "{label}: a touched apparatus refuses credit rather than leaving it incomplete"
            );
        }
    }
}

/// Trace: FR-111-AC-2
///
/// A diff that does not reach any protected entry passes: no
/// `apparatus_touched`, and the receipt is exactly what it would have been
/// with no plan linked at all.
#[test]
fn tc_964_002_a_diff_outside_the_protected_set_is_unaffected() {
    let unlinked = verify_change_assurance(&base_input()).expect("verification succeeds");
    let mut input = base_input();
    input.diff_paths = Some(vec![
        "src/lib.rs".to_owned(),
        "eval/labels-archive/gold.jsonl".to_owned(),
    ]);
    input.governing_plan = Some(plan_protecting(&["checker/config.toml", "eval/labels/**"]));
    let receipt = verify_change_assurance(&input).expect("verification succeeds");
    assert_eq!(receipt.body.outcome, Outcome::Valid);
    assert_eq!(receipt.body.reasons, Vec::new());
    // Everything the verification itself decided is unchanged from an
    // unlinked verification. Only the two PLAT-1015 members that record what
    // was SUPPLIED (not what mattered to the verdict) legitimately differ:
    // this scenario supplies a diff, `base_input()` supplies none.
    assert!(receipt.body.diff_paths_supplied);
    assert!(!unlinked.body.diff_paths_supplied);
    assert_eq!(
        UnsealedReceipt {
            diff_paths_supplied: false,
            governing_plan_id: None,
            ..receipt.body.clone()
        },
        unlinked.body
    );
}

/// Trace: FR-111-AC-3
///
/// A plan that declares a negative control this checker has no rule to
/// evaluate — `gain-within-noise`, here — is reported as
/// `negative_control_uncaught` rather than silently passing, and the
/// uncaught control alone leaves the outcome `incomplete` rather than
/// `invalid`.
#[test]
fn tc_964_003_an_uncatchable_negative_control_is_reported() {
    let mut input = base_input();
    let controls = NegativeControls::new([
        NegativeControl::new(NegativeControlKind::ApparatusEdit, "answer key edits").unwrap(),
        NegativeControl::new(NegativeControlKind::GainWithinNoise, "margin claims").unwrap(),
    ])
    .unwrap();
    input.diff_paths = Some(vec!["src/lib.rs".to_owned()]);
    input.governing_plan = Some(GoverningPlan {
        protected_apparatus: Some(apparatus(&["checker/config.toml"])),
        negative_controls: Some(controls),
    });
    let receipt = verify_change_assurance(&input).expect("verification succeeds");
    // The declared `apparatus-edit` control has a rule and the diff never
    // touched anything: it is caught, evaluated, and found clean. Only the
    // uncatchable control remains.
    assert_eq!(receipt.body.reasons, vec![Reason::NegativeControlUncaught]);
    assert_eq!(receipt.body.outcome, Outcome::Incomplete);
}

/// Trace: FR-111-AC-4
///
/// A plan that protects apparatus, checked with no diff retained, is
/// `diff_missing` and `incomplete` — never vacuously `valid` — while the same
/// missing diff with no plan linked changes nothing.
#[test]
fn tc_964_004_a_protecting_plan_with_no_retained_diff_is_incomplete() {
    let mut input = base_input();
    assert_eq!(input.diff_paths, None);
    input.governing_plan = Some(plan_protecting(&["checker/config.toml"]));
    let receipt = verify_change_assurance(&input).expect("verification succeeds");
    assert_eq!(receipt.body.reasons, vec![Reason::DiffMissing]);
    assert_eq!(receipt.body.outcome, Outcome::Incomplete);

    let unlinked = verify_change_assurance(&base_input()).expect("verification succeeds");
    assert_eq!(unlinked.body.outcome, Outcome::Valid);
    assert_eq!(unlinked.body.reasons, Vec::new());
}

/// Trace: FR-111-AC-5, TC-1915
///
/// A receipt sealed with both a governing plan and a retained diff records
/// `governing_plan_id` and `diff_paths_supplied: true`; the same base input
/// sealed with neither records `governing_plan_id: null` and
/// `diff_paths_supplied: false` as explicit fields — not silent absence —
/// so an auditor reading the sealed receipt afterward can tell "no plan/diff
/// was ever given" apart from "one was given and found nothing to refuse"
/// (PLAT-1015, closing the audit gap PLAT-997 left open).
#[test]
fn tc_1015_005_the_receipt_records_governing_plan_id_and_diff_paths_supplied() {
    let mut linked = base_input();
    linked.diff_paths = Some(vec!["src/lib.rs".to_owned()]);
    linked.governing_plan = Some(plan_protecting(&["checker/config.toml"]));
    linked.governing_plan_id = Some("MP-1015-LINKED".to_owned());
    let receipt = verify_change_assurance(&linked).expect("verification succeeds");
    assert_eq!(
        receipt.body.governing_plan_id.map(|id| id.to_string()),
        Some("MP-1015-LINKED".to_owned())
    );
    assert!(receipt.body.diff_paths_supplied);

    // Even a retained diff that touches nothing is still "supplied": the
    // member records whether a diff was given, not whether it mattered.
    let mut linked_untouched = base_input();
    linked_untouched.diff_paths = Some(Vec::new());
    let untouched_receipt =
        verify_change_assurance(&linked_untouched).expect("verification succeeds");
    assert!(untouched_receipt.body.diff_paths_supplied);
    assert_eq!(untouched_receipt.body.governing_plan_id, None);

    // The base fixture is unlinked and carries no diff: both members record
    // the omission explicitly rather than leaving it silent.
    let unlinked = verify_change_assurance(&base_input()).expect("verification succeeds");
    assert_eq!(unlinked.body.governing_plan_id, None);
    assert!(!unlinked.body.diff_paths_supplied);
}
