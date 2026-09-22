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
    BatteryAnswers, BinaryRow, LabelRow, MutantClass, SeverityPair, answers,
    asserts_only_its_own_setup, binary_report, constant_binary_baseline, constant_label_baseline,
    constructed_mock_only, label_agreement, mutants, mutated, severity_discrimination,
    tally_binary,
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

/// One `FullBattery` request per target, answers indexed by row id.
async fn ask_all(client: &Client, targets: &[GapTriple]) -> BTreeMap<String, BatteryAnswers> {
    let mut out = BTreeMap::new();
    for triple in targets {
        let response = client
            .system_one(build_request(triple, Variant::FullBattery))
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
    let said = ask_all(&client, &targets).await;
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
