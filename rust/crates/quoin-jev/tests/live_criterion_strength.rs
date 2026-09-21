// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The criterion-strength lens against the real service (PLAT-917).
//!
//! Every other test in this crate scripts its answers through
//! `typesafe_sdk_http::Mock`. Those tests are SDK/API conformance -- they
//! prove the crate builds a well-formed request and parses a well-formed
//! response. A mock agrees with whatever the test wrote into it, so they say
//! nothing about whether Jev can answer this question set at all. This file
//! is the only place that finds out, through
//! [`quoin_jev::client::production`] -- the SDK's real `Reqwest` transport,
//! `POST https://api.typesafe.ai/v1/systemone`, a real key. That constructor
//! has no other caller and no other test.
//!
//! **Nothing here runs in the default gate.** It sits behind the non-default
//! `live-api` feature, so `make rust-gate` stays socketless and key-free and
//! the invariant in `lib.rs` continues to hold for every unattended run.
//!
//! ```bash
//! cd rust && cargo test -p quoin-jev --features live-api -- --nocapture
//! ```
//!
//! # Correctness is a gate; speed and throughput are measures
//!
//! [`the_lens_beats_the_do_nothing_baseline`] **fails the build** when the
//! lens is not worth integrating. [`benchmark_latency_and_throughput`] and
//! [`repeated_runs_report_a_disagreement_rate`] report numbers and assert
//! only that the instrument itself is sane. A measure that gated would make
//! the suite fail on a slow afternoon; a gate that only measured would let an
//! ineffective lens ship.
//!
//! ## Why the gate is not an accuracy percentage
//!
//! MEASURED on the shipped corpus, and pinned by
//! `grading_math.rs::the_do_nothing_lens_scores_over_eighty_percent_so_agreement_cannot_be_the_gate`:
//! **a lens that answers `sound` to all eleven `weakness_kind` fixtures
//! scores 81.8% agreement and has found nothing.** `sound` is one of the two
//! recorded readings on six of the seven contested rows, so the do-nothing
//! answer is right most of the time by construction.
//!
//! An 80%-agreement gate would therefore be passed by a lens that never
//! fires. The gate is instead, in both halves:
//!
//! 1. **beat the best constant predictor** the corpus admits
//!    ([`support::trivial_baseline`], computed from the fixtures rather than
//!    written down, so it cannot go stale), and
//! 2. **find the defects** -- non-zero [`support::defect_recall`] over the
//!    rows where no reader thought `sound` was defensible.
//!
//! Both are pre-registered here, before the first live number was taken,
//! which is the discipline PLAT-838's M7 states and this lens needs equally.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod support;

use std::time::{Duration, Instant};

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use quoin_jev::{FrContext, FrVerdict, JevErrorCode, QuestionSet};
use support::{
    Graded, Verdict, corpus, defect_recall, disagreement, grade_coverage, grade_weakness, report,
    tally, trivial_baseline,
};

/// The confidence cutoff passed to `verdict::extract`.
///
/// 0.7 is the crate's own unmeasured default, recorded as such in PR #571.
/// It only sets whether a finding is annotated `unconfirmed`; it never
/// suppresses one, so it cannot change which rows this file grades -- only
/// how they print. Resetting it from the calibration curve is M3's job.
const CONFIDENCE_THRESHOLD: f64 = 0.7;

/// The shipped question set, compiled in so the asset and this test cannot
/// drift -- the same discipline `question_set.rs` and `lens.rs` already hold.
const QUESTION_SET: &str =
    include_str!("../../../../skills/spec-criterion-strength-analysis/assets/question-set.json");

/// One pass over the corpus: 15 requests, graded, with the cost and timing
/// the M5 measure needs.
struct Pass {
    graded: Vec<Graded>,
    input_tokens: u64,
    output_tokens: u64,
    latencies: Vec<Duration>,
    elapsed: Duration,
}

impl Pass {
    /// Per-request latency, sorted, for the percentile lines.
    fn percentile(&self, fraction: f64) -> Duration {
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();
        let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }
}

/// Builds a client against the real service.
///
/// Panics rather than skipping when no key resolves: a silent skip is how a
/// suite reports green over a measurement that never ran, and this crate's
/// own error taxonomy already has a name for the condition.
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

/// One request, with the fixture id in any failure message.
///
/// A transport failure aborts the pass rather than scoring as a wrong answer:
/// an outage is not a classifier verdict, and folding the two together would
/// let a 503 read as poor accuracy.
async fn call(
    client: &Client,
    context: &FrContext,
    questions: &QuestionSet,
    fixture_id: &str,
) -> (FrVerdict, Duration) {
    let started = Instant::now();
    let verdict = quoin_jev::lens::run(client, context, questions, CONFIDENCE_THRESHOLD)
        .await
        .unwrap_or_else(|error| {
            panic!("{fixture_id}: {} — {}", error.code.as_str(), error.message)
        });
    (verdict, started.elapsed())
}

/// Runs every fixture once, sequentially.
async fn run_once(client: &Client, questions: &QuestionSet) -> Pass {
    let corpus = corpus();
    let mut pass = Pass {
        graded: Vec::with_capacity(15),
        input_tokens: 0,
        output_tokens: 0,
        latencies: Vec::with_capacity(15),
        elapsed: Duration::ZERO,
    };
    let started = Instant::now();

    for fixture in &corpus.weakness_kind_fixtures {
        let (verdict, latency) =
            call(client, &fixture.context(), questions, &fixture.fixture_id).await;
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        pass.latencies.push(latency);
        pass.graded.push(grade_weakness(fixture, &verdict));
    }
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let (verdict, latency) =
            call(client, &fixture.context(), questions, &fixture.fixture_id).await;
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        pass.latencies.push(latency);
        pass.graded.push(grade_coverage(fixture, &verdict));
    }

    pass.elapsed = started.elapsed();
    pass
}

/// Provenance: PLAT-917. **The gate.** Does the real service, on the shipped
/// question set, do better than doing nothing -- and does it actually find
/// the defects eleven human-labelled criteria contain?
///
/// Fails the build when it does not. Both bars are stated in this file's
/// module doc and were fixed before the first live number was taken.
#[tokio::test]
async fn the_lens_beats_the_do_nothing_baseline() {
    let client = live_client();
    let questions = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");
    let pass = run_once(&client, &questions).await;

    println!(
        "{}",
        report("criterion-strength vs. the labelled corpus", &pass.graded)
    );

    // The call worked and the crate understood the answer. These come first:
    // a lens that answered nothing would otherwise reach the accuracy bars
    // with an empty denominator.
    assert_eq!(pass.graded.len(), 15, "every fixture was graded");
    let unanswered: Vec<&str> = pass
        .graded
        .iter()
        .filter(|row| row.verdict == Verdict::Unanswered)
        .map(|row| row.fixture_id.as_str())
        .collect();
    assert!(
        unanswered.is_empty(),
        "the service left fixtures unanswered: {unanswered:?}"
    );
    let unrecognized: Vec<(&str, &str)> = pass
        .graded
        .iter()
        .filter(|row| row.verdict == Verdict::Unrecognized)
        .map(|row| (row.fixture_id.as_str(), row.actual.as_str()))
        .collect();
    assert!(
        unrecognized.is_empty(),
        "labels outside the declared answer_space: {unrecognized:?}"
    );
    assert!(
        pass.input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    // Bar 1: beat the best constant predictor this corpus admits.
    let agreement = tally(&pass.graded).agreement().expect("15 rows graded");
    let (baseline_label, baseline) = trivial_baseline(&pass.graded);
    println!(
        "**GATE 1** agreement {agreement:.1}% vs. best constant predictor \
         (`{baseline_label}` everywhere) {baseline:.1}%"
    );
    assert!(
        agreement > baseline,
        "the lens scored {agreement:.1}%, no better than answering `{baseline_label}` \
         to everything ({baseline:.1}%). An agreement score at or below the \
         do-nothing baseline is not evidence the lens works."
    );

    // Bar 2: it has to find the defects, not just score well.
    let recall = defect_recall(&pass.graded).expect("the corpus holds unambiguous defects");
    println!("**GATE 2** defect recall {recall:.1}% (rows no reader called `sound`)");
    assert!(
        recall > 0.0,
        "the lens flagged none of the criteria every reader agreed were defective. \
         It can score well on this corpus by never firing; that is the failure \
         this bar exists to catch."
    );
}

/// Provenance: PLAT-917. **A measure, not a gate.** Latency, throughput and
/// cost for one pass -- PLAT-837's M5, plus the throughput figure that
/// decides whether this can run per-commit, per-PR or on demand.
///
/// Asserts only that the instrument is sane. A slow afternoon must not fail
/// the build.
#[tokio::test]
async fn benchmark_latency_and_throughput() {
    let client = live_client();
    let questions = QuestionSet::parse(QUESTION_SET).expect("parses");
    let pass = run_once(&client, &questions).await;

    let count = pass.latencies.len() as f64;
    let total = pass.elapsed.as_secs_f64();
    let mean = pass.latencies.iter().map(Duration::as_secs_f64).sum::<f64>() / count;
    let tokens = pass.input_tokens + pass.output_tokens;

    println!("\n## Benchmark — one sequential pass, 15 requests\n");
    println!("| Measure | Value |");
    println!("| --- | --- |");
    println!("| requests | {} |", pass.latencies.len());
    println!("| wall clock | {total:.2}s |");
    println!("| latency mean | {mean:.2}s |");
    println!("| latency min | {:.2}s |", pass.percentile(0.0).as_secs_f64());
    println!("| latency p50 | {:.2}s |", pass.percentile(0.5).as_secs_f64());
    println!("| latency p90 | {:.2}s |", pass.percentile(0.9).as_secs_f64());
    println!("| latency max | {:.2}s |", pass.percentile(1.0).as_secs_f64());
    println!("| throughput (sequential) | {:.2} req/s |", count / total);
    println!("| input tokens | {} |", pass.input_tokens);
    println!("| output tokens | {} |", pass.output_tokens);
    println!("| tokens/request | {:.0} |", tokens as f64 / count);

    // Concurrent throughput: five of the same request in flight at once,
    // against the sequential rate above. This is the number that decides
    // whether a 100-FR repo is a minute or an hour -- a sequential rate
    // extrapolated to a whole repo would overstate the cost by whatever
    // concurrency the service actually allows.
    let corpus = corpus();
    let context = corpus.weakness_kind_fixtures[0].context();
    let id = &corpus.weakness_kind_fixtures[0].fixture_id;
    let started = Instant::now();
    let burst = tokio::join!(
        call(&client, &context, &questions, id),
        call(&client, &context, &questions, id),
        call(&client, &context, &questions, id),
        call(&client, &context, &questions, id),
        call(&client, &context, &questions, id),
    );
    let burst_elapsed = started.elapsed().as_secs_f64();
    let slowest = [burst.0.1, burst.1.1, burst.2.1, burst.3.1, burst.4.1]
        .into_iter()
        .max()
        .unwrap()
        .as_secs_f64();
    println!("| throughput (5 concurrent) | {:.2} req/s |", 5.0 / burst_elapsed);
    println!("| 5-concurrent wall clock | {burst_elapsed:.2}s (slowest single {slowest:.2}s) |");
    println!(
        "\n5 concurrent requests took {burst_elapsed:.2}s against {:.2}s if run one \
         after another at the mean above — a speedup of {:.1}x.\n",
        mean * 5.0,
        (mean * 5.0) / burst_elapsed,
    );

    assert!(total > 0.0, "a pass that took no time did not happen");
    assert!(
        pass.latencies.iter().all(|latency| *latency > Duration::ZERO),
        "a request that took no time did not reach the network"
    );
}

/// Provenance: PLAT-917. **A measure, not a gate.** PLAT-837's M1: over
/// byte-identical input, how often does the verdict move?
///
/// The number to beat is the ad-hoc LLM pass's **17.0%** (N=12, `quire-rs`
/// PR #482). N here defaults to 2 -- the smallest run that can observe a
/// disagreement at all -- and is raised with `JEV_RUNS`. The ticket asks for
/// N >= 20, which is 300 requests, so that is opt-in rather than the standing
/// cost of running this file.
#[tokio::test]
async fn repeated_runs_report_a_disagreement_rate() {
    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2);
    assert!(runs >= 2, "a disagreement rate needs at least two runs");

    let client = live_client();
    let questions = QuestionSet::parse(QUESTION_SET).expect("parses");
    let mut passes = Vec::with_capacity(runs);
    for _ in 0..runs {
        passes.push(run_once(&client, &questions).await.graded);
    }

    let mut rates = Vec::new();
    for left in 0..passes.len() {
        for right in (left + 1)..passes.len() {
            rates.push(disagreement(&passes[left], &passes[right]).expect("both passes graded"));
        }
    }
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    let lowest = rates.iter().copied().fold(f64::INFINITY, f64::min);
    let highest = rates.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    println!(
        "\n**M1 disagreement** over {runs} run(s), {} pair(s), 15 fixtures each: \
         mean {mean:.1}%, range {lowest:.1}%-{highest:.1}%.\n\
         Baseline to beat: 17.0% (ad-hoc LLM pass, N=12, quire-rs PR #482).\n",
        rates.len(),
    );

    assert!(
        (0.0..=100.0).contains(&mean),
        "a rate outside 0-100% means the comparator is wrong, not the service"
    );
}
