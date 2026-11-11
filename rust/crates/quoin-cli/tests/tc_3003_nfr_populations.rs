// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1401: the NFR percentage metrics are measured over populations the
//! code enumerates, and each plan's declared set must equal that enumeration.
//! PLAT-1403: the span-breadth plan (MP-219) is measured the same way, by
//! replaying its frozen labels.
//!
//! Provenance: PLAT-1401, PLAT-1403

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod nfr_populations;

use ix_trace_rs::trace;
use nfr_populations::span_breadth::{self, Unavailable};
use nfr_populations::{Probed, assert_accepted, commands, manifests};
use quoin_measurement::verify::Reason;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("an isolated home is creatable")
}

/// NFR-003-M-1 and NFR-003-M-2: every command refuses an unknown flag out loud,
/// and none fails silently.
#[trace("FR-131-AC-9", "TC-3003", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3003_every_command_fails_a_bad_argument_out_loud() {
    let home = home();
    let (actionable, silent) = commands::bad_arguments(home.path());
    assert!(
        actionable.examined.contains("catalog") && actionable.examined.len() > 10,
        "{:?}",
        actionable.examined
    );
    assert_eq!(
        actionable.matched, actionable.examined,
        "a command fails quietly"
    );
    assert!(silent.matched.is_empty(), "{:?}", silent.matched);
    assert_accepted("MP-250", &actionable);
    assert_accepted("MP-251", &silent);
}

/// NFR-007-M-1: `ix-flow` absent and `ix-flow` failing both reach the caller as a
/// non-zero exit, for every workflow command.
#[cfg(unix)]
#[trace("FR-131-AC-10", "TC-3004", "NFR-007-M-1")]
#[test]
fn tc_3004_an_ix_flow_failure_is_a_non_zero_quoin_exit() {
    let home = home();
    let probed = commands::external_tool_failures(home.path());
    assert_eq!(probed.examined.len(), 4, "{:?}", probed.examined);
    assert_eq!(probed.matched, probed.examined);
    assert_accepted("MP-252", &probed);
}

/// NFR-008-M-1 and NFR-008-M-2: every corrupt manifest aborts assembly naming
/// its file and every candidate without one is skipped. Both populations are
/// probed and checked before either observation is recorded.
#[trace("FR-131-AC-11", "TC-3005", "NFR-008-M-1", "NFR-008-M-2")]
#[test]
fn tc_3005_corrupt_manifests_abort_and_missing_ones_are_skipped() {
    let harness = manifests::Harness::new();
    let corrupt = manifests::corrupt_manifests(&harness);
    let missing = manifests::missing_manifests(&harness);
    assert_eq!(
        corrupt.matched, corrupt.examined,
        "a corrupt manifest did not abort naming its file"
    );
    assert_eq!(
        missing.matched, missing.examined,
        "a missing manifest was not skipped"
    );
    assert_accepted("MP-253", &corrupt);
    assert_accepted("MP-254", &missing);
}

/// A scratch repository with one typed document, standing in for a label
/// file's repositories.
fn fixture_roots() -> BTreeMap<String, PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quoin-quire/tests/fixtures/repo");
    BTreeMap::from([("fixture".to_owned(), root)])
}

/// The replay states every label it was given as examined, and as matched only
/// those whose statement is unchanged and whose reviewed outcome the classifier
/// reproduces: exact spans, a reviewed refusal, never a drifted statement, a
/// missing row, or a refusal the classifier did not give.
#[trace("FR-131-AC-13", "TC-3300")]
#[test]
fn tc_3300_the_span_breadth_replay_states_what_it_examined_and_matched() {
    let label = |id: &str, row: &str, statement: &str, outcome: &str| {
        format!(
            r#"{{"id": "{id}", "repository": "fixture", "document": "spec/FR-001-example.md",
               "rowId": "{row}", "normalizedStatement": "{statement}", {outcome}}}"#
        )
    };
    let reproduced = label(
        "reproduced",
        "FR-001-AC-1",
        "every stored payload shall round-trip through the reader unchanged.",
        r#""expectedSpans": {
            "domain": {"start": 6, "end": 20, "text": "stored payload"},
            "oracle": {"start": 21, "end": 67, "text": "shall round-trip through the reader unchanged."}}"#,
    );
    let precondition_invented = reproduced
        .replace("reproduced", "precondition-invented")
        .replace(
            r#""oracle""#,
            r#""precondition": {"start": 0, "end": 5, "text": "Every"}, "oracle""#,
        );
    let drifted = label(
        "drifted",
        "FR-001-AC-1",
        "an earlier wording of the statement",
        r#""expectedRefusal": "span:refused-weak-boundary""#,
    );
    let wrong_refusal = label(
        "wrong-refusal",
        "FR-001-AC-2",
        "the system shall refuse a payload over its ceiling.",
        r#""expectedRefusal": "span:refused-weak-boundary""#,
    );
    let refused = label(
        "refused",
        "FR-001-AC-2",
        "the system shall refuse a payload over its ceiling.",
        r#""expectedRefusal": "example:specific-scenario""#,
    );
    let withdrawn = label(
        "withdrawn",
        "FR-001-AC-9",
        "a row the document no longer has",
        r#""expectedRefusal": "span:refused-weak-boundary""#,
    );
    let labels = format!(
        r#"{{"labels": [{reproduced}, {precondition_invented}, {drifted}, {wrong_refusal}, {refused}, {withdrawn}]}}"#
    );

    let probed = span_breadth::replay(&labels, &fixture_roots()).expect("the checkout is supplied");
    let ids = |ids: &[&str]| {
        ids.iter()
            .map(|id| (*id).to_owned())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        probed.examined,
        ids(&[
            "reproduced",
            "precondition-invented",
            "drifted",
            "wrong-refusal",
            "refused",
            "withdrawn"
        ])
    );
    assert_eq!(
        probed.matched,
        ids(&["refused"])
            .union(&ids(&["reproduced"]))
            .cloned()
            .collect()
    );
}

/// A label whose repository checkout is not supplied makes the replay refuse,
/// naming the repository, rather than examine the labels it can reach.
#[trace("FR-131-AC-13", "TC-3300")]
#[test]
fn tc_3300_a_replay_without_a_checkout_refuses_instead_of_examining_a_subset() {
    let only_quoin = BTreeMap::from([("quoin".to_owned(), nfr_populations::repository())]);
    let Unavailable(missing) = span_breadth::replay(&span_breadth::labels_json(), &only_quoin)
        .err()
        .expect("a replay lacking the quire-rs checkout must refuse");
    assert_eq!(missing, BTreeSet::from(["quire-rs".to_owned()]));
}

/// The full replay: every label of the frozen file, classified in the checkout
/// of its repository, is stated as examined, and intake and the checker read
/// the observation as stating exactly the declared set. Whether every label
/// still reproduces is the measurement; this test asserts only that the
/// observation states its population.
///
/// Needs a quire-rs checkout in `QUIRE_RS_CHECKOUT`; not part of the default
/// gate.
#[trace("FR-131-AC-13", "TC-3301")]
#[test]
#[ignore = "needs a quire-rs checkout in QUIRE_RS_CHECKOUT"]
fn tc_3301_the_frozen_labels_are_replayed_and_their_ids_stated() {
    let quire = std::env::var_os("QUIRE_RS_CHECKOUT").expect("QUIRE_RS_CHECKOUT names a checkout");
    let roots = BTreeMap::from([
        ("quoin".to_owned(), nfr_populations::repository()),
        ("quire-rs".to_owned(), PathBuf::from(quire)),
    ]);
    let probed: Probed =
        span_breadth::replay(&span_breadth::labels_json(), &roots).expect("both checkouts given");
    assert_eq!(probed.examined.len(), 40);
    let verdict = nfr_populations::verdict("MP-219", &probed);
    let population: Vec<_> = verdict
        .reasons
        .iter()
        .filter(|reason| {
            matches!(
                reason,
                Reason::PopulationUnstated
                    | Reason::PopulationMalformed
                    | Reason::PopulationPartial
                    | Reason::PopulationOutsideSet
                    | Reason::PopulationSetUndeclared
            )
        })
        .collect();
    assert!(population.is_empty(), "{:?}", verdict.reasons);
    eprintln!(
        "MP-219: {} of {} labels reproduced; verdict {:?} {:?}",
        probed.matched.len(),
        probed.examined.len(),
        verdict.verdict,
        verdict.reasons
    );
}
