// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-838: the EARS semantic lens against the real Jev service.
//!
//! Same discipline as `live_criterion_strength.rs` (PLAT-917): every other
//! test in this crate scripts its answers through `typesafe_sdk_http::Mock`.
//! This file is the only place that finds out whether Jev can answer this
//! question set at all, through `quoin_jev::client::production` -- a real
//! key, a real request.
//!
//! **Nothing here runs in the default gate.** It sits behind the non-default
//! `live-api` feature, so `make rust-gate` stays socketless and key-free.
//!
//! ```bash
//! cd rust && cargo test -p quoin-jev --features live-api -- --nocapture
//! ```
//!
//! # The gate bars were pre-registered in MP-229 before this file's first live call
//!
//! [`the_ears_lens_gate_mp_229`] is the ONE test that fails the build. Its
//! four bars are copied verbatim from `spec/assurance/MP-229-jev-ears-gate.md`
//! ("Ship-as-advisory bars"), which was committed in the same PR before any
//! live call this file makes. Every other test here reports a number and
//! asserts only that the instrument is sane -- a slow afternoon or a small
//! corpus must never fail the build on its own.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "every cast in this file converts a request/row/token count (never realistically \
              exceeding a few thousand) to or from f64 for a printed ratio or a percentile \
              index -- none crosses a wire or persistence boundary, which is what this \
              workspace's cast lints exist to guard"
)]
#![allow(
    clippy::too_many_lines,
    reason = "the gate test reads as one linear pre-registered checklist (MP-222 through \
              MP-231, in the order MP-229 states them); splitting it into helpers that no \
              other test calls would scatter the checklist rather than clarify it"
)]

mod support;

use std::time::{Duration, Instant};

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use quoin_jev::JevErrorCode;
use support::ears::{
    DefectRule, EarsQuestionSet, EarsVerdict, NO_DEFECT, QUESTION_SET_JSON, QUESTION_SET_V5_JSON,
    grade_defect, grade_defect_derived, grade_pattern, grade_under, jev_flags_defect,
    jev_flags_defect_v3, jev_flags_under, m2_corpus, m6_corpus, reaches, run as ears_run,
    v3_reaches,
};
use support::grading::{
    Graded, Verdict, defect_recall, disagreement, expected_calibration_error, no_defect_recall,
    percent, report, tally, trivial_baseline,
};

/// PLAT-838's stated floor: "M1 disagreement (N=5 minimum, state N)". This
/// default is intentionally the floor, not the ticket's own N>=20 (300
/// requests) -- that larger N is opt-in via `JEV_RUNS`, matching
/// `live_criterion_strength.rs`'s own precedent for the same cost tradeoff.
const DEFAULT_M2_RUNS: usize = 5;

fn live_client() -> Client {
    let config = quoin_jev::config::resolve(&Process).unwrap_or_else(|error| {
        assert_eq!(
            error.code,
            JevErrorCode::MissingKey,
            "unexpected config failure: {error}"
        );
        panic!(
            "the `live-api` feature is on but no API key resolved. This test does not \
             skip. Set TYPESAFE_API_KEY, or drop --features live-api."
        );
    });
    quoin_jev::client::production(config).expect("builds a real transport")
}

/// One M2 pass: every fixture once, graded both ways (the gated defect/clean
/// reduction, and the supplementary raw 6-way pattern read).
struct M2Pass {
    defect_graded: Vec<Graded>,
    pattern_graded: Vec<Graded>,
    input_tokens: u64,
    output_tokens: u64,
    latencies: Vec<Duration>,
    elapsed: Duration,
}

async fn run_m2_pass(client: &Client, qs: &EarsQuestionSet) -> M2Pass {
    let corpus = m2_corpus();
    let mut pass = M2Pass {
        defect_graded: Vec::with_capacity(corpus.fixtures.len()),
        pattern_graded: Vec::with_capacity(corpus.fixtures.len()),
        input_tokens: 0,
        output_tokens: 0,
        latencies: Vec::with_capacity(corpus.fixtures.len()),
        elapsed: Duration::ZERO,
    };
    let started = Instant::now();
    for fixture in &corpus.fixtures {
        let call_started = Instant::now();
        let verdict: EarsVerdict = ears_run(client, &fixture.context(), qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        pass.latencies.push(call_started.elapsed());
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        pass.defect_graded.push(grade_defect(fixture, &verdict));
        pass.pattern_graded.push(grade_pattern(fixture, &verdict));
    }
    pass.elapsed = started.elapsed();
    pass
}

/// One M6 pass: every real statement once, with whether Jev's answer flags
/// it as a semantic defect against its own engine-derived pattern.
struct M6Pass {
    /// `(statement id, flagged?, confidence)` for the engine-clean pool.
    clean: Vec<(String, Option<bool>, Option<f64>)>,
    /// Same, for the engine-flagged pool.
    flagged: Vec<(String, Option<bool>, Option<f64>)>,
    input_tokens: u64,
    output_tokens: u64,
    latencies: Vec<Duration>,
    elapsed: Duration,
}

async fn run_m6_pass(client: &Client, qs: &EarsQuestionSet) -> M6Pass {
    let corpus = m6_corpus();
    let mut pass = M6Pass {
        clean: Vec::with_capacity(corpus.clean.len()),
        flagged: Vec::with_capacity(corpus.flagged.len()),
        input_tokens: 0,
        output_tokens: 0,
        latencies: Vec::with_capacity(corpus.clean.len() + corpus.flagged.len()),
        elapsed: Duration::ZERO,
    };
    let started = Instant::now();
    for statement in &corpus.clean {
        let call_started = Instant::now();
        let verdict = ears_run(client, &statement.context(), qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    statement.id(),
                    error.code.as_str(),
                    error.message
                )
            });
        pass.latencies.push(call_started.elapsed());
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        let flag = jev_flags_defect(statement, &verdict);
        pass.clean
            .push((statement.id(), flag, verdict.pattern_confidence));
    }
    for statement in &corpus.flagged {
        let call_started = Instant::now();
        let verdict = ears_run(client, &statement.context(), qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    statement.id(),
                    error.code.as_str(),
                    error.message
                )
            });
        pass.latencies.push(call_started.elapsed());
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        let flag = jev_flags_defect(statement, &verdict);
        pass.flagged
            .push((statement.id(), flag, verdict.pattern_confidence));
    }
    pass.elapsed = started.elapsed();
    pass
}

/// Provenance: PLAT-838. **The gate.** Every bar here is copied verbatim from
/// `spec/assurance/MP-229-jev-ears-gate.md`'s "Ship-as-advisory bars", which
/// was committed before this file made its first live call (`git log` on
/// that path shows the commit predates any `TYPESAFE_API_KEY` use in this
/// branch's history).
///
/// This test grades through the **shipped** rule ([`grade_defect`] /
/// [`jev_flags_defect`]), which consults the six-way `ears_pattern_actual`
/// choice. It is kept as the `v1` column. The verdict PLAT-838 reports is
/// [`the_ears_lens_gate_mp_229_v3`], which applies the same four bars to the
/// `v3` derivation rule end to end.
#[tokio::test]
async fn the_ears_lens_gate_mp_229() {
    let client = live_client();
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);

    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_M2_RUNS);
    assert!(runs >= 2, "a disagreement rate needs at least two runs");

    let mut m2_passes = Vec::with_capacity(runs);
    for _ in 0..runs {
        m2_passes.push(run_m2_pass(&client, &qs).await);
    }
    let first = &m2_passes[0];

    println!(
        "{}",
        report(
            "M2 defect/clean grading (gated; pass 1 of the M1 repetition set)",
            &first.defect_graded,
            NO_DEFECT,
        )
    );
    println!(
        "{}",
        report(
            "M2 raw ears_pattern_actual grading (supplementary, not gated; pass 1)",
            &first.pattern_graded,
            "<none>",
        )
    );

    let unanswered: Vec<&str> = first
        .defect_graded
        .iter()
        .filter(|row| row.verdict == Verdict::Unanswered)
        .map(|row| row.fixture_id.as_str())
        .collect();
    assert!(
        unanswered.is_empty(),
        "the service left M2 fixtures unanswered: {unanswered:?}"
    );
    assert!(
        first.input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    // MP-222: margin over the best constant predictor.
    let agreement = tally(&first.defect_graded)
        .agreement()
        .expect("16 rows graded");
    let (baseline_label, baseline) = trivial_baseline(&first.defect_graded);
    let margin = agreement - baseline;
    println!(
        "**MP-222** agreement {agreement:.1}% vs. best constant predictor \
         (`{baseline_label}` everywhere) {baseline:.1}% — margin {margin:+.1}pp"
    );

    // MP-223 / MP-224: defect recall and no-defect recall.
    let defect_r = defect_recall(&first.defect_graded, NO_DEFECT);
    let no_defect_r = no_defect_recall(&first.defect_graded, NO_DEFECT);
    println!(
        "**MP-223** defect recall {} — **MP-224** no-defect recall {}",
        defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        no_defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
    );

    // MP-225: disagreement under repetition, over the N M2 passes.
    let mut rates = Vec::new();
    for left in 0..m2_passes.len() {
        for right in (left + 1)..m2_passes.len() {
            rates.push(
                disagreement(
                    &m2_passes[left].defect_graded,
                    &m2_passes[right].defect_graded,
                )
                .expect("both passes graded"),
            );
        }
    }
    let mp225 = rates.iter().sum::<f64>() / rates.len() as f64;
    println!(
        "**MP-225** disagreement under repetition, N={runs}, {} pair(s): mean {mp225:.1}%, \
         range {:.1}%-{:.1}%",
        rates.len(),
        rates.iter().copied().fold(f64::INFINITY, f64::min),
        rates.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );

    // MP-231: engine/semantic delta, one pass over the real-statement corpus.
    let m6 = run_m6_pass(&client, &qs).await;
    let forward_answered: Vec<bool> = m6.clean.iter().filter_map(|(_, flag, _)| *flag).collect();
    let forward_delta = (!forward_answered.is_empty()).then(|| {
        percent(
            forward_answered.iter().filter(|f| **f).count(),
            forward_answered.len(),
        )
    });
    let inverse_answered: Vec<bool> = m6.flagged.iter().filter_map(|(_, flag, _)| *flag).collect();
    let inverse_delta = (!inverse_answered.is_empty()).then(|| {
        percent(
            inverse_answered.iter().filter(|f| !**f).count(),
            inverse_answered.len(),
        )
    });
    println!(
        "**MP-231** forward delta (engine-clean, Jev flags): {} over {}/{} answered — \
         inverse delta (engine-flagged, Jev calls clean): {} over {}/{} answered",
        forward_delta.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        forward_answered.len(),
        m6.clean.len(),
        inverse_delta.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        inverse_answered.len(),
        m6.flagged.len(),
    );
    assert!(m6.input_tokens > 0, "the M6 pass never reached the service");

    // ---- GATE ASSERTIONS — copied verbatim from MP-229 ----
    assert!(
        margin > 0.0,
        "MP-229 bar 1 (MP-222 margin > 0): got {margin:+.1}pp. The lens scored no better \
         than the best constant predictor on the M2 corpus."
    );
    assert!(
        defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 2 (MP-223 defect recall > 0%): got {defect_r:?}. The lens found none of \
         the M2 corpus's labelled defects."
    );
    assert!(
        no_defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 3 (MP-224 no-defect recall > 0%): got {no_defect_r:?}. The lens is the \
         flag-everything failure PLAT-917 measured on the sibling lens."
    );
    assert!(
        forward_delta.is_some_and(|v| v > 0.0 && v > mp225),
        "MP-229 bar 4 (MP-231 forward delta both non-zero and above the MP-225 noise floor): \
         forward delta {forward_delta:?}, MP-225 disagreement {mp225:.1}%. A delta at or below \
         the lens's own repetition noise is not evidence it sees something the engine misses."
    );
}

/// Provenance: PLAT-838. **A measure, not a gate.** MP-227: latency,
/// throughput and cost for one M2 pass plus a concurrency burst.
#[tokio::test]
async fn benchmark_latency_and_throughput() {
    let client = live_client();
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);
    let pass = run_m2_pass(&client, &qs).await;

    let count = pass.latencies.len() as f64;
    let total = pass.elapsed.as_secs_f64();
    let mean = pass
        .latencies
        .iter()
        .map(Duration::as_secs_f64)
        .sum::<f64>()
        / count;
    let tokens = pass.input_tokens + pass.output_tokens;
    let mut sorted = pass.latencies.clone();
    sorted.sort_unstable();
    let percentile = |fraction: f64| {
        let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    };

    println!(
        "\n## MP-227 — one sequential M2 pass, {} requests\n",
        pass.latencies.len()
    );
    println!("| Measure | Value |");
    println!("| --- | --- |");
    println!("| wall clock | {total:.2}s |");
    println!("| latency mean | {mean:.2}s |");
    println!("| latency p50 | {:.2}s |", percentile(0.5).as_secs_f64());
    println!("| latency p90 | {:.2}s |", percentile(0.9).as_secs_f64());
    println!("| latency max | {:.2}s |", percentile(1.0).as_secs_f64());
    println!("| throughput (sequential) | {:.2} req/s |", count / total);
    println!("| input tokens | {} |", pass.input_tokens);
    println!("| output tokens | {} |", pass.output_tokens);
    println!("| tokens/request | {:.0} |", tokens as f64 / count);

    assert!(total > 0.0, "a pass that took no time did not happen");
    assert!(
        pass.latencies
            .iter()
            .all(|latency| *latency > Duration::ZERO),
        "a request that took no time did not reach the network"
    );
}

// ---------------------------------------------------------------------
// v3: derive the defect call from the `noul` answers alone
// ---------------------------------------------------------------------

/// Provenance: PLAT-838 follow-up. **`v3`, pre-registered before its first
/// call.**
///
/// The shipped question failed MP-229 bar 1 by -12.5pp. Its label depends on
/// a six-way `choice`; PLAT-839's gap-analysis lens passed its own gate
/// asking narrow yes/no questions. `v3` tests whether that shape helps here:
/// the request is unchanged (same question set, same context, the six-way
/// choice is still asked and still answered), but the defect call is derived
/// from the three `noul` answers via [`support::ears::derive_defect_from_noul`] instead.
/// Both columns are graded from the same responses, so service variance is
/// not a confound between them.
///
/// **Bars.** MP-229 bars 1-3, verbatim, applied to the `v3-derived` column:
/// margin over the best constant predictor > 0, defect recall > 0,
/// no-defect recall > 0. Bar 4 (MP-231's forward delta) is **out of scope
/// and not evaluated here**: it measures the M6 real-statement corpus
/// through `jev_flags_defect`, which this change does not touch, and it
/// already passed. A `v3` that cleared bars 1-3 would still need bar 4 rerun
/// before any GO.
///
/// # `v3` result, 2026-09-21: bars 1-3 clear, by a wide margin
///
/// MEASURED against `jev-latest`, N=5 full passes. Every pass returned the
/// identical `v3-derived` score; the `v3-direct` column moved once (no-defect
/// recall 45.5% to 54.5%), so the spread is the shipped rule's, not the
/// derived rule's.
///
/// | column | MP-222 agreement | margin | MP-223 defect recall | MP-224 no-defect recall |
/// | --- | --- | --- | --- | --- |
/// | `v3-direct` (shipped rule) | 62.5% | -12.5 pp | 75.0% | 45.5-54.5% |
/// | **`v3-derived`** | **93.8%** | **+18.8 pp** | **100.0%** | **81.8%** |
///
/// Deleting the six-way term turned a -12.5pp failure into a +18.8pp pass,
/// on the same sixteen responses. Fourteen rows land on the primary
/// reading, one on the contested reading (`EARS-FIX-011`), one wrong
/// (`EARS-FIX-013`, a clean statement called defective).
///
/// The shipped gate [`the_ears_lens_gate_mp_229`], rerun the same day,
/// still fails bar 1 only: MP-225 disagreement 2.5% (range 0.0-6.2%) over
/// 10 pairs, and MP-231 forward delta 45.0% over 20/20 answered, inverse
/// 8.0% over 25/25 -- so bar 4 clears its noise floor today as it did
/// before.
///
/// **What this does not yet establish.** Bar 4 is measured through
/// `jev_flags_defect`, which still consults the six-way choice, so the M6
/// pool is not flagged by the rule `v3` gates on. Re-deriving that flagging
/// under [`support::ears::derive_defect_from_noul`] and re-measuring MP-231
/// is the remaining step, and it must be pre-registered before it is run. The
/// corpus is also sixteen agent-labelled fixtures carrying five defects,
/// three of them `unmeasurable_response` -- a single `noul` question
/// reaches those three, so the margin rests on a narrow base.
///
/// # Superseded, and by what
///
/// **Every number in the table above was measured against the sixteen-fixture
/// M2 corpus revision (`8b5d87f`), not the grown fifty-nine-fixture one this
/// file now reads.** This test still runs and still reports its two columns,
/// but on the grown corpus its numbers are not the ones above; treat the
/// table as a dated record of the sixteen-fixture run, not as a live
/// measurement. [`the_ears_lens_gate_mp_229_v3`] is the test that carries the
/// verdict now: it applies all four MP-229 bars to the `v3-derived` column
/// over the grown corpus, with MP-231 re-derived under the same rule.
#[tokio::test]
async fn the_ears_lens_with_noul_derived_defect_v3() {
    let client = live_client();
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);
    let corpus = m2_corpus();

    let mut direct: Vec<Graded> = Vec::with_capacity(corpus.fixtures.len());
    let mut derived: Vec<Graded> = Vec::with_capacity(corpus.fixtures.len());
    let mut input_tokens = 0u64;

    for fixture in &corpus.fixtures {
        let verdict: EarsVerdict = ears_run(&client, &fixture.context(), &qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        input_tokens += verdict.usage_input_tokens;
        direct.push(grade_defect(fixture, &verdict));
        derived.push(grade_defect_derived(fixture, &verdict));
    }

    println!(
        "{}",
        report(
            "M2 v3-direct (shipped rule: pattern mismatch OR unmeasurable; same calls as v3-derived)",
            &direct,
            NO_DEFECT,
        )
    );
    println!(
        "{}",
        report(
            "M2 v3-derived (noul answers only, six-way choice not consulted)",
            &derived,
            NO_DEFECT,
        )
    );

    assert!(
        input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    for (name, graded) in [("v3-direct", &direct), ("v3-derived", &derived)] {
        let agreement = tally(graded).agreement().expect("16 rows graded");
        let (baseline_label, baseline) = trivial_baseline(graded);
        let defect_r = defect_recall(graded, NO_DEFECT);
        let no_defect_r = no_defect_recall(graded, NO_DEFECT);
        println!(
            "**GATE {name}** MP-222 agreement {agreement:.1}% vs `{baseline_label}` \
             {baseline:.1}% — margin {:+.1}pp | MP-223 defect recall {} | MP-224 no-defect \
             recall {}",
            agreement - baseline,
            defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
            no_defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        );
    }

    let agreement = tally(&derived).agreement().expect("16 rows graded");
    let (_, baseline) = trivial_baseline(&derived);
    let margin = agreement - baseline;
    let defect_r = defect_recall(&derived, NO_DEFECT);
    let no_defect_r = no_defect_recall(&derived, NO_DEFECT);
    assert!(
        margin > 0.0,
        "MP-229 bar 1 (MP-222 margin > 0) on v3-derived: got {margin:+.1}pp"
    );
    assert!(
        defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 2 (MP-223 defect recall > 0%) on v3-derived: got {defect_r:?}"
    );
    assert!(
        no_defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 3 (MP-224 no-defect recall > 0%) on v3-derived: got {no_defect_r:?}"
    );
}

/// PLAT-838's stated M1 floor is N>=5 passes on the sixteen-fixture corpus
/// (80 requests). The grown corpus is 59 fixtures, so five passes plus the M6
/// pool would be 340 requests in one test. Three passes over 59 fixtures is
/// 177 requests -- more than the 80 the N=5 run cost, still three
/// independent pairs for MP-225, and the sampling that matters for
/// MP-222/223/224 is the corpus itself, which grew 3.7x. `JEV_RUNS` raises
/// it; whatever N is used is printed with every number it produced.
const DEFAULT_M2_RUNS_V3: usize = 3;

/// One `v3` M2 pass over the grown corpus: every fixture once, graded under
/// both the shipped rule and the `v3` derivation, from the same response.
struct V3Pass {
    direct: Vec<Graded>,
    derived: Vec<Graded>,
    input_tokens: u64,
    /// Rows where the service left at least one noul the `v3` rule consults
    /// unanswered. This exists because the rule's `unwrap_or(0.5)` default
    /// does NOT behave as its own prose claims on `condition_is_unwanted`
    /// (see [`support::ears::derive_defect_from_noul`]'s "Defect found"
    /// note): at the default the When/If branch fires. The code is left as
    /// pre-registered; this count is how a reader tells whether that branch
    /// could have touched the numbers. Zero means it could not have.
    unanswered_noul_rows: usize,
}

async fn run_v3_m2_pass(client: &Client, qs: &EarsQuestionSet) -> V3Pass {
    let corpus = m2_corpus();
    let mut pass = V3Pass {
        direct: Vec::with_capacity(corpus.fixtures.len()),
        derived: Vec::with_capacity(corpus.fixtures.len()),
        input_tokens: 0,
        unanswered_noul_rows: 0,
    };
    for fixture in &corpus.fixtures {
        let verdict = ears_run(client, &fixture.context(), qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        pass.input_tokens += verdict.usage_input_tokens;
        let consults_disambiguators = fixture.engine_naive_pattern == "event_driven";
        if verdict.response_measurable.is_none()
            || (consults_disambiguators
                && (verdict.condition_is_unwanted.is_none()
                    || verdict.trigger_is_momentary.is_none()))
        {
            pass.unanswered_noul_rows += 1;
        }
        pass.direct.push(grade_defect(fixture, &verdict));
        pass.derived.push(grade_defect_derived(fixture, &verdict));
    }
    pass
}

/// Defect recall over just the rows whose fixture id is in `ids`.
fn defect_recall_over(graded: &[Graded], ids: &[String]) -> Option<f64> {
    let subset: Vec<Graded> = graded
        .iter()
        .filter(|row| ids.contains(&row.fixture_id))
        .cloned()
        .collect();
    defect_recall(&subset, NO_DEFECT)
}

/// Provenance: PLAT-838 follow-up. **The `v3` gate — all four MP-229 bars,
/// one classification rule, one corpus revision.**
///
/// # What this test exists to fix
///
/// [`the_ears_lens_with_noul_derived_defect_v3`] cleared MP-229 bars 1-3 on
/// the `v3` derivation but left two holes its own doc names:
///
/// 1. **Bar 4 was measured under the wrong rule.** MP-231's forward delta
///    came from [`jev_flags_defect`], which consults the six-way choice --
///    the term `v3` deletes. A verdict mixing a `v1` delta into a `v3`
///    decision is not internally consistent. This test computes MP-231
///    through [`jev_flags_defect_v3`], which calls the same
///    [`support::ears::derive_defect_from_noul`] the M2 grading calls, and
///    gates on that number. The `v1` delta is still printed beside it, as
///    the comparison it now is rather than the gate input it was.
/// 2. **Five confirmed defects is too small a base for a recall number.**
///    The M2 corpus grew from 16 fixtures / 5 defects to 59 fixtures / 19
///    defects, holding the defect share at 32.2% (it was 31.2%) so that the
///    constant-predictor baseline MP-222 measures against does not move
///    merely because defects were added. Thirty-one of the fifty-nine rows
///    are now verbatim real statements from four agent-ix spec trees, each
///    recorded with its repo, path and commit. Every label remains
///    **agent-labelled, not human ground truth** -- unchanged from the
///    original corpus, and stated in the fixture file's own `$comment`.
///
/// # Pre-registration
///
/// The four bars are MP-229's "Ship-as-advisory bars", **unchanged** -- this
/// test adds no bar and moves no threshold, so those bars need no fresh
/// pre-registration. What is new and therefore pre-registered before this
/// test's first live call is (a) the grown corpus, (b) the `v3` MP-231
/// derivation, added to `spec/assurance/MP-231-jev-ears-engine-semantic-delta.md`
/// as `jev.ears-delta-v3`, and (c) the reachable/unreachable defect-recall
/// split below. All three are committed in the same commit as this test and
/// before any live call it makes.
///
/// # The reachable/unreachable split is reported, never filtered
///
/// [`support::ears::derive_defect_from_noul`]'s two disambiguators are
/// phrased for a `When` statement, so the rule can raise a pattern-confusion
/// defect only where the engine read `event_driven`. The original corpus
/// happened to contain no unreachable defect, so its 100% defect recall was
/// a fact about that corpus. The grown corpus carries five unreachable
/// defects on purpose (`while_is_really_when`, `if_is_really_when`,
/// `where_is_really_if`, `where_is_really_while`, `missing_trigger`). Both
/// halves are printed. The gate is MP-229's bar on the **whole**
/// population; the split exists so nobody reads an overall number without
/// seeing where it comes from.
///
/// # Result, 2026-09-22: all four bars clear — GO, as advisory
///
/// MEASURED, N=3 passes over 59 fixtures plus one 45-statement M6 pass, 222
/// requests.
///
/// **The record backing this table was withdrawn, 2026-09-23.**
/// `spec/evidence/measurements/plat838-ears-v3-20260922T0420Z.json` had no
/// `rawEvidence` and a dirty `quire-rs` source, both refused by the ported
/// intake validator, so it was removed rather than backfilled with data not
/// actually measured (agent-ix/quoin#607). Bytes remain in git history at
/// `0a257691` (#575). The numbers below are unbacked by a retained record
/// until a fresh `v3` run is recorded cleanly.
///
/// | bar | metric | value |
/// | --- | --- | --- |
/// | 1 | MP-222 margin over `clean` (69.5%) | **+13.6 pp** (agreement 83.1%) |
/// | 2 | MP-223 defect recall | **61.1%** (11/18) |
/// | 3 | MP-224 no-defect recall | **90.0%** (36/40) |
/// | 4 | MP-231 `v3` forward delta vs MP-225 0.0% | **30.0%** (6/20) |
///
/// The shipped six-way rule, graded from the same responses, scores
/// **-13.6 pp** on bar 1. MP-225 was 0.0% across all three pairs: every
/// pass returned an identical `v3` score. Zero rows had an unanswered
/// consulted noul, so the `condition_is_unwanted` default-fires defect
/// noted on [`support::ears::derive_defect_from_noul`] touched nothing here.
///
/// **The number that changed most is defect recall, and it fell.** The
/// 16-fixture run reported 100%; this one reports 61.1%, split 76.9% (10/13)
/// on reachable defects and 20.0% (1/5) on the five the rule structurally
/// cannot see. That is the blind spot's cost, measured for the first time.
/// It is not a regression in the lens -- the same rule, the same service, a
/// corpus that can finally see the gap.
///
/// MP-231's inverse delta is 88.0% under `v3` against 12.0% under `v1`.
/// That is **not** an engine false-positive rate: `v3` has no question that
/// could corroborate `ears:missing-subject`, `ears:non-singular` or
/// `ears:unclassifiable`, which is what the flagged pool is flagged under.
/// MP-231 bars nothing in that direction and MP-229 quotes neither.
///
/// Calibration is poor: ECE 0.3551, and the 0.9-1.0 confidence bucket
/// agreed on only 53.8% of its 13 rows. MP-229's M7 bound is 0.15, so this
/// blocks a later promotion to a hard `quire validate --strict` gate. It
/// does not touch the ship-as-advisory verdict, which is what this test
/// gates.
#[tokio::test]
async fn the_ears_lens_gate_mp_229_v3() {
    let client = live_client();
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);
    let corpus = m2_corpus();

    let reachable: Vec<String> = corpus
        .fixtures
        .iter()
        .filter(|fixture| v3_reaches(fixture))
        .map(|fixture| fixture.fixture_id.clone())
        .collect();
    let unreachable: Vec<String> = corpus
        .fixtures
        .iter()
        .filter(|fixture| !v3_reaches(fixture))
        .map(|fixture| fixture.fixture_id.clone())
        .collect();
    let defect_rows = corpus
        .fixtures
        .iter()
        .filter(|fixture| fixture.labels.has_defect)
        .count();
    let real_rows = corpus
        .fixtures
        .iter()
        .filter(|fixture| fixture.provenance == "real")
        .count();
    println!(
        "\n## M2 corpus revision under test: {} fixtures ({real_rows} real, {} synthetic), \
         {defect_rows} agent-labelled defects\n",
        corpus.fixtures.len(),
        corpus.fixtures.len() - real_rows,
    );

    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_M2_RUNS_V3);
    assert!(runs >= 2, "a disagreement rate needs at least two runs");

    let mut passes = Vec::with_capacity(runs);
    for _ in 0..runs {
        passes.push(run_v3_m2_pass(&client, &qs).await);
    }
    let first = &passes[0];
    assert!(
        first.input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    println!(
        "{}",
        report(
            "M2 v3-derived (noul answers only, six-way choice not consulted) — pass 1",
            &first.derived,
            NO_DEFECT,
        )
    );
    println!(
        "{}",
        report(
            "M2 v1-direct (shipped rule, same responses) — pass 1, comparison only",
            &first.direct,
            NO_DEFECT,
        )
    );

    for (name, graded) in [("v1-direct", &first.direct), ("v3-derived", &first.derived)] {
        let agreement = tally(graded).agreement().expect("the corpus is non-empty");
        let (baseline_label, baseline) = trivial_baseline(graded);
        println!(
            "**{name}** MP-222 agreement {agreement:.1}% vs `{baseline_label}` {baseline:.1}% \
             — margin {:+.1}pp | MP-223 defect recall {} | MP-224 no-defect recall {}",
            agreement - baseline,
            defect_recall(graded, NO_DEFECT)
                .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
            no_defect_recall(graded, NO_DEFECT)
                .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        );
    }

    // The gated column.
    let agreement = tally(&first.derived)
        .agreement()
        .expect("the corpus is non-empty");
    let (baseline_label, baseline) = trivial_baseline(&first.derived);
    let margin = agreement - baseline;
    let defect_r = defect_recall(&first.derived, NO_DEFECT);
    let no_defect_r = no_defect_recall(&first.derived, NO_DEFECT);

    let unanswered_rows: usize = passes.iter().map(|pass| pass.unanswered_noul_rows).sum();
    println!(
        "\n**Unanswered consulted nouls:** {unanswered_rows} row(s) across all {runs} pass(es). \
         Zero means the `condition_is_unwanted` default-fires defect documented on \
         `derive_defect_from_noul` could not have affected any number below."
    );
    println!(
        "**MP-223 split** reachable-by-v3 rows {} | unreachable-by-v3 rows {}",
        defect_recall_over(&first.derived, &reachable)
            .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        defect_recall_over(&first.derived, &unreachable)
            .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
    );

    // MP-225 on the gated column.
    let mut rates = Vec::new();
    for left in 0..passes.len() {
        for right in (left + 1)..passes.len() {
            rates.push(
                disagreement(&passes[left].derived, &passes[right].derived)
                    .expect("both passes graded"),
            );
        }
    }
    let mp225 = rates.iter().sum::<f64>() / rates.len() as f64;
    println!(
        "**MP-225** (v3-derived) disagreement under repetition, N={runs}, {} pair(s): mean \
         {mp225:.1}%, range {:.1}%-{:.1}%",
        rates.len(),
        rates.iter().copied().fold(f64::INFINITY, f64::min),
        rates.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );

    // MP-231, re-derived under the v3 rule, over the same M6 corpus.
    let m6 = m6_corpus();
    let mut forward_v3 = Vec::new();
    let mut forward_v1 = Vec::new();
    let mut inverse_v3 = Vec::new();
    let mut inverse_v1 = Vec::new();
    let mut m6_tokens = 0u64;
    for (pool, delta_v3, delta_v1) in [
        (&m6.clean, &mut forward_v3, &mut forward_v1),
        (&m6.flagged, &mut inverse_v3, &mut inverse_v1),
    ] {
        for statement in pool {
            let verdict = ears_run(&client, &statement.context(), &qs)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "{}: {} — {}",
                        statement.id(),
                        error.code.as_str(),
                        error.message
                    )
                });
            m6_tokens += verdict.usage_input_tokens;
            if let Some(flag) = jev_flags_defect_v3(statement, &verdict) {
                delta_v3.push(flag);
            }
            if let Some(flag) = jev_flags_defect(statement, &verdict) {
                delta_v1.push(flag);
            }
        }
    }
    assert!(m6_tokens > 0, "the M6 pass never reached the service");

    let rate = |answered: &[bool], want: bool| {
        (!answered.is_empty()).then(|| {
            percent(
                answered.iter().filter(|f| **f == want).count(),
                answered.len(),
            )
        })
    };
    let forward_delta = rate(&forward_v3, true);
    let inverse_delta = rate(&inverse_v3, false);
    println!(
        "\n**MP-231 (v3, `jev.ears-delta-v3`)** forward delta {} over {}/{} answered — inverse \
         delta {} over {}/{} answered",
        forward_delta.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        forward_v3.len(),
        m6.clean.len(),
        inverse_delta.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        inverse_v3.len(),
        m6.flagged.len(),
    );
    println!(
        "**MP-231 (v1, comparison only)** forward delta {} over {}/{} answered — inverse delta \
         {} over {}/{} answered",
        rate(&forward_v1, true).map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        forward_v1.len(),
        m6.clean.len(),
        rate(&inverse_v1, false).map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        inverse_v1.len(),
        m6.flagged.len(),
    );

    println!(
        "\n### MP-229 verdict inputs (v3-derived, N={runs}, corpus {} fixtures)\n\n\
         | bar | metric | value |\n| --- | --- | --- |\n\
         | 1 | MP-222 margin over `{baseline_label}` | {margin:+.1}pp |\n\
         | 2 | MP-223 defect recall | {} |\n\
         | 3 | MP-224 no-defect recall | {} |\n\
         | 4 | MP-231 forward delta vs MP-225 {mp225:.1}% | {} |",
        corpus.fixtures.len(),
        defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        no_defect_r.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
        forward_delta.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%")),
    );

    // ---- GATE ASSERTIONS — MP-229's four bars, verbatim, on the v3 column ----
    assert!(
        margin > 0.0,
        "MP-229 bar 1 (MP-222 margin > 0) on v3-derived over the grown corpus: got {margin:+.1}pp"
    );
    assert!(
        defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 2 (MP-223 defect recall > 0%) on v3-derived: got {defect_r:?}"
    );
    assert!(
        no_defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 3 (MP-224 no-defect recall > 0%) on v3-derived: got {no_defect_r:?}"
    );
    assert!(
        forward_delta.is_some_and(|v| v > 0.0 && v > mp225),
        "MP-229 bar 4 (MP-231 forward delta non-zero and above the MP-225 noise floor), both \
         re-derived under the v3 rule: forward delta {forward_delta:?}, MP-225 {mp225:.1}%"
    );
}

// ---------------------------------------------------------------------
// PLAT-979: error analysis before any further variant
// ---------------------------------------------------------------------

/// Formats an optional probability for a per-row table.
fn cell(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"))
}

/// Provenance: PLAT-979 (PLAT-838 front). **A diagnostic, not a gate.**
///
/// PLAT-979's method is error analysis first: every disagreement between the
/// lens and the corpus is read and classified (wording / missing
/// sub-question / bad option-list / threshold / bad label) before anything
/// is changed. The gate tests print only aggregate numbers and the
/// [`report`] table, which carries the derived label but not the `noul`
/// answers it was derived from -- so a disagreement cannot be classified
/// from their output. This test prints, per M2 fixture, every raw answer
/// the `v3` rule reads, the six-way pick and its confidence, and the label
/// each rule derives. It asserts only that the service answered.
#[tokio::test]
async fn ears_per_row_answers_for_error_analysis() {
    let client = live_client();
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);
    let corpus = m2_corpus();

    println!(
        "\n| fixture | engine | defect_kind | expected | v3 | v1 | response_measurable | \
         trigger_is_momentary | condition_is_unwanted | six-way pick | pick conf |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    let mut answered = 0usize;
    for fixture in &corpus.fixtures {
        let verdict = ears_run(&client, &fixture.context(), &qs)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        if verdict.response_measurable.is_some() {
            answered += 1;
        }
        let v3 = grade_defect_derived(fixture, &verdict);
        let v1 = grade_defect(fixture, &verdict);
        println!(
            "| {} | {} | {} | {} | {}{} | {}{} | {} | {} | {} | {} | {} |",
            fixture.fixture_id,
            fixture.engine_naive_pattern,
            fixture.labels.defect_kind.as_deref().unwrap_or("—"),
            v3.contested.join("/"),
            v3.actual_class,
            if v3.verdict.agrees() { "" } else { " ✗" },
            v1.actual_class,
            if v1.verdict.agrees() { "" } else { " ✗" },
            cell(verdict.response_measurable),
            cell(verdict.trigger_is_momentary),
            cell(verdict.condition_is_unwanted),
            verdict.pattern.as_deref().unwrap_or("—"),
            cell(verdict.pattern_confidence),
        );
    }
    assert_eq!(
        answered,
        corpus.fixtures.len(),
        "the service left `response_measurable` unanswered on some fixture"
    );
}

/// One PLAT-979 variant, measured end to end under MP-229's four bars: `N`
/// M2 passes and one M6 pass through `question_set_json`, every label and
/// confidence from `rule` (see [`support::ears::grade_under`]).
///
/// Prints, beyond the bars: every disagreement row with the answers behind
/// it (so the next round of error analysis needs no extra call), the
/// reachable/unreachable defect-recall split, and MP-226's calibration error
/// twice -- over the derived confidence this variant grades, and over the
/// six-way pick's confidence every run through `v3` used, for comparison.
async fn run_variant_gate(variant: &str, question_set_json: &str, rule: DefectRule) {
    let client = live_client();
    let qs = EarsQuestionSet::parse(question_set_json);
    let corpus = m2_corpus();
    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_M2_RUNS_V3);
    assert!(runs >= 2, "a disagreement rate needs at least two runs");

    let mut passes: Vec<Vec<Graded>> = Vec::with_capacity(runs);
    let mut first_pick_confidence: Vec<Graded> = Vec::new();
    let mut first_rows: Vec<String> = Vec::new();
    let mut unanswered = 0usize;
    for pass_index in 0..runs {
        let mut graded = Vec::with_capacity(corpus.fixtures.len());
        for fixture in &corpus.fixtures {
            let verdict = ears_run(&client, &fixture.context(), &qs)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "{}: {} — {}",
                        fixture.fixture_id,
                        error.code.as_str(),
                        error.message
                    )
                });
            if verdict.response_measurable.is_none()
                || verdict.condition_is_unwanted.is_none()
                || verdict.trigger_is_momentary.is_none()
            {
                unanswered += 1;
            }
            let row = grade_under(fixture, &verdict, rule);
            if pass_index == 0 {
                if !row.verdict.agrees() {
                    first_rows.push(format!(
                        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                        fixture.fixture_id,
                        fixture.engine_naive_pattern,
                        fixture.labels.defect_kind.as_deref().unwrap_or("—"),
                        row.expected,
                        row.actual_class,
                        cell(verdict.response_measurable),
                        cell(verdict.trigger_is_momentary),
                        cell(verdict.condition_is_unwanted),
                        cell(row.confidence),
                    ));
                }
                let mut by_pick = row.clone();
                by_pick.confidence = verdict.pattern_confidence;
                first_pick_confidence.push(by_pick);
            }
            graded.push(row);
        }
        passes.push(graded);
    }
    let first = &passes[0];

    println!(
        "\n## {variant} — {} fixtures, N={runs}, rule {rule:?}\n",
        corpus.fixtures.len()
    );
    println!(
        "| fixture | engine | defect_kind | expected | {variant} | response_measurable | \
         trigger_is_momentary | condition_is_unwanted | derived conf |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    for row in &first_rows {
        println!("{row}");
    }
    println!(
        "\n{}",
        report(&format!("M2 {variant} — pass 1"), first, NO_DEFECT)
    );

    let agreement = tally(first).agreement().expect("the corpus is non-empty");
    let (baseline_label, baseline) = trivial_baseline(first);
    let margin = agreement - baseline;
    let defect_r = defect_recall(first, NO_DEFECT);
    let no_defect_r = no_defect_recall(first, NO_DEFECT);
    let ids = |want: bool| -> Vec<String> {
        corpus
            .fixtures
            .iter()
            .filter(|fixture| reaches(fixture, rule) == want)
            .map(|fixture| fixture.fixture_id.clone())
            .collect()
    };
    let show = |value: Option<f64>| {
        value.map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.1}%"))
    };
    println!(
        "**{variant}** MP-222 agreement {agreement:.1}% vs `{baseline_label}` {baseline:.1}% — \
         margin {margin:+.1}pp | MP-223 defect recall {} (reachable {}, unreachable {}) | \
         MP-224 no-defect recall {}",
        show(defect_r),
        show(defect_recall_over(first, &ids(true))),
        show(defect_recall_over(first, &ids(false))),
        show(no_defect_r),
    );
    println!(
        "**MP-226** ECE over the derived confidence {} | over the six-way pick's confidence \
         (the pre-v4 instrument, same labels) {}",
        expected_calibration_error(first)
            .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.4}")),
        expected_calibration_error(&first_pick_confidence)
            .map_or_else(|| "not_computed".to_owned(), |v| format!("{v:.4}")),
    );
    println!("**Unanswered nouls:** {unanswered} row(s) across {runs} pass(es)");

    let mut rates = Vec::new();
    for left in 0..passes.len() {
        for right in (left + 1)..passes.len() {
            rates.push(disagreement(&passes[left], &passes[right]).expect("both passes graded"));
        }
    }
    let mp225 = rates.iter().sum::<f64>() / rates.len() as f64;
    println!(
        "**MP-225** N={runs}, {} pair(s): mean {mp225:.1}%, range {:.1}%-{:.1}%",
        rates.len(),
        rates.iter().copied().fold(f64::INFINITY, f64::min),
        rates.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );

    let m6 = m6_corpus();
    let mut forward = Vec::new();
    let mut inverse = Vec::new();
    for (pool, delta) in [(&m6.clean, &mut forward), (&m6.flagged, &mut inverse)] {
        for statement in pool {
            let verdict = ears_run(&client, &statement.context(), &qs)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "{}: {} — {}",
                        statement.id(),
                        error.code.as_str(),
                        error.message
                    )
                });
            if let Some(flag) = jev_flags_under(statement, &verdict, rule) {
                delta.push(flag);
            }
        }
    }
    let rate = |answered: &[bool], want: bool| {
        (!answered.is_empty()).then(|| {
            percent(
                answered.iter().filter(|f| **f == want).count(),
                answered.len(),
            )
        })
    };
    let forward_delta = rate(&forward, true);
    println!(
        "**MP-231 ({variant})** forward delta {} over {}/{} answered — inverse delta {} over \
         {}/{} answered",
        show(forward_delta),
        forward.len(),
        m6.clean.len(),
        show(rate(&inverse, false)),
        inverse.len(),
        m6.flagged.len(),
    );

    assert!(
        margin > 0.0,
        "MP-229 bar 1 (MP-222 margin > 0) on {variant}: got {margin:+.1}pp"
    );
    assert!(
        defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 2 (MP-223 defect recall > 0%) on {variant}: got {defect_r:?}"
    );
    assert!(
        no_defect_r.is_some_and(|v| v > 0.0),
        "MP-229 bar 3 (MP-224 no-defect recall > 0%) on {variant}: got {no_defect_r:?}"
    );
    assert!(
        forward_delta.is_some_and(|v| v > 0.0 && v > mp225),
        "MP-229 bar 4 (MP-231 forward delta non-zero and above MP-225) on {variant}: forward \
         delta {forward_delta:?}, MP-225 {mp225:.1}%"
    );
}

/// Provenance: PLAT-979 (PLAT-838 front). **`v4`: the instrument fix.** Same
/// request and same labels as `v3`; only the confidence MP-226 grades
/// changes, from the six-way pick's to the `v3` call's own
/// ([`support::ears::derived_confidence`]). Pre-registered in MP-229's
/// Collection Procedure before its first call.
#[tokio::test]
async fn the_ears_lens_gate_mp_229_v4() {
    run_variant_gate("v4", QUESTION_SET_JSON, DefectRule::WhenOnly).await;
}

/// Provenance: PLAT-979 (PLAT-838 front). **`v5`: wording.** `v4` with two
/// `noul` questions reworded (`ears-question-set-v5.json`). Pre-registered
/// in MP-229's Collection Procedure before its first call.
#[tokio::test]
async fn the_ears_lens_gate_mp_229_v5() {
    run_variant_gate("v5", QUESTION_SET_V5_JSON, DefectRule::WhenOnly).await;
}

/// Provenance: PLAT-979 (PLAT-838 front). **`v6`: read the answers `v3`
/// ignores.** `v5`'s wording with [`DefectRule::KeywordContradiction`].
/// Pre-registered in MP-229's Collection Procedure before its first call.
#[tokio::test]
async fn the_ears_lens_gate_mp_229_v6() {
    run_variant_gate("v6", QUESTION_SET_V5_JSON, DefectRule::KeywordContradiction).await;
}
