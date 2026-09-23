// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The OTHER six questions in PLAT-839's gap-analysis battery, graded.
//!
//! `live_gap_semantic.rs` graded exactly one of the seven questions the lens
//! asks — `assertion_vacuous` — against mutation truth, and reported a GO on
//! that basis. The other six (`test_asserts_intent`,
//! `tests_only_its_own_mock`, `code_implements_intent`,
//! `code_exceeds_requirement`, `divergence_kind`, `severity`) were asked in
//! the same live calls and never graded against anything. This file grades
//! as many of them as honestly admit ground truth, and says plainly which do
//! not. It is evaluation-only, behind the non-default `live-api` feature, and
//! changes nothing in `quoin-jev`'s `src/` or in `skills/gap-analysis/`:
//!
//! ```bash
//! cd rust && cargo test -p quoin-jev --features live-api --test live_gap_battery -- --nocapture
//! ```
//!
//! # The ground truth, per question
//!
//! **Targeted mutation, not the coarse stub.** MP-232's mutation replaced the
//! whole covered symbol with a default-returning body. That answers "would
//! this test notice if the implementation vanished", which is
//! `assertion_vacuous`'s question and nothing else's. The mutants in
//! `fixtures/gap-battery-mutants.json` are *targeted*: each contradicts (or
//! adds to) the SPECIFIC behaviour its triple's requirement states, leaving
//! the rest of the symbol intact. Each was applied as an exact source
//! substitution in a scratch, uncommitted edit of this worktree; exactly the
//! one owning test was rerun; the pass/fail was recorded; the edit was
//! reverted. Nothing mutated is committed — the fixture holds the
//! substitution text and the outcome, not a mutated tree.
//!
//! **A static check, not a second opinion.**
//! `gap_battery_support::asserts_only_its_own_setup` answers
//! `tests_only_its_own_mock` from the test's own source text. Its rule is
//! stated on the function. The lens is never consulted about it.
//!
//! **Constructed rows, said to be constructed.** The static check finds ZERO
//! mock-only tests among the corpus's 29 real ones — a true finding about
//! this corpus, and also the reason the question cannot be scored on it: a
//! constant "no" predictor is already perfect. Eight constructed rows
//! (`MOCK-01`..`MOCK-08`, `fixtures/gap-battery-mockonly.json`) supply the
//! positive class, exactly as `GAP-28`/`GAP-29` supplied one for
//! `assertion_vacuous`. Both the all-real result and the balanced result are
//! reported; neither is presented as the other.
//!
//! # Pre-registered bars, fixed and committed before the first live call
//!
//! Every bar below compares against a constant predictor computed FROM THE
//! DATA, never a written-down guess, and every population's minimum size is
//! asserted rather than skipped past — a population too small to measure is
//! a failure to report, not a measurement to omit.
//!
//! **Bar 1 — `test_asserts_intent`.** Over one row per violating mutant
//! (shown the UNMUTATED triple), truth = the owning test failed under that
//! mutant. Agreement must exceed the constant predictor. The sample must
//! carry both classes — at least one mutant caught and at least one that
//! survived — or the question is not measurable on it and this file says so
//! instead of reporting a number.
//!
//! **Bar 2 — `code_implements_intent`.** Over paired rows: the shipped body
//! (truth = yes) and the violating-mutant body (truth = no) for every mutant
//! the owning test caught. A mutant the test did NOT catch is excluded from
//! the negative class and reported separately — with no test failure, this
//! file has no mechanical proof the mutated body violates the requirement,
//! and MP-232's own exclusion discipline says such a row is excluded, never
//! defaulted to a class. Agreement must exceed the constant predictor (50%
//! by construction on a balanced population) and BOTH classes must have
//! non-zero recall.
//!
//! **Bar 3 — `tests_only_its_own_mock`.** Over the 29 real rows plus the 8
//! constructed ones, truth = the static check. Agreement must exceed the
//! constant predictor and the mock-only class must have non-zero recall. The
//! all-real 29-row specificity is reported alongside, ungated, because a
//! constant predictor scores 100% there.
//!
//! **Bar 4 — `code_exceeds_requirement`.** Over paired rows: the shipped body
//! (truth = no) and the additive-mutant body (truth = yes) for every additive
//! mutant the owning test did NOT catch — a mutant that changed asserted
//! behaviour is not purely additive and is excluded. Agreement must exceed
//! the constant predictor and both classes must have non-zero recall. **The
//! negative class here is weaker than the positive one and is labelled as
//! such:** that the shipped, reviewed, `Trace:`-tagged body implements
//! nothing beyond its requirement is an assumption about this corpus, not a
//! fact the compiler checked.
//!
//! **Bar 5 — `divergence_kind`.** Two predictors over the same rows: the
//! six-way label asked for directly, and the label DERIVED from the same
//! response's own `noul` answers by the rule in
//! `BatteryAnswers::derived_divergence_kind`, fixed here before the first
//! live call. PLAT-838 took a sibling lens from 62.5% to 93.8% with exactly
//! that move. Both are reported; the bar is that the better of the two
//! exceeds the majority-label constant predictor. Truth covers four of the
//! six labels — `aligned`, `test_weaker_than_requirement`,
//! `code_short_of_requirement`, `code_exceeds_requirement`. No row in this
//! population is mechanically `test_stronger_than_requirement` or
//! `requirement_ambiguous`, so this file does not claim to have tested them.
//!
//! **Bar 6 — `severity`.** No label, because there is no mechanical fact a
//! rubric point corresponds to. What is measured is discrimination over the
//! Bar 2 pairs: the share where the confirmed-defective mutant is scored
//! strictly more severe than the shipped body. A constant answer wins no
//! pair, so the bar is 50% — a coin flip — and this file does not report the
//! number as an accuracy.
//!
//! # Measured — three live runs, every bar reported, two of six not held
//!
//! Stable across three consecutive `FullBattery` passes (56 requests each,
//! ~8s wall clock). **Bars 2, 4, 5 and 6 held in all three runs; Bars 1 and 3
//! failed in all three.** A strong number on one question says nothing about
//! another, and this file asserts every bar rather than the ones that passed.
//!
//! | Bar | Measured (runs 1/2/3) | Constant predictor | Held |
//! | --- | --- | --- | --- |
//! | 1 `test_asserts_intent` | 45.5% / 45.5% / 45.5% | `always yes` 81.8% | **no** |
//! | 2 `code_implements_intent` | 66.7% / 66.7% / 66.7% | `always yes` 50.0% | yes |
//! | 3 `tests_only_its_own_mock` | 73.0% / 70.3% / 67.6% | `always no` 78.4% | **no** |
//! | 4 `code_exceeds_requirement` | 62.5% / 62.5% / 56.2% | `always yes` 50.0% | yes |
//! | 5 `divergence_kind`, derived | 46.4% / 46.4% / 42.9% | majority label 32.1% | yes |
//! | 5 `divergence_kind`, asked | 39.3% / 35.7% / 35.7% | majority label 32.1% | yes |
//! | 6 `severity` discrimination | 100% / 88.9% / 88.9% of 9 pairs | coin flip 50% | yes |
//!
//! Bar 1 is the sharpest negative. On the eleven violating mutants the lens
//! was shown the SHIPPED triple and asked whether its test asserts the
//! requirement's stated behaviour; it was right 5 times out of 11, said yes to
//! both tests that in fact missed their targeted mutant, and said no to four
//! tests that caught theirs. Saying "yes" to everything would have scored
//! 81.8%.
//!
//! Bar 3's failure has a second number that matters more than the headline.
//! The static check finds ZERO mock-only tests among the 29 real ones, and the
//! lens called 10, 11 and 12 of them mock-only across the three runs — a false
//! positive on roughly a third of a corpus with no positives in it. It did
//! find all eight constructed mock-only rows every time (recall 100%), so the
//! failure is specificity, not blindness.
//!
//! Bar 5 reproduces PLAT-838's lesson on a second lens: deriving the six-way
//! label from the response's own `noul` answers beat asking for it directly in
//! all three runs (46.4/46.4/42.9 vs. 39.3/35.7/35.7). Both beat the majority
//! label, so the bar held either way, and neither is near usable accuracy.
//!
//! One shape worth naming for whoever picks this up: for several triples the
//! `divergence_kind` answer did not move at all between the shipped body and
//! its mutant (`GAP-09`, `GAP-12`, `GAP-20`, `GAP-22` answer the same label for
//! the original, the violating mutant and the additive mutant). The `noul`
//! answers underneath it do move — Bar 2 and Bar 6 both separate the classes —
//! so the closed six-way choice is losing information the open questions carry.
//!
//! # `battery-v1` (PLAT-979): Bars 1 and 3, pre-registered before its first live call
//!
//! Error analysis first. The per-row answers come from a fresh replication of
//! the v0 `FullBattery` pass, and the row outcomes match the three runs above
//! (Bar 1 45.5%, Bar 3 70.3%). Each disagreement was classified as one of
//! wording, missing sub-question, bad option list, threshold or bad label, in
//! that priority order.
//!
//! **Bar 1, 6 disagreements: 4 bad label, 2 wording.** All four false
//! negatives (`GAP-09`, `GAP-12`, `GAP-20`, `GAP-22`) are FR-101 triples whose
//! targeted mutant contradicts behaviour the covered SYMBOL documents, never
//! the cited acceptance criterion. Each rationale names no requirement, and
//! the corpus's own `note` on each, written before any live call, records the
//! trace tag as mismatched to the test. The owning test killing that mutant
//! does not show that the test asserts the REQUIREMENT's stated behaviour.
//! The lens said no on all four, and against the question as asked, it was
//! right. The two false positives (`GAP-05` P=0.53, `GAP-25` P=0.63) are
//! partial tests. Each asserts some output but not the clause its mutant
//! breaks. "Rather than merely invoking it" sets the bar at any assertion at
//! all, which a partial test clears.
//!
//! **Bar 3, 11 disagreements, all false positives: 10 wording, 1 bad label.**
//! "Values the test itself configured" is ambiguous, because every test writes
//! both its inputs and its expected values. `GAP-08`, for example, asserts
//! enum variants a classifier returns and was called mock-only at P=0.73. One
//! row, `GAP-15`, is a plausible bad label: its corpus note says the
//! `MethodCatalogFixture` hands back a canned payload the test then asserts.
//! That fixture's body is outside the test source the static check reads, and
//! outside the state the lens is shown. It is reported here and NOT
//! relabelled. The static check stays the ground truth as pre-registered.
//!
//! Two changes, fixed here and committed before the first `FullBatteryV1` call:
//!
//! 1. **Bad label, Bar 1 only.** P1 admits only a violating mutant that
//!    contradicts behaviour its triple's cited requirement states, which is
//!    MP-233's own definition of P1, applied by
//!    `gap_battery_support::contradicts_cited_requirement`. That rule drops
//!    the four FR-101 mutants and leaves 7 rows, below this file's own minimum
//!    of 8. So one new targeted mutant is added for `GAP-07` (FR-062-AC-9), in
//!    `fixtures/gap-battery-mutants-v1-additions.json`. `GAP-07` is the only
//!    remaining real triple whose cited requirement text states the behaviour
//!    its test exercises. `GAP-01` and `GAP-27` say in their own `ac_text` that
//!    no criterion covers the behaviour. The mutant targets the FR statement's
//!    own SHALL clause, "name the failed input". It was applied, run against
//!    the one owning test, and recorded before any v1 call, the same way as
//!    the others. The v0 fixture and the v0 test's population are unchanged.
//! 2. **Wording, Bars 1 and 3 only.** `test_asserts_intent` is asked as the
//!    counterfactual its ground truth checks: would the test fail if the code
//!    stopped doing the requirement's stated thing? `tests_only_its_own_mock`
//!    is asked as the definition its static check applies: no assertion
//!    states an expected value absent from the test's own inputs. The other
//!    five questions are word-for-word unchanged. The texts live in
//!    `gap_semantic_support` as `Variant::FullBatteryV1`.
//!
//! **Caveat on change 2 for Bar 3.** The reworded question states the static
//! check's rule. If the lens now agrees, that shows it can apply a rule a
//! twenty-line text scan already applies without it. That is not evidence the
//! lens adds anything over the scan, and it will not be reported as such.
//!
//! **Bars, `battery-v1`.** One pass, with `FullBattery` (v0) and
//! `FullBatteryV1` asked over the same rows, so the label fix and the wording
//! fix separate:
//!
//! - **Bar 1-v1**: `FullBatteryV1`'s `test_asserts_intent` over the corrected
//!   P1 (at least 8 rows, both classes present) must exceed the constant
//!   predictor computed over those rows.
//! - **Bar 3-v1**: `FullBatteryV1`'s `tests_only_its_own_mock` over the same 37
//!   rows as v0 must exceed the constant predictor, and mock-only recall must
//!   stay above 0.
//!
//! Reported and ungated: v0 wording over the corrected P1, which isolates the
//! label fix; v0 wording over the original P1, which replicates v0; and the
//! v1 false-positive count over the 29 real rows. A threshold change was NOT
//! tried. With 2 negative rows in Bar 1, any threshold would be fitted to
//! those two answers.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::too_many_lines,
    reason = "the gate is one measurement with six pre-registered bars; splitting it \
              would put the bars somewhere other than where the population is built"
)]

mod gap_battery_support;
mod gap_semantic_support;

use std::collections::BTreeMap;

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use gap_battery_support::{
    BatteryAnswers, BinaryRow, LabelRow, Mutant, MutantClass, SeverityPair, answers,
    asserts_only_its_own_setup, binary_report, constant_binary_baseline, constant_label_baseline,
    constructed_mock_only, contradicts_cited_requirement, label_agreement, mutants,
    mutants_v1_additions, mutated, severity_discrimination, tally_binary,
};
use gap_semantic_support::{GapTriple, Variant, build_request, corpus};
use quoin_jev::JevErrorCode;

/// Builds a client against the real service. Panics rather than skipping when
/// no key resolves — same discipline as the sibling live files: a silent skip
/// is how a suite reports green over a measurement that never ran.
fn live_client() -> Client {
    let config = quoin_jev::config::resolve(&Process).unwrap_or_else(|error| {
        assert_eq!(
            error.code,
            JevErrorCode::MissingKey,
            "unexpected config failure: {error}"
        );
        panic!(
            "the `live-api` feature is on but no API key resolved. This test \
             does not skip. Set TYPESAFE_API_KEY, or drop --features live-api."
        );
    });
    quoin_jev::client::production(config).expect("builds a real transport")
}

/// Every distinct triple variant this file asks about, deduplicated by id, so
/// a triple that appears in two populations costs one request and not two.
fn request_targets() -> Vec<GapTriple> {
    let triples = corpus();
    let mut by_id: BTreeMap<String, GapTriple> = BTreeMap::new();
    for triple in &triples {
        by_id.insert(triple.id.clone(), triple.clone());
    }
    for mutant in mutants() {
        let row = mutated(&triples, &mutant);
        by_id.insert(row.id.clone(), row);
    }
    for row in constructed_mock_only() {
        by_id.insert(row.id.clone(), row);
    }
    by_id.into_values().collect()
}

/// One `variant` request per target, answers indexed by row id.
async fn ask_all(
    client: &Client,
    targets: &[GapTriple],
    variant: Variant,
) -> BTreeMap<String, BatteryAnswers> {
    let mut out = BTreeMap::new();
    for triple in targets {
        let response = client
            .system_one(build_request(triple, variant))
            .await
            .unwrap_or_else(|error| panic!("{}: {error}", triple.id));
        out.insert(triple.id.clone(), answers(&response));
    }
    out
}

/// A binary row for `key` over `id`, or an unanswered one when the lens left
/// that question out.
fn binary_row(
    said: &BTreeMap<String, BatteryAnswers>,
    id: &str,
    key: &str,
    truth: bool,
) -> BinaryRow {
    let answer = said.get(id);
    BinaryRow {
        id: id.to_owned(),
        truth,
        predicted_prob: answer.and_then(|answer| answer.p(key)),
        predicted: answer.and_then(|answer| answer.says(key)),
    }
}

/// Records one bar's verdict instead of panicking on the spot.
///
/// A bar that fails must not stop the bars after it from being MEASURED. The
/// first live run of this file failed Bar 1 and, with a plain `assert!`, that
/// left the other five unmeasured — which reads as "we do not know" but is
/// easily mistaken for "they were fine". Every bar is evaluated, every number
/// is printed, and [`Bars::settle`] fails the test at the end naming every
/// bar that did not hold.
#[derive(Debug, Default)]
struct Bars {
    failed: Vec<String>,
}

impl Bars {
    /// Records `bar` as failed when `held` is false.
    fn check(&mut self, held: bool, bar: &str, why: &str) {
        println!("{} {bar}: {why}", if held { "PASS" } else { "FAIL" });
        if !held {
            self.failed.push(format!("{bar}: {why}"));
        }
    }

    /// One question's two standing conditions: beat the constant predictor,
    /// and answer both classes at least once.
    fn beats_constant(&mut self, bar: &str, rows: &[BinaryRow], both_classes: bool) {
        let (stats, unanswered) = tally_binary(rows);
        let (label, baseline) = constant_binary_baseline(rows);
        let Some(agreement) = stats.accuracy() else {
            self.check(
                false,
                bar,
                &format!("the lens answered none of {} rows", rows.len()),
            );
            return;
        };
        self.check(
            agreement > baseline,
            bar,
            &format!(
                "agreement {agreement:.1}% vs. constant predictor `{label}` \
                 {baseline:.1}% over {} rows ({unanswered} unanswered)",
                rows.len()
            ),
        );
        if both_classes {
            self.check(
                stats.recall().is_some_and(|value| value > 0.0),
                bar,
                &format!("yes-class recall {:?}", stats.recall()),
            );
            self.check(
                stats.no_defect_recall().is_some_and(|value| value > 0.0),
                bar,
                &format!(
                    "no-class recall {:?} (a lens that answers one way for \
                     everything fails here by construction)",
                    stats.no_defect_recall()
                ),
            );
        }
    }

    /// Fails the test naming every bar that did not hold.
    fn settle(self) {
        assert!(
            self.failed.is_empty(),
            "{} pre-registered bar(s) did not hold:\n  - {}",
            self.failed.len(),
            self.failed.join("\n  - ")
        );
    }
}

/// **The gate.** Provenance: PLAT-839. Grades the six ungraded questions
/// against the ground truth described in this file's module doc, and checks
/// every pre-registered bar.
#[tokio::test]
async fn the_rest_of_the_battery_beats_doing_nothing() {
    let client = live_client();
    let triples = corpus();
    let all_mutants = mutants();
    let targets = request_targets();
    let said = ask_all(&client, &targets, Variant::FullBattery).await;
    let mut bars = Bars::default();

    // ---- Bar 1: test_asserts_intent -------------------------------------
    let violating: Vec<_> = all_mutants
        .iter()
        .filter(|mutant| mutant.class == MutantClass::Violating)
        .collect();
    assert!(
        violating.len() >= 8,
        "too few violating mutants to measure test_asserts_intent: {}",
        violating.len()
    );
    let asserts_intent: Vec<BinaryRow> = violating
        .iter()
        .map(|mutant| {
            binary_row(
                &said,
                &mutant.base_id,
                "test_asserts_intent",
                mutant.killed_by_owning_test,
            )
        })
        .collect();
    println!(
        "{}",
        binary_report(
            "Bar 1 — test_asserts_intent (targeted mutation)",
            &asserts_intent
        )
    );
    let survived = asserts_intent.iter().filter(|row| !row.truth).count();
    assert!(
        survived > 0 && survived < asserts_intent.len(),
        "the targeted-mutation sample is single-class ({survived} of {} mutants \
         survived their owning test), so no predictor can beat the constant one and \
         this question is not measurable on it. That is a fixture to widen, not a \
         result to report.",
        asserts_intent.len()
    );
    bars.beats_constant("BAR 1 test_asserts_intent", &asserts_intent, false);

    // ---- Bar 2: code_implements_intent ----------------------------------
    let confirmed: Vec<_> = violating
        .iter()
        .filter(|mutant| mutant.killed_by_owning_test)
        .collect();
    for mutant in violating.iter().filter(|m| !m.killed_by_owning_test) {
        println!(
            "(excluded from Bar 2's negative class, no test failure to prove the \
             violation: {} — {})",
            mutant.id(),
            mutant.rationale
        );
    }
    assert!(
        confirmed.len() >= 5,
        "too few confirmed violating mutants to measure code_implements_intent: {}",
        confirmed.len()
    );
    let mut implements: Vec<BinaryRow> = Vec::new();
    let mut severity_pairs: Vec<SeverityPair> = Vec::new();
    for mutant in &confirmed {
        implements.push(binary_row(
            &said,
            &mutant.base_id,
            "code_implements_intent",
            true,
        ));
        implements.push(binary_row(
            &said,
            &mutant.id(),
            "code_implements_intent",
            false,
        ));
        severity_pairs.push(SeverityPair {
            id: mutant.id(),
            original: said.get(&mutant.base_id).and_then(|a| a.severity),
            mutant: said.get(&mutant.id()).and_then(|a| a.severity),
        });
    }
    println!(
        "{}",
        binary_report(
            "Bar 2 — code_implements_intent (shipped vs. violating mutant)",
            &implements
        )
    );
    bars.beats_constant("BAR 2 code_implements_intent", &implements, true);

    // ---- Bar 3: tests_only_its_own_mock ---------------------------------
    let mut mock_rows: Vec<BinaryRow> = Vec::new();
    let mut real_only: Vec<BinaryRow> = Vec::new();
    for triple in &triples {
        let (truth, reason) = asserts_only_its_own_setup(&triple.test_body);
        println!("static check {}: {truth} — {reason}", triple.id);
        let row = binary_row(&said, &triple.id, "tests_only_its_own_mock", truth);
        real_only.push(row.clone());
        mock_rows.push(row);
    }
    for triple in constructed_mock_only() {
        let (truth, reason) = asserts_only_its_own_setup(&triple.test_body);
        println!(
            "static check {} (CONSTRUCTED): {truth} — {reason}",
            triple.id
        );
        assert!(
            truth,
            "{}: a constructed mock-only row the static check does not call mock-only \
             is a broken fixture, not a measurement",
            triple.id
        );
        mock_rows.push(binary_row(
            &said,
            &triple.id,
            "tests_only_its_own_mock",
            truth,
        ));
    }
    let (real_stats, _) = tally_binary(&real_only);
    println!(
        "\n**Bar 3, the 29 REAL rows alone** — the static check finds {} mock-only \
         tests among them, so a constant `no` predictor already scores {:.1}%. \
         Ungated, reported for what it is: the lens said `mock-only` for {} of them \
         (false positives), and cleared {}.",
        real_only.iter().filter(|row| row.truth).count(),
        constant_binary_baseline(&real_only).1,
        real_stats.false_positive + real_stats.true_positive,
        real_stats.true_negative
    );
    println!(
        "{}",
        binary_report(
            "Bar 3 — tests_only_its_own_mock (29 real + 8 constructed)",
            &mock_rows
        )
    );
    bars.beats_constant("BAR 3 tests_only_its_own_mock", &mock_rows, false);
    let (mock_stats, _) = tally_binary(&mock_rows);
    bars.check(
        mock_stats.recall().is_some_and(|value| value > 0.0),
        "BAR 3 tests_only_its_own_mock",
        &format!(
            "mock-only-class recall {:?} over the constructed rows",
            mock_stats.recall()
        ),
    );

    // ---- Bar 4: code_exceeds_requirement --------------------------------
    let additive: Vec<_> = all_mutants
        .iter()
        .filter(|mutant| mutant.class == MutantClass::Additive && !mutant.killed_by_owning_test)
        .collect();
    for mutant in all_mutants
        .iter()
        .filter(|m| m.class == MutantClass::Additive && m.killed_by_owning_test)
    {
        println!(
            "(excluded from Bar 4, the owning test caught it so it is not purely \
             additive: {} — {})",
            mutant.id(),
            mutant.rationale
        );
    }
    assert!(
        additive.len() >= 4,
        "too few surviving additive mutants to measure code_exceeds_requirement: {}",
        additive.len()
    );
    let mut exceeds: Vec<BinaryRow> = Vec::new();
    for mutant in &additive {
        exceeds.push(binary_row(
            &said,
            &mutant.base_id,
            "code_exceeds_requirement",
            false,
        ));
        exceeds.push(binary_row(
            &said,
            &mutant.id(),
            "code_exceeds_requirement",
            true,
        ));
    }
    println!(
        "{}",
        binary_report(
            "Bar 4 — code_exceeds_requirement (shipped vs. additive mutant)",
            &exceeds
        )
    );
    bars.beats_constant("BAR 4 code_exceeds_requirement", &exceeds, true);

    // ---- Bar 5: divergence_kind -----------------------------------------
    let mut labels: BTreeMap<String, LabelRow> = BTreeMap::new();
    for mutant in &violating {
        let truth = if mutant.killed_by_owning_test {
            "aligned"
        } else {
            "test_weaker_than_requirement"
        };
        labels
            .entry(mutant.base_id.clone())
            .or_insert_with(|| LabelRow {
                id: mutant.base_id.clone(),
                truth,
                asked: said
                    .get(&mutant.base_id)
                    .and_then(|a| a.divergence_kind.clone()),
                derived: said
                    .get(&mutant.base_id)
                    .and_then(BatteryAnswers::derived_divergence_kind),
            });
        if mutant.killed_by_owning_test {
            let id = mutant.id();
            labels.insert(
                id.clone(),
                LabelRow {
                    asked: said.get(&id).and_then(|a| a.divergence_kind.clone()),
                    derived: said
                        .get(&id)
                        .and_then(BatteryAnswers::derived_divergence_kind),
                    id,
                    truth: "code_short_of_requirement",
                },
            );
        }
    }
    for mutant in &additive {
        let id = mutant.id();
        labels.insert(
            id.clone(),
            LabelRow {
                asked: said.get(&id).and_then(|a| a.divergence_kind.clone()),
                derived: said
                    .get(&id)
                    .and_then(BatteryAnswers::derived_divergence_kind),
                id,
                truth: "code_exceeds_requirement",
            },
        );
    }
    let labels: Vec<LabelRow> = labels.into_values().collect();
    println!("\n## Bar 5 — divergence_kind\n");
    println!("| Row | Truth | Asked | Derived |");
    println!("| --- | --- | --- | --- |");
    for row in &labels {
        println!(
            "| {} | {} | {} | {} |",
            row.id,
            row.truth,
            row.asked.as_deref().unwrap_or("n/a"),
            row.derived.unwrap_or("n/a"),
        );
    }
    let (asked_rate, asked_n, asked_missing) = label_agreement(&labels, |row| row.asked.clone());
    let (derived_rate, derived_n, derived_missing) =
        label_agreement(&labels, |row| row.derived.map(ToOwned::to_owned));
    let (majority, majority_rate) = constant_label_baseline(&labels);
    println!(
        "\n- asked directly: {asked_rate:.1}% over {asked_n} answered ({asked_missing} unanswered)"
    );
    println!(
        "- derived from the same response's noul answers: {derived_rate:.1}% over \
         {derived_n} answered ({derived_missing} unanswered)"
    );
    println!("- majority-label constant predictor `{majority}`: {majority_rate:.1}%");
    let best = asked_rate.max(derived_rate);
    bars.check(
        best > majority_rate,
        "BAR 5 divergence_kind",
        &format!(
            "asked {asked_rate:.1}%, derived {derived_rate:.1}%, majority label \
             `{majority}` {majority_rate:.1}%"
        ),
    );

    // ---- Bar 6: severity discrimination ---------------------------------
    let (win_rate, compared, ties) = severity_discrimination(&severity_pairs);
    println!(
        "\n**BAR 6 severity** — over {compared} (shipped, confirmed-defective mutant) \
         pairs, the mutant scored strictly more severe {win_rate:.1}% of the time \
         ({ties} ties). This is discrimination, not accuracy: severity has no \
         mechanical ground truth, and a constant answer wins no pair, so the null \
         is 50%."
    );
    assert!(
        compared >= 5,
        "too few severity pairs both sides answered to say anything: {compared}"
    );
    bars.check(
        win_rate > 50.0,
        "BAR 6 severity discrimination",
        &format!(
            "the confirmed-defective mutant scored strictly more severe in \
             {win_rate:.1}% of {compared} pairs, against a 50% coin-flip null"
        ),
    );

    bars.settle();
}

/// Bar 1's rows for `population`, one per violating mutant, scored against
/// `said`'s `test_asserts_intent` on the UNMUTATED triple.
fn asserts_intent_rows(
    said: &BTreeMap<String, BatteryAnswers>,
    population: &[&Mutant],
) -> Vec<BinaryRow> {
    population
        .iter()
        .map(|mutant| {
            binary_row(
                said,
                &mutant.base_id,
                "test_asserts_intent",
                mutant.killed_by_owning_test,
            )
        })
        .collect()
}

/// Bar 3's rows: the 29 real triples and the 8 constructed ones, truth from
/// the static check, exactly as the v0 gate builds them.
fn mock_only_rows(
    said: &BTreeMap<String, BatteryAnswers>,
    triples: &[GapTriple],
) -> Vec<BinaryRow> {
    triples
        .iter()
        .cloned()
        .chain(constructed_mock_only())
        .map(|triple| {
            let (truth, _) = asserts_only_its_own_setup(&triple.test_body);
            binary_row(said, &triple.id, "tests_only_its_own_mock", truth)
        })
        .collect()
}

/// **`battery-v1` (PLAT-979).** Provenance: PLAT-979, PLAT-839. The label fix
/// and the rewording pre-registered in this file's module doc, measured over
/// the same rows as v0's wording in the same pass, so the effect of each can
/// be read on its own.
#[tokio::test]
async fn battery_v1_corrects_bar_1_labels_and_rewords_bars_1_and_3() {
    let client = live_client();
    let triples = corpus();

    let v0_violating: Vec<Mutant> = mutants()
        .into_iter()
        .filter(|mutant| mutant.class == MutantClass::Violating)
        .collect();
    let additions = mutants_v1_additions();
    assert!(
        additions
            .iter()
            .all(|mutant| mutant.class == MutantClass::Violating),
        "the v1 additions are violating mutants for P1 only"
    );
    let original: Vec<&Mutant> = v0_violating.iter().collect();
    let mut corrected: Vec<&Mutant> = Vec::new();
    for mutant in v0_violating.iter().chain(&additions) {
        if contradicts_cited_requirement(&triples, mutant) {
            corrected.push(mutant);
        } else {
            println!(
                "(excluded from corrected P1, the mutant contradicts no behaviour its \
                 triple's cited requirement states: {} — {})",
                mutant.id(),
                mutant.rationale
            );
        }
    }
    assert!(
        corrected.len() >= 8,
        "corrected P1 is below the pre-registered minimum of 8 rows: {}",
        corrected.len()
    );
    let survived = corrected
        .iter()
        .filter(|mutant| !mutant.killed_by_owning_test)
        .count();
    assert!(
        survived > 0 && survived < corrected.len(),
        "corrected P1 is single-class ({survived} of {} survived), so it is not \
         measurable",
        corrected.len()
    );

    // Every row either bar reads: the 29 real triples (a superset of every
    // P1 base) and the 8 constructed mock-only rows.
    let targets: Vec<GapTriple> = triples
        .iter()
        .cloned()
        .chain(constructed_mock_only())
        .collect();
    let v0 = ask_all(&client, &targets, Variant::FullBattery).await;
    let v1 = ask_all(&client, &targets, Variant::FullBatteryV1).await;
    let mut bars = Bars::default();

    // ---- Bar 1 ------------------------------------------------------------
    let v0_original = asserts_intent_rows(&v0, &original);
    let v0_corrected = asserts_intent_rows(&v0, &corrected);
    let v1_corrected = asserts_intent_rows(&v1, &corrected);
    println!(
        "{}",
        binary_report(
            "Bar 1, v0 wording, ORIGINAL P1 (replicates v0; ungated)",
            &v0_original
        )
    );
    println!(
        "{}",
        binary_report(
            "Bar 1, v0 wording, CORRECTED P1 (label fix alone; ungated)",
            &v0_corrected
        )
    );
    println!(
        "{}",
        binary_report("Bar 1-v1, v1 wording, CORRECTED P1 (gated)", &v1_corrected)
    );
    bars.beats_constant("BAR 1-v1 test_asserts_intent", &v1_corrected, false);

    // ---- Bar 3 ------------------------------------------------------------
    let v0_mock = mock_only_rows(&v0, &triples);
    let v1_mock = mock_only_rows(&v1, &triples);
    for (label, rows) in [("v0", &v0_mock), ("v1", &v1_mock)] {
        let real: Vec<BinaryRow> = rows
            .iter()
            .filter(|row| !row.id.starts_with("MOCK-"))
            .cloned()
            .collect();
        let (stats, _) = tally_binary(&real);
        println!(
            "\n**Bar 3 {label}, the 29 REAL rows alone** (ungated): the lens said \
             mock-only for {} of them and cleared {}.",
            stats.false_positive + stats.true_positive,
            stats.true_negative
        );
    }
    println!(
        "{}",
        binary_report("Bar 3, v0 wording (replicates v0; ungated)", &v0_mock)
    );
    println!(
        "{}",
        binary_report("Bar 3-v1, v1 wording (gated)", &v1_mock)
    );
    bars.beats_constant("BAR 3-v1 tests_only_its_own_mock", &v1_mock, false);
    let (v1_mock_stats, _) = tally_binary(&v1_mock);
    bars.check(
        v1_mock_stats.recall().is_some_and(|value| value > 0.0),
        "BAR 3-v1 tests_only_its_own_mock",
        &format!(
            "mock-only-class recall {:?} over the constructed rows",
            v1_mock_stats.recall()
        ),
    );

    bars.settle();
}
