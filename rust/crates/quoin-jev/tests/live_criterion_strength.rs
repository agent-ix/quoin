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
//!
//! # Pre-registration, round two (PLAT-917) -- written before any round-two call
//!
//! Round one sent the corpus's own fields only. `statement` went out empty on
//! 10 of 11 criteria and 5 carried no FR prose at all, so Jev judged
//! sentences in an isolation the human readers never had. Round one's result
//! (46.7% agreement, `sound` returned 0 of 5 times) is therefore a lower
//! bound, not a verdict. Round two gives the lens a fair shot, under bars
//! fixed here first.
//!
//! **Variants.** Selected with `JEV_VARIANT`; the default is `v1`.
//!
//! | id | FR context | `weakness_kind` question |
//! | --- | --- | --- |
//! | `v0` | corpus fields only (round one, re-run under the corrected grader) | shipped: "Classify this acceptance criterion's weakness, if any." |
//! | `v1` | full: statement, Description, Behavior, Constraints, verbatim from the cited spec file (`criterion-strength-fr-context.json`) | shipped |
//! | `v2` | full | [`V2_CHOICE_QUESTION`]: neutral, and defines `sound` |
//!
//! `v2` is the one extra variant allowed. It targets round one's dominant
//! failure: the shipped question asks for a weakness before it offers
//! `sound`, and defines none of the six labels. `v2` changes that one string
//! and nothing else. All three variants are reported whatever they score.
//!
//! **Bars.** A variant passes when all three hold on one graded pass:
//!
//! 1. agreement > the best constant predictor, one constant per family
//!    ([`support::trivial_baseline`]);
//! 2. defect recall > 0 over the criteria no reader called `sound`;
//! 3. **`sound` returned on at least 3 of the 5 criteria whose primary
//!    reading is `sound`** -- a false-positive rate on sound criteria below
//!    50%. Below that, a flag from this lens on a criterion is more likely
//!    noise than signal, and PLAT-837's M2 names false positives as the
//!    headline cost because each one costs a human read.
//!
//! **GO** requires one variant to pass all three bars on the gate run *and*
//! in at least 3 of 5 repeated runs (`JEV_RUNS=5`). Fifteen fixtures and one
//! run can pass by luck; a variant that fails most repeats is not a result.
//!
//! **Grader corrections made before round two, both in the lens's favour
//! until fixed:** the constant predictor now picks one constant per family
//! (round one's single-label form scored it at 60%, understating the bar),
//! and coverage rows no longer count as free "found" defects in recall.
//!
//! # Round-two result, 2026-09-21: NO-GO
//!
//! MEASURED against `jev-latest`, graded under the corrected grader. The
//! constant predictor (`sound` on criteria, level 2 on FRs) scores 80.0%.
//!
//! | variant | agreement | margin | defect recall | `sound` cleared | ECE | passes clearing all bars | M1 disagreement | tokens/request |
//! | --- | --- | --- | --- | --- | --- | --- | --- | --- |
//! | `v0` | 46.7% | -33.3 pp | 2/2 | 0/5 | 0.343 | 0 of 5 | 4.0% | 1521 |
//! | `v1` | 33.3% | -46.7 pp | 2/2 | 0/5 | 0.277 | 0 of 5 | 4.0% | 2015 |
//! | `v2` | 60.0-66.7% | -13.3 pp | 1/2 | 4/5 | 0.428 | 0 of 5 | 4.0% | 2110 |
//!
//! No variant clears bar 1 on any of fifteen passes. Full FR context made
//! the shipped question *worse* (46.7% to 33.3%). The neutral question fixes
//! the never-says-`sound` failure (0/5 to 4/5) but then clears
//! `CS-FIX-003`, a clean `implementation_coupled` defect, and still trails
//! the constant by 13 points. Bar 2's denominator is two criteria
//! (`CS-FIX-003`, `CS-FIX-005`), so it barely constrains anything on this
//! corpus. Latency is not a concern: p50 about 0.12 s per request, about
//! 8 requests/s sequential and 22-25 concurrent.
//!
//! # `v3`, pre-registered before its first call -- follow-up (PLAT-917)
//!
//! `v0`/`v1`/`v2` all ask Jev to pick `weakness_kind` directly from six
//! labels in one shot. PLAT-839's gap-analysis lens (a sibling evaluation,
//! same corpus of tickets) passed its own gate asking narrow yes/no
//! questions graded against mechanical ground truth -- the opposite shape
//! from an open 6-way pick. `v3` tests whether that pattern holds here: send
//! the identical `v0` request (corpus-only context, shipped question set --
//! `weakness_kind` is still asked, so the request shape does not change),
//! but derive the label from the five `noul` answers the response already
//! carries for every AC row, via [`derive_weakness_kind`], instead of using
//! Jev's own `choice` answer. Because it is the same request as `v0`, each
//! call is graded two ways from one response: `v3-direct` (Jev's own
//! `weakness_kind` choice -- a fresh, independent `v0` sample) and
//! `v3-derived` (this function's rule). Comparing the two from the same
//! calls removes day-to-day service variance as a confound between them.
//!
//! [`derive_weakness_kind`]'s priority order is read off
//! `question-set.json`'s own question text and its `falsifiable` note ("the
//! core check"), not fit to the fifteen fixtures' answer key -- a rule fit to
//! the corpus it is graded against would make any margin meaningless.
//! **Stated blind spot, before running:** no combination of the five `noul`
//! answers can express `happy_path_only`, since none of them test coverage
//! breadth. A fixture whose primary reading is `happy_path_only`
//! (`CS-FIX-014`) cannot be reached by this rule.
//!
//! Bars: identical to `v0`/`v1`/`v2` -- agreement beats the constant
//! predictor, defect recall > 0, `sound` cleared >= 3 of 5.
//!
//! # `v3` result, 2026-09-21: NO-GO, and the hypothesis is refuted
//!
//! MEASURED against `jev-latest`, N=5 (five full passes; every pass returned
//! the identical score, so the spread is zero -- this corpus is not where
//! service variance lives).
//!
//! | variant | agreement | margin | defect recall | `sound` cleared |
//! | --- | --- | --- | --- | --- |
//! | `v0` (round two) | 46.7% | -33.3 pp | 2/2 | 0/5 |
//! | `v1` | 33.3% | -46.7 pp | 2/2 | 0/5 |
//! | `v2` | 60.0-66.7% | -13.3 pp | 1/2 | 4/5 |
//! | `v3-direct` (fresh `v0` sample) | 46.7-53.3% | -26.7 pp | 2/2 | 0/5 |
//! | **`v3-derived`** | **33.3%** | **-46.7 pp** | 2/2 | 0/5 |
//!
//! Deriving the label made it **worse**, not better -- `v3-derived` ties
//! `v1` for the lowest score any variant has scored, and never answers
//! `sound` on any row. Eight of the eleven criteria derive to
//! `unmeasurable_threshold`, a label no fixture carries, because Jev answers
//! `threshold_present` false on most of them.
//!
//! ## Why, and why no further derivation rule will fix it
//!
//! [`the_noul_answers_are_reported_per_question`] scores each `noul`
//! question on its own against the corpus's recorded answer for it. MEASURED
//! on the same corpus:
//!
//! | question | compared | agreement | best constant |
//! | --- | --- | --- | --- |
//! | `falsifiable` | 9 | 88.9% | 88.9% |
//! | `implementation_coupled` | 10 | 80.0% | 80.0% |
//! | `restates_requirement` | 11 | 81.8% | 81.8% |
//! | `states_observable_outcome` | 11 | 63.6% | 81.8% |
//! | `threshold_present` | 11 | 72.7% | 72.7% |
//!
//! **Not one of the five yes/no questions beats its own constant
//! predictor.** Three of them *are* constants: Jev answered `falsifiable`
//! true on all 9 rows it was compared on, `restates_requirement` false on
//! all 11, and `implementation_coupled` true on exactly the 2 rows the
//! reader did. The other two vary and score below the constant.
//!
//! This refutes the hypothesis at its root. `v3` was built on the reading
//! that PLAT-839 passed because it asked narrow yes/no questions, so asking
//! this lens's label out of its yes/no sub-answers should help. It does not,
//! because the sub-answers carry no information over the constant either. A
//! different decision rule composes the same non-signal differently. The
//! corpus said as much before the run, on `CS-FIX-014`: "`weakness_kind` is
//! not a deterministic function of the five `noul` answers".

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "test-only statistics: counts are at most a few hundred and confidences lie in [0, 1], \
              so no cast here can truncate, lose a sign, or lose precision"
)]

mod support;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use quoin_jev::{FrContext, FrVerdict, JevErrorCode, QuestionSet};
use support::{
    Graded, Verdict, corpus, defect_recall, disagreement, grade_coverage, grade_weakness, report,
    sound_recall, tally, trivial_baseline,
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

/// The `weakness_kind` question `v2` sends instead of the shipped one.
///
/// Neutral about whether a weakness exists, and states what `sound` means in
/// the terms the five `noul` questions already use.
const V2_CHOICE_QUESTION: &str = "Which one label best describes this acceptance criterion as \
written? Answer `sound` when it is falsifiable, names an outcome observable from outside, states \
any threshold it relies on, and says more than the FR sentence it belongs to. Otherwise pick the \
weakness that applies.";

/// Which pre-registered variant this run measures. See the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// Corpus fields only, shipped question.
    V0,
    /// Full FR context, shipped question.
    V1,
    /// Full FR context, neutral question.
    V2,
}

impl Variant {
    /// Reads `JEV_VARIANT`. Panics on anything but the three ids, so a typo
    /// cannot silently measure the default.
    fn from_env() -> Self {
        match std::env::var("JEV_VARIANT").as_deref() {
            Err(_) | Ok("v1") => Self::V1,
            Ok("v0") => Self::V0,
            Ok("v2") => Self::V2,
            Ok(other) => panic!("JEV_VARIANT must be v0, v1 or v2; got {other:?}"),
        }
    }

    fn full_context(self) -> bool {
        self != Self::V0
    }

    fn questions(self) -> QuestionSet {
        let mut set = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");
        if self == Self::V2 {
            V2_CHOICE_QUESTION.clone_into(&mut set.choice.question);
        }
        set
    }
}

/// `v3`'s derivation rule. See this file's module doc for why the priority
/// order is fixed before any call, and why `happy_path_only` cannot be
/// reached.
fn derive_weakness_kind(noul: &[(String, f64)]) -> String {
    let value = |id: &str| -> f64 {
        noul.iter()
            .find(|(key, _)| key == id)
            .map_or(0.5, |(_, value)| *value)
    };
    if value("falsifiable") < 0.5 {
        "unfalsifiable"
    } else if value("restates_requirement") >= 0.5 {
        "restates_requirement"
    } else if value("implementation_coupled") >= 0.5 {
        "implementation_coupled"
    } else if value("threshold_present") < 0.5 {
        "unmeasurable_threshold"
    } else {
        "sound"
    }
    .to_owned()
}

/// Builds a synthetic [`FrVerdict`] carrying only `label` for `ac_id`, so
/// [`grade_weakness`] can score a derived label exactly as it scores Jev's
/// own `choice` answer -- same grader, same contested-label handling, no
/// second scoring path to keep in sync with the first.
fn synthetic_verdict(ac_id: &str, label: &str, noul_values: Vec<(String, f64)>) -> FrVerdict {
    if label == "sound" {
        return FrVerdict {
            classifier: "derived-from-noul".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: Vec::new(),
            sound: vec![ac_id.to_owned()],
            unrecognized: Vec::new(),
            unanswered: Vec::new(),
            coverage: None,
        };
    }
    let severity = quoin_jev::Severity::for_weakness_kind(label)
        .expect("derive_weakness_kind only returns labels with a severity, or `sound` above");
    FrVerdict {
        classifier: "derived-from-noul".to_owned(),
        usage_input_tokens: 0,
        usage_output_tokens: 0,
        findings: vec![quoin_jev::Finding {
            ac_id: ac_id.to_owned(),
            weakness_kind: label.to_owned(),
            severity,
            confidence: 1.0,
            unconfirmed: false,
            probabilities: Vec::new(),
            noul: quoin_jev::verdict::NoulSignals {
                values: noul_values,
            },
        }],
        sound: Vec::new(),
        unrecognized: Vec::new(),
        unanswered: Vec::new(),
        coverage: None,
    }
}

/// The three pre-registered bars, evaluated on one graded pass.
struct Bars {
    agreement: f64,
    baseline_label: String,
    baseline: f64,
    defect_recall: f64,
    sound_cleared: usize,
    sound_total: usize,
}

/// The pre-registered floor for bar 3.
const SOUND_CLEARED_FLOOR: usize = 3;

impl Bars {
    fn of(graded: &[Graded]) -> Self {
        let (baseline_label, baseline) = trivial_baseline(graded);
        let (sound_cleared, sound_total) =
            sound_recall(graded).expect("the corpus has sound criteria");
        Self {
            agreement: tally(graded).agreement().expect("rows graded"),
            baseline_label,
            baseline,
            defect_recall: defect_recall(graded).expect("the corpus holds unambiguous defects"),
            sound_cleared,
            sound_total,
        }
    }

    fn beats_baseline(&self) -> bool {
        self.agreement > self.baseline
    }

    fn finds_defects(&self) -> bool {
        self.defect_recall > 0.0
    }

    fn clears_sound(&self) -> bool {
        self.sound_cleared >= SOUND_CLEARED_FLOOR
    }

    fn all(&self) -> bool {
        self.beats_baseline() && self.finds_defects() && self.clears_sound()
    }

    fn line(&self) -> String {
        format!(
            "agreement {:.1}% vs constant ({}) {:.1}% [{}] | defect recall {:.1}% [{}] | \
             sound {}/{} (floor {SOUND_CLEARED_FLOOR}) [{}]",
            self.agreement,
            self.baseline_label,
            self.baseline,
            pass(self.beats_baseline()),
            self.defect_recall,
            pass(self.finds_defects()),
            self.sound_cleared,
            self.sound_total,
            pass(self.clears_sound()),
        )
    }
}

fn pass(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

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
async fn run_once(client: &Client, questions: &QuestionSet, variant: Variant) -> Pass {
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
        let context = if variant.full_context() {
            fixture.context_full()
        } else {
            fixture.context()
        };
        let (verdict, latency) = call(client, &context, questions, &fixture.fixture_id).await;
        pass.input_tokens += verdict.usage_input_tokens;
        pass.output_tokens += verdict.usage_output_tokens;
        pass.latencies.push(latency);
        pass.graded.push(grade_weakness(fixture, &verdict));
    }
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let context = if variant.full_context() {
            fixture.context_full()
        } else {
            fixture.context()
        };
        let (verdict, latency) = call(client, &context, questions, &fixture.fixture_id).await;
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
    let variant = Variant::from_env();
    let questions = variant.questions();
    let pass = run_once(&client, &questions, variant).await;

    println!(
        "{}",
        report(
            &format!("criterion-strength vs. the labelled corpus, variant {variant:?}"),
            &pass.graded
        )
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

    // The three pre-registered bars. Evaluated together and reported
    // together before any assertion, so a failure on bar 1 cannot hide what
    // bars 2 and 3 would have said.
    let bars = Bars::of(&pass.graded);
    println!("**GATE {variant:?}** {}", bars.line());
    assert!(
        bars.all(),
        "{variant:?} does not clear the pre-registered bars: {}",
        bars.line()
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
    let variant = Variant::from_env();
    let questions = variant.questions();
    let pass = run_once(&client, &questions, variant).await;
    println!("\nvariant {variant:?}");

    let count = pass.latencies.len() as f64;
    let total = pass.elapsed.as_secs_f64();
    let mean = pass
        .latencies
        .iter()
        .map(Duration::as_secs_f64)
        .sum::<f64>()
        / count;
    let tokens = pass.input_tokens + pass.output_tokens;

    println!("\n## Benchmark — one sequential pass, 15 requests\n");
    println!("| Measure | Value |");
    println!("| --- | --- |");
    println!("| requests | {} |", pass.latencies.len());
    println!("| wall clock | {total:.2}s |");
    println!("| latency mean | {mean:.2}s |");
    println!(
        "| latency min | {:.2}s |",
        pass.percentile(0.0).as_secs_f64()
    );
    println!(
        "| latency p50 | {:.2}s |",
        pass.percentile(0.5).as_secs_f64()
    );
    println!(
        "| latency p90 | {:.2}s |",
        pass.percentile(0.9).as_secs_f64()
    );
    println!(
        "| latency max | {:.2}s |",
        pass.percentile(1.0).as_secs_f64()
    );
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
    let context = if variant.full_context() {
        corpus.weakness_kind_fixtures[0].context_full()
    } else {
        corpus.weakness_kind_fixtures[0].context()
    };
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
    println!(
        "| throughput (5 concurrent) | {:.2} req/s |",
        5.0 / burst_elapsed
    );
    println!("| 5-concurrent wall clock | {burst_elapsed:.2}s (slowest single {slowest:.2}s) |");
    println!(
        "\n5 concurrent requests took {burst_elapsed:.2}s against {:.2}s if run one \
         after another at the mean above — a speedup of {:.1}x.\n",
        mean * 5.0,
        (mean * 5.0) / burst_elapsed,
    );

    assert!(total > 0.0, "a pass that took no time did not happen");
    assert!(
        pass.latencies
            .iter()
            .all(|latency| *latency > Duration::ZERO),
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
    let variant = Variant::from_env();
    let questions = variant.questions();
    let mut passes = Vec::with_capacity(runs);
    let mut cleared = 0;
    for run in 1..=runs {
        let graded = run_once(&client, &questions, variant).await.graded;
        let bars = Bars::of(&graded);
        println!("run {run} ({variant:?}): {}", bars.line());
        if bars.all() {
            cleared += 1;
        }
        passes.push(graded);
    }
    println!(
        "\n**Bars cleared** in {cleared} of {runs} run(s) for {variant:?} \
         (GO needs at least 3 of 5).\n"
    );

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

/// Provenance: PLAT-917 follow-up. **`v3`, pre-registered before its first
/// call.** See this file's module doc for the hypothesis, the derivation
/// rule and its stated blind spot. Reports `v3-direct` and `v3-derived`
/// side by side from the same live calls, and gates only on `v3-derived`.
#[tokio::test]
async fn the_lens_with_noul_derived_labels_v3() {
    let client = live_client();
    let questions = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");
    let corpus = corpus();

    let mut direct_graded = Vec::with_capacity(15);
    let mut derived_graded = Vec::with_capacity(15);
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;

    for fixture in &corpus.weakness_kind_fixtures {
        // Corpus-only context, matching `v0` -- isolates the
        // derivation-strategy variable from the context variable `v1`
        // already showed hurts on this corpus.
        let context = fixture.context();
        let ac_ids = context.ac_ids();
        let request = quoin_jev::lens::build_request(&context, &questions);
        let response = client
            .system_one(request)
            .await
            .map_err(|error| quoin_jev::error::classify(&error))
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        input_tokens += response.usage.input_tokens;
        output_tokens += response.usage.output_tokens;

        // v3-direct: Jev's own `weakness_kind` choice from this same
        // response -- a fresh, independent v0 sample.
        let direct_verdict =
            quoin_jev::verdict::extract(&response, &questions, &ac_ids, CONFIDENCE_THRESHOLD);
        direct_graded.push(grade_weakness(fixture, &direct_verdict));

        // v3-derived: the five `noul` answers from the same response, run
        // through `derive_weakness_kind` instead.
        let ac_id = &ac_ids[0];
        let noul_values: Vec<(String, f64)> = questions
            .noul
            .iter()
            .filter_map(|entry| {
                let key = QuestionSet::noul_key(ac_id, entry);
                match response.answer(&key) {
                    Some(typesafe_sdk_answers::Answer::Noul(answer)) => {
                        Some((entry.id.clone(), answer.noul))
                    }
                    _ => None,
                }
            })
            .collect();
        let label = derive_weakness_kind(&noul_values);
        let derived_verdict = synthetic_verdict(ac_id, &label, noul_values);
        derived_graded.push(grade_weakness(fixture, &derived_verdict));
    }

    // Coverage rows are outside this hypothesis (a `score` question, not
    // `choice`) -- computed once and shared by both columns, so the
    // comparison below isolates only the weakness_kind change.
    let mut coverage_graded = Vec::with_capacity(4);
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let context = fixture.context();
        let verdict = quoin_jev::lens::run(&client, &context, &questions, CONFIDENCE_THRESHOLD)
            .await
            .unwrap_or_else(|error| {
                panic!("{}: {} — {}", fixture.fixture_id, error.code.as_str(), error.message)
            });
        input_tokens += verdict.usage_input_tokens;
        output_tokens += verdict.usage_output_tokens;
        coverage_graded.push(grade_coverage(fixture, &verdict));
    }
    direct_graded.extend(coverage_graded.clone());
    derived_graded.extend(coverage_graded);

    println!(
        "{}",
        report(
            "criterion-strength v3-direct (fresh v0 sample, same calls as v3-derived)",
            &direct_graded
        )
    );
    let direct_bars = Bars::of(&direct_graded);
    println!("**GATE v3-direct** {}", direct_bars.line());

    println!(
        "{}",
        report("criterion-strength v3-derived (from noul answers)", &derived_graded)
    );
    let derived_bars = Bars::of(&derived_graded);
    println!("**GATE v3-derived** {}", derived_bars.line());
    println!("tokens: {input_tokens} in, {output_tokens} out (one call per weakness fixture, shared by both columns)");

    assert_eq!(direct_graded.len(), 15, "every fixture was graded (direct)");
    assert_eq!(derived_graded.len(), 15, "every fixture was graded (derived)");
    assert!(
        input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    assert!(
        derived_bars.all(),
        "v3-derived does not clear the pre-registered bars: {}",
        derived_bars.line()
    );
}

/// Provenance: PLAT-917 follow-up. **Reported, never gated** -- the
/// diagnostic that says whether `v3`'s failure is the derivation rule or the
/// answers it derives from.
///
/// [`the_lens_with_noul_derived_labels_v3`] scores one composed label per
/// row, so a wrong label cannot say *which* of the five `noul` answers was
/// wrong. This measures each `noul` question on its own against the answer
/// the corpus's reader recorded for it (`labels.falsifiable` and its four
/// siblings, which three fixtures omit -- an omitted id is "no answer
/// recorded", so it is skipped rather than defaulted). It asserts only that
/// rows were compared at all; every rate it prints is a measure.
///
/// Read it beside the constant predictor printed for each question: a
/// question whose answers never vary carries no information, whatever its
/// agreement rate.
#[tokio::test]
async fn the_noul_answers_are_reported_per_question() {
    let client = live_client();
    let questions = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");
    let corpus = corpus();

    // question id -> (agreed, compared, jev said true, reader said true)
    let mut per_question: BTreeMap<&'static str, (u32, u32, u32, u32)> = BTreeMap::new();

    for fixture in &corpus.weakness_kind_fixtures {
        let context = fixture.context();
        let ac_ids = context.ac_ids();
        let request = quoin_jev::lens::build_request(&context, &questions);
        let response = client
            .system_one(request)
            .await
            .map_err(|error| quoin_jev::error::classify(&error))
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} — {}",
                    fixture.fixture_id,
                    error.code.as_str(),
                    error.message
                )
            });
        let ac_id = &ac_ids[0];
        for (id, expected) in fixture.labels.noul() {
            let Some(entry) = questions.noul.iter().find(|entry| entry.id == id) else {
                continue;
            };
            let key = QuestionSet::noul_key(ac_id, entry);
            let Some(typesafe_sdk_answers::Answer::Noul(answer)) = response.answer(&key) else {
                continue;
            };
            let actual = answer.noul >= 0.5;
            let slot = per_question.entry(id).or_default();
            slot.1 += 1;
            if actual == expected {
                slot.0 += 1;
            }
            if actual {
                slot.2 += 1;
            }
            if expected {
                slot.3 += 1;
            }
        }
    }

    println!("\n## noul answers, per question (M2 diagnostic, reported only)\n");
    println!("| question | compared | agreed | agreement | Jev said true | reader said true | best constant |");
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (id, (agreed, compared, jev_true, reader_true)) in &per_question {
        let total = f64::from(*compared);
        let constant = f64::from((*reader_true).max(compared - reader_true)) / total * 100.0;
        println!(
            "| `{id}` | {compared} | {agreed} | {:.1}% | {jev_true} | {reader_true} | {constant:.1}% |",
            f64::from(*agreed) / total * 100.0
        );
    }

    assert!(
        per_question.values().all(|(_, compared, _, _)| *compared > 0),
        "a question with nothing compared means the key or the corpus changed, not that Jev agreed"
    );
    assert!(!per_question.is_empty(), "no noul answer was compared at all");
}
