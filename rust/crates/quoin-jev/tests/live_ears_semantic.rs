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
    EarsQuestionSet, EarsVerdict, M2Fixture, NO_DEFECT, QUESTION_SET_JSON, grade_defect,
    grade_pattern, jev_flags_defect, m2_corpus, m6_corpus, run as ears_run,
};
use support::grading::{
    Graded, Verdict, defect_recall, disagreement, no_defect_recall, percent, report, tally,
    trivial_baseline,
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
        .filter(|row| row.verdict == support::grading::Verdict::Unanswered)
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

/// `v3`'s derivation rule, fixed before its first call.
///
/// The shipped rule ([`support::ears::grade_defect`]) calls a statement
/// defective when Jev's six-way `ears_pattern_actual` choice disagrees with
/// the engine's naive pattern, **or** when Jev judges the response
/// unmeasurable. `v3` deletes the first term: the label comes only from the
/// three `noul` answers, plus the engine's own deterministic pattern as
/// context. Jev's six-way pick is not consulted at all.
///
/// The priority order is read off `ears-question-set.json`'s own `note`
/// fields, not fitted to this corpus's answer key:
///
/// * `response_measurable` — the note names the denylist it exists to
///   replace ("`shall be robust` sails through"), so an unmeasurable
///   response is a defect on any pattern.
/// * `condition_is_unwanted` — the note says "Disambiguates When/If", so on
///   a statement the engine read as `event_driven`, an unwanted condition
///   means the `When` should have been an `If`.
/// * `trigger_is_momentary` — the note says "Disambiguates When/While", so
///   on the same engine reading, a non-momentary trigger means the `When`
///   should have been a `While`.
///
/// A missing answer scores `0.5`, which fires no branch: an answer the
/// service did not give must not manufacture a defect.
///
/// **Stated blind spot, before running.** Both disambiguators are phrased
/// relative to a `When` statement, so this rule can raise a
/// pattern-confusion defect only where the engine already read
/// `event_driven`. A `While`-really-`When` or an `If`-really-`When`
/// statement is unreachable. Both pattern-confusion defects in this corpus
/// (`EARS-FIX-002`, `EARS-FIX-007`) happen to be engine-`event_driven`, so
/// the blind spot costs nothing here — that is a fact about the corpus, not
/// a property of the rule, and it means this corpus cannot measure the cost.
fn derive_defect_from_noul(engine_naive_pattern: &str, verdict: &EarsVerdict) -> &'static str {
    if verdict.response_measurable.unwrap_or(0.5) < 0.5 {
        return "defect";
    }
    if engine_naive_pattern == "event_driven"
        && (verdict.condition_is_unwanted.unwrap_or(0.5) >= 0.5
            || verdict.trigger_is_momentary.unwrap_or(0.5) < 0.5)
    {
        return "defect";
    }
    NO_DEFECT
}

/// Grades one M2 fixture under [`derive_defect_from_noul`], through the same
/// primary/contested/wrong comparison [`support::ears::grade_defect`] uses,
/// so the two columns differ only in how the label was reached.
fn grade_defect_derived(fixture: &M2Fixture, verdict: &EarsVerdict) -> Graded {
    let expected = if fixture.labels.has_defect {
        "defect"
    } else {
        NO_DEFECT
    }
    .to_owned();
    let contested: Vec<String> = match &fixture.labels.has_defect_contested {
        Some(all) => all
            .iter()
            .map(|v| if *v { "defect" } else { NO_DEFECT }.to_owned())
            .collect(),
        None => vec![expected.clone()],
    };
    let actual_class = derive_defect_from_noul(&fixture.engine_naive_pattern, verdict).to_owned();
    let outcome = if actual_class == expected {
        Verdict::Primary
    } else if contested.iter().any(|r| *r == actual_class) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };
    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected,
        contested,
        actual: actual_class.clone(),
        actual_class,
        verdict: outcome,
        confidence: verdict.pattern_confidence,
    }
}

/// Provenance: PLAT-838 follow-up. **`v3`, pre-registered before its first
/// call.**
///
/// The shipped question failed MP-229 bar 1 by -12.5pp. Its label depends on
/// a six-way `choice`; PLAT-839's gap-analysis lens passed its own gate
/// asking narrow yes/no questions. `v3` tests whether that shape helps here:
/// the request is unchanged (same question set, same context, the six-way
/// choice is still asked and still answered), but the defect call is derived
/// from the three `noul` answers via [`derive_defect_from_noul`] instead.
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
