// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The PLAT-839 gap-analysis semantic lens against the real Jev service.
//!
//! PLAT-839 asks whether Jev is worth wiring into `gap-analysis` step 5 as
//! the default, replacing the ad-hoc, opt-in, unreproducible LLM pass. The
//! standing ruling on that ticket is explicit: **Jev does not get integrated
//! anywhere without real proof that it is worth integrating**, and this file
//! is that proof, not the integration. Nothing in `quoin-jev`'s `src/` or in
//! `skills/gap-analysis/` changes because of this file; it is evaluation-only
//! and lives entirely behind the non-default `live-api` feature, same as
//! `live_criterion_strength.rs` (PLAT-917):
//!
//! ```bash
//! cd rust && cargo test -p quoin-jev --features live-api -- --nocapture
//! ```
//!
//! # M7 — mechanical ground truth, not a human labeller
//!
//! [`gap_semantic_support::CORPUS`] is 29 `requirement<->test<->code`
//! triples: 27 pulled from this repository's own `Trace:`-tagged tests, plus
//! two constructed adverse fixtures (`GAP-28`/`GAP-29`) per the ticket's own
//! acceptance criteria. 23 carry `mutation_vacuous`: ground truth from
//! actually stubbing the covered symbol in a scratch edit of this worktree,
//! rerunning exactly that one test, and recording pass (test still green ->
//! WAS vacuous) or fail (test caught the stub -> NOT vacuous), then
//! reverting the edit uncommitted. The other 6 are excluded from grading
//! (documented per-row in `mutation_method`) because their return type
//! admits no safe "stub returning a default" -- a generic function, two
//! re-exported constants, an enum-completeness guard, and a two-symbol
//! triple. See this crate's PR description for the per-triple mutation log
//! (which symbol, what stub, pass/fail) -- it is not repeated in this source
//! file so the two cannot drift silently if one is edited alone.
//!
//! # Two pre-registered variants, fixed before the first live call
//!
//! [`Variant::Solo`] (`assertion_vacuous` alone) and [`Variant::FullBattery`]
//! (the ticket's whole seven-question shape, one batched request per triple)
//! are the only two shapes this file tests, chosen and coded before any
//! live number existed. Both are reported; neither is dropped after seeing a
//! result -- PLAT-838's own review flagged that exact failure mode ("Pick
//! the... variant that... looked best... after the fact") in a sibling
//! lens's evaluation, and this file does not repeat it.
//!
//! # Pre-registered bars (fixed before the first live call)
//!
//! Bar 1 (beat doing nothing): M7 agreement on the mutated subset exceeds
//! [`gap_semantic_support::constant_baseline`] -- the best of "always
//! vacuous" or "always not-vacuous" on this corpus, computed from the data
//! rather than a written-down guess.
//!
//! Bar 2 (finds real vacuous tests): vacuous-class recall > 0 -- of the rows
//! mutation confirmed vacuous, at least one is actually flagged.
//!
//! Bar 3 (PLAT-917's own lesson -- "Jev never once returned the 'no defect'
//! answer"): not-vacuous-class recall > 0 -- of the rows mutation confirmed
//! NOT vacuous, at least one is actually cleared. A lens that flags
//! everything passes Bar 2 for free and fails this one by construction.
//!
//! All three are asserted in [`the_lens_beats_doing_nothing`] for BOTH
//! variants; a variant that fails any bar is reported as failing, not
//! silently excluded.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod gap_semantic_support;

use std::time::{Duration, Instant};

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use gap_semantic_support::{
    Graded, Variant, build_request, constant_baseline, corpus, disagreement, grade, mutated_corpus,
    report, tally, verdict_disagreement,
};
use quoin_jev::JevErrorCode;

/// Builds a client against the real service. Panics rather than skipping
/// when no key resolves -- same discipline as `live_criterion_strength.rs`:
/// a silent skip is how a suite reports green over a measurement that never
/// ran.
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

/// One pass over `triples` under `variant`: every graded row, plus the cost
/// and timing M5 needs.
struct Pass {
    graded: Vec<Graded>,
    input_tokens: u64,
    output_tokens: u64,
    latencies: Vec<Duration>,
    elapsed: Duration,
    ungraded: Vec<String>,
}

async fn run_pass(
    client: &Client,
    triples: &[gap_semantic_support::GapTriple],
    variant: Variant,
) -> Pass {
    let mut pass = Pass {
        graded: Vec::with_capacity(triples.len()),
        input_tokens: 0,
        output_tokens: 0,
        latencies: Vec::with_capacity(triples.len()),
        elapsed: Duration::ZERO,
        ungraded: Vec::new(),
    };
    let started = Instant::now();
    for triple in triples {
        let request = build_request(triple, variant);
        let request_started = Instant::now();
        let response = client.system_one(request).await.unwrap_or_else(|error| {
            panic!("{}: {error}", triple.id);
        });
        pass.latencies.push(request_started.elapsed());
        pass.input_tokens += response.usage.input_tokens;
        pass.output_tokens += response.usage.output_tokens;
        if triple.mutation_vacuous.is_some() {
            match grade(triple, &response) {
                Some(graded) => pass.graded.push(graded),
                None => pass.ungraded.push(triple.id.clone()),
            }
        }
    }
    pass.elapsed = started.elapsed();
    pass
}

/// **The gate.** Provenance: PLAT-839. Runs both pre-registered variants over
/// the mutated subset and checks all three bars stated in this file's module
/// doc, for each variant independently.
#[tokio::test]
async fn the_lens_beats_doing_nothing() {
    let client = live_client();
    let mutated = mutated_corpus();
    assert!(
        mutated.len() >= 10,
        "the mutated subset is too small to measure precision/recall meaningfully: {}",
        mutated.len()
    );

    for variant in [Variant::Solo, Variant::FullBattery] {
        let pass = run_pass(&client, &mutated, variant).await;
        println!(
            "{}",
            report(
                &format!("gap-analysis assertion_vacuous — {variant:?}"),
                &pass.graded
            )
        );

        assert!(
            pass.ungraded.is_empty(),
            "{variant:?}: rows the response never answered assertion_vacuous for: {:?}",
            pass.ungraded
        );
        assert_eq!(
            pass.graded.len(),
            mutated.len(),
            "{variant:?}: every mutated row graded"
        );

        let stats = tally(&pass.graded);
        let (baseline_label, baseline) = constant_baseline(&pass.graded);
        let agreement = stats.accuracy().expect("non-empty graded set");
        println!(
            "**{variant:?} GATE 1** agreement {agreement:.1}% vs. constant predictor \
             (`{baseline_label}`) {baseline:.1}%"
        );
        assert!(
            agreement > baseline,
            "{variant:?} scored {agreement:.1}%, no better than the constant predictor \
             `{baseline_label}` at {baseline:.1}%"
        );

        let recall = stats.recall();
        println!("**{variant:?} GATE 2** vacuous-class recall: {recall:?}");
        assert!(
            recall.is_some_and(|value| value > 0.0),
            "{variant:?} found none of the mutation-confirmed-vacuous tests"
        );

        let no_defect_recall = stats.no_defect_recall();
        println!("**{variant:?} GATE 3** not-vacuous-class recall: {no_defect_recall:?}");
        assert!(
            no_defect_recall.is_some_and(|value| value > 0.0),
            "{variant:?} never once called a mutation-confirmed-NOT-vacuous test \
             clear -- it flags everything, the exact failure PLAT-917 found in a \
             sibling lens"
        );

        println!(
            "**{variant:?} M5 (this pass)** {} requests, {}+{} tokens, {:.2}s wall clock",
            pass.latencies.len(),
            pass.input_tokens,
            pass.output_tokens,
            pass.elapsed.as_secs_f64()
        );
    }
}

/// **A measure, not a gate.** Provenance: PLAT-839. M1 (disagreement) and M6
/// (per-triple severity-verdict stability, `FullBattery` only) over N repeated
/// runs. The ticket asks N >= 20 for M1; that is 20 * 23 = 460 requests, so
/// it is opt-in via `JEV_RUNS`. This file's default is 5 -- PLAT-839's own
/// stated floor ("N=5 minimum, state N") -- so the standing cost of running
/// this file is 5 passes, not 20.
#[tokio::test]
async fn repeated_runs_report_disagreement_and_verdict_stability() {
    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5);
    assert!(runs >= 2, "a disagreement rate needs at least two runs");

    let client = live_client();
    let mutated = mutated_corpus();
    let mut passes = Vec::with_capacity(runs);
    for _ in 0..runs {
        passes.push(
            run_pass(&client, &mutated, Variant::FullBattery)
                .await
                .graded,
        );
    }

    let mut predicted_rates = Vec::new();
    let mut verdict_rates = Vec::new();
    for left in 0..passes.len() {
        for right in (left + 1)..passes.len() {
            if let Some(rate) = disagreement(&passes[left], &passes[right]) {
                predicted_rates.push(rate);
            }
            if let Some(rate) = verdict_disagreement(&passes[left], &passes[right]) {
                verdict_rates.push(rate);
            }
        }
    }

    let mean = |rates: &[f64]| {
        rates.iter().sum::<f64>() / f64::from(u32::try_from(rates.len()).unwrap_or(u32::MAX))
    };
    println!(
        "\n**M1 disagreement** over {runs} run(s), {} pair(s), {} triples each: \
         mean {:.1}%, range {:.1}%-{:.1}%.\n\
         (`predicted` = assertion_vacuous thresholded at 0.5.)\n",
        predicted_rates.len(),
        mutated.len(),
        mean(&predicted_rates),
        predicted_rates
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min),
        predicted_rates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max),
    );
    println!(
        "**M6 per-triple severity-verdict stability** (PASS/CONDITIONAL/FAIL proxy, \
         not the real audit-level Verdict -- this evaluation aggregates nothing): \
         mean {:.1}%, range {:.1}%-{:.1}%.\n",
        mean(&verdict_rates),
        verdict_rates.iter().copied().fold(f64::INFINITY, f64::min),
        verdict_rates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max),
    );

    assert!(
        predicted_rates
            .iter()
            .all(|rate| (0.0..=100.0).contains(rate)),
        "a rate outside 0-100% means the comparator is wrong, not the service"
    );
}

/// **A measure, not a gate.** Provenance: PLAT-839's M5: cost and latency for
/// one full pass over the whole 29-triple corpus (not just the mutated
/// subset), `FullBattery` variant -- the realistic per-audit shape.
#[tokio::test]
async fn benchmark_cost_and_latency() {
    let client = live_client();
    let all = corpus();
    let pass = run_pass(&client, &all, Variant::FullBattery).await;

    let count = f64::from(u32::try_from(pass.latencies.len()).unwrap_or(u32::MAX));
    let mean = pass
        .latencies
        .iter()
        .map(Duration::as_secs_f64)
        .sum::<f64>()
        / count;
    let mut sorted = pass.latencies.clone();
    sorted.sort_unstable();
    let p50 = sorted[(sorted.len() / 2).min(sorted.len() - 1)];
    // Integer arithmetic throughout -- no float round-trip needed for a
    // percentile index over a small, exactly-known length.
    let p90 = sorted[(sorted.len() * 9 / 10).min(sorted.len() - 1)];

    println!(
        "\n## M5 benchmark — one sequential pass, {} triples, FullBattery\n",
        all.len()
    );
    println!("| Measure | Value |");
    println!("| --- | --- |");
    println!("| requests | {} |", pass.latencies.len());
    println!("| wall clock | {:.2}s |", pass.elapsed.as_secs_f64());
    println!("| latency mean | {mean:.2}s |");
    println!("| latency p50 | {:.2}s |", p50.as_secs_f64());
    println!("| latency p90 | {:.2}s |", p90.as_secs_f64());
    println!("| input tokens | {} |", pass.input_tokens);
    println!("| output tokens | {} |", pass.output_tokens);
    let total_tokens = pass.input_tokens + pass.output_tokens;
    println!(
        "| tokens/request | {:.0} |",
        f64::from(u32::try_from(total_tokens).unwrap_or(u32::MAX)) / count
    );
    // $42/billion input tokens, output free -- the ticket's own stated
    // pricing (`## Cost`), not independently priced here.
    let cost =
        f64::from(u32::try_from(pass.input_tokens).unwrap_or(u32::MAX)) / 1_000_000_000.0 * 42.0;
    println!("| estimated cost (ticket's stated $42/B input, output free) | ${cost:.4} |");

    assert!(
        pass.elapsed > Duration::ZERO,
        "a pass that took no time did not happen"
    );
}
