// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Nothing any of the three views reports depends on the order the bindings
//! arrive in.
//!
//! `canonicalizeBindings` (`load.ts:417`) sorts on six levels. Four are field
//! comparisons and `model::binding::canonicalize` reproduces them. The last
//! two compare `JSON.stringify(symbols)` and
//! `JSON.stringify(affirmations ?? [])`, and `JSON.stringify` emits an
//! object's members in JavaScript insertion order — for a `.passthrough()`
//! schema, the order the bytes on disk happened to use. Reproducing that would
//! mean carrying every undeclared member of every binding, and its file order,
//! to break a tie.
//!
//! The claim this crate makes instead is stronger than the tie-break: binding
//! order is not observable at all. Fan-out accumulates into sets, churn keys
//! its events by `(obligation, who, commit, note)` and sorts the suites it
//! unions, and change-impact reads only `suite` from the first binding of each
//! `(obligation, suite)` pair. So this test takes every case in the golden
//! corpus, permutes its bindings three ways, and asserts all three views come
//! out byte-identical each time.
//!
//! That is why this is not an entry in `tc_385_parity.rs`'s
//! `DECLARED_DIVERGENCES`: a divergence with no observable effect could never
//! fire, and a declaration that cannot fire is a claim about nothing. The
//! property is asserted here instead, where it can fail.
//!
//! Trace: FR-062-AC-1
//!
//! Provenance: quoin#385

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod support;

use quoin_graph_analysis::model::binding::{Binding, BindingInput};
use quoin_graph_analysis::{
    ArtifactId, GraphAnalysis, GraphAnalysisInput, RelationKind, analyze_change_impact,
    analyze_churn, analyze_fan_out, render_graph_analysis, render_graph_analysis_json,
};
use serde_json::Value;

/// The count below which this property is asserted over nothing.
const PERMUTED_FLOOR: usize = 15;

/// The three views of one input, as `(view, json, markdown)`.
fn views(
    id: &str,
    input: &GraphAnalysisInput,
    case: &Value,
) -> Vec<(&'static str, String, String)> {
    let requested: Vec<ArtifactId> = case["requested"]
        .as_array()
        .expect("requested is an array")
        .iter()
        .map(|value| ArtifactId::new(value.as_str().expect("an artifact id")))
        .collect();
    let relations: Option<Vec<RelationKind>> = case["relations"].as_array().map(|kinds| {
        kinds
            .iter()
            .map(|value| RelationKind::new(value.as_str().expect("a relation kind")))
            .collect()
    });
    let impact = analyze_change_impact(input, &requested, relations.as_deref())
        .unwrap_or_else(|error| panic!("{id}: change impact is computable: {error}"));
    [
        ("fan-out", GraphAnalysis::from(analyze_fan_out(input))),
        ("churn", GraphAnalysis::from(analyze_churn(input))),
        ("change-impact", GraphAnalysis::from(impact)),
    ]
    .into_iter()
    .map(|(view, analysis)| {
        let json = render_graph_analysis_json(&analysis)
            .unwrap_or_else(|error| panic!("{id}/{view}: canonicalizable: {error}"));
        (view, json, render_graph_analysis(&analysis))
    })
    .collect()
}

/// The three permutations, named so a failure says which one broke it.
fn permutations(bindings: &[Binding]) -> Vec<(&'static str, Vec<Binding>)> {
    let reversed: Vec<Binding> = bindings.iter().rev().cloned().collect();
    let mut rotated = bindings.to_vec();
    if !rotated.is_empty() {
        rotated.rotate_left(1);
    }
    // Every other binding first, then the rest: a permutation that is neither
    // the reverse nor a rotation, so a comparator that happens to be stable
    // under both is still moved.
    let mut interleaved: Vec<Binding> = bindings.iter().step_by(2).cloned().collect();
    interleaved.extend(bindings.iter().skip(1).step_by(2).cloned());
    vec![
        ("reversed", reversed),
        ("rotated", rotated),
        ("interleaved", interleaved),
    ]
}

/// Permuting a case's bindings changes none of its three views.
///
/// Trace: FR-062-AC-1
#[test]
fn tc_385_040_permuting_the_bindings_changes_no_view() {
    let corpus = support::corpus();
    let mut permuted = 0usize;
    for case in corpus["cases"].as_array().expect("cases is an array") {
        let id = case["id"].as_str().expect("a case id");
        let input = support::input(case);
        let BindingInput::Available(ref bindings) = input.bindings else {
            continue;
        };
        if bindings.len() < 2 {
            continue;
        }
        let expected = views(id, &input, case);
        // Two of the three permutations coincide with the original at some
        // lengths — `interleaved` is the identity on two bindings. A
        // permutation that moved nothing tests nothing, so it is dropped
        // rather than counted, and a case left with none is a failure.
        let moved: Vec<(&str, Vec<Binding>)> = permutations(bindings)
            .into_iter()
            .filter(|(_, permutation)| permutation != bindings)
            .collect();
        assert!(
            !moved.is_empty(),
            "{id}: no permutation of {} bindings moved anything, so this case tests nothing",
            bindings.len()
        );
        for (name, permutation) in moved {
            let shuffled = GraphAnalysisInput {
                bindings: BindingInput::Available(permutation),
                ..input.clone()
            };
            assert_eq!(
                views(id, &shuffled, case),
                expected,
                "{id}: the {name} binding order produced a different report. Binding order is \
                 not observable, which is why the retained tie-break on JSON.stringify is not \
                 reproduced; if it has become observable, that reasoning no longer holds."
            );
            permuted += 1;
        }
    }
    assert!(
        permuted >= PERMUTED_FLOOR,
        "anti-vacuity floor: at least {PERMUTED_FLOOR} permuted runs expected, saw {permuted} — \
         a property asserted over no input is not asserted"
    );
}
