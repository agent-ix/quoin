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
use nfr_populations::span_breadth::{self, Replay, Unavailable, Unmatched};
use nfr_populations::{
    assert_accepted, assert_population_stated, context, ix_flow, manifests, nfr003,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// NFR-003-M-1 and NFR-003-M-2: the populations are command x error class,
/// enumerated by the class modules; each plan declares exactly them, and intake
/// and the checker read the observation as stating that set. The per-class
/// tests (TC-3700..3706) own what each class reads.
#[trace("FR-131-AC-9", "TC-3003", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3003_the_nfr003_populations_are_stated_by_class() {
    let (actionable, silent) = nfr003::populations(&context::Context::new());
    assert!(actionable.examined.len() > 100, "{:?}", actionable.examined);
    for (plan, probed) in [("MP-250", &actionable), ("MP-251", &silent)] {
        assert_population_stated(plan, probed);
    }
}

/// NFR-007-M-1: every way `ix-flow` fails reaches the caller as a non-zero exit
/// naming what failed, for every workflow command.
#[cfg(unix)]
#[trace("FR-131-AC-10", "TC-3004", "NFR-007-M-1")]
#[test]
fn tc_3004_the_nfr007_population_is_stated_by_failure_mode() {
    let probed = ix_flow::external_tool_failures(&context::Context::new());
    assert!(probed.examined.len() >= 24, "{:?}", probed.examined);
    let unmatched: BTreeSet<_> = probed.examined.difference(&probed.matched).collect();
    assert!(
        unmatched.is_empty(),
        "an ix-flow failure is not surfaced: {unmatched:?}"
    );
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

/// A scratch copy of the quire fixture repository, standing in for a label
/// file's repositories, with a third criterion whose statement holds an
/// ellipsis.
fn fixture_roots() -> (tempfile::TempDir, BTreeMap<String, PathBuf>) {
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quoin-quire/tests/fixtures/repo");
    let copy = tempfile::tempdir().expect("a scratch directory is creatable");
    copy_tree(&source, copy.path());
    let document = copy.path().join("spec/FR-001-example.md");
    let mut text = std::fs::read_to_string(&document).expect("the fixture document is readable");
    text.push_str("| FR-001-AC-3 | The system shall refuse a payload\u{2026} over its ceiling. | Test (TC-003) |\n");
    std::fs::write(&document, text).expect("the fixture document is writable");
    let roots = BTreeMap::from([("fixture".to_owned(), copy.path().to_path_buf())]);
    (copy, roots)
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("the target directory is creatable");
    for entry in std::fs::read_dir(from).expect("the fixture directory is readable") {
        let entry = entry.expect("the entry is readable");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("the entry has a type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("the file copies");
        }
    }
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
               "rowId": "{row}", "statement": "{statement}", {outcome}}}"#
        )
    };
    let reproduced = label(
        "reproduced",
        "FR-001-AC-1",
        "Every stored payload shall round-trip through the reader unchanged.",
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
        "The system shall refuse a payload over its ceiling.",
        r#""expectedRefusal": "span:refused-weak-boundary""#,
    );
    let refused = label(
        "refused",
        "FR-001-AC-2",
        "The system shall refuse a payload over its ceiling.",
        r#""expectedRefusal": "example:specific-scenario""#,
    );
    let withdrawn = label(
        "withdrawn",
        "FR-001-AC-9",
        "a row the document no longer has",
        r#""expectedRefusal": "span:refused-weak-boundary""#,
    );
    // The label file's `normalizedStatement` rewrote the ellipsis to `...`; the
    // replay compares the label's own `statement`, so that rewrite is ignored.
    let ellipsis = label(
        "ellipsis",
        "FR-001-AC-3",
        "The system shall refuse a payload\u{2026} over its ceiling.",
        r#""normalizedStatement": "the system shall refuse a payload... over its ceiling.", "expectedRefusal": "example:specific-scenario""#,
    );
    let labels = format!(
        r#"{{"labels": [{reproduced}, {precondition_invented}, {drifted}, {wrong_refusal}, {refused}, {withdrawn}, {ellipsis}]}}"#
    );

    let (_scratch, roots) = fixture_roots();
    let Replay { probed, unmatched } =
        span_breadth::replay(&labels, &roots).expect("the checkout is supplied");
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
            "withdrawn",
            "ellipsis"
        ])
    );
    assert_eq!(probed.matched, ids(&["reproduced", "refused", "ellipsis"]));
    assert_eq!(
        unmatched,
        BTreeMap::from([
            ("drifted".to_owned(), Unmatched::StatementDrift),
            ("wrong-refusal".to_owned(), Unmatched::OutcomeDiffers),
            (
                "precondition-invented".to_owned(),
                Unmatched::OutcomeDiffers
            ),
            ("withdrawn".to_owned(), Unmatched::RowMissing),
        ])
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
    let Replay { probed, unmatched } =
        span_breadth::replay(&span_breadth::labels_json(), &roots).expect("both checkouts given");
    nfr_populations::assert_population_stated("MP-219", &probed);
    eprintln!("MP-219: unmatched {unmatched:?}");
}
