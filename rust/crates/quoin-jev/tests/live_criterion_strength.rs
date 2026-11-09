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
//!
//! # `v4`, pre-registered before its first call -- follow-up (PLAT-917)
//!
//! Every variant so far has sent the `weakness_kind` question through the
//! SDK's [`typesafe_sdk_questions::choice_of`] builder, which maps each label
//! to `Entry::null()` -- the wire carries the six label *strings* and nothing
//! else. The six labels are in-house jargon: their meanings live in
//! `skills/spec-criterion-strength-analysis/SKILL.md` and in
//! `question-set.json`'s five `noul` question texts, neither of which was ever
//! sent. Jev has been asked to pick between `unfalsifiable`,
//! `unmeasurable_threshold`, `restates_requirement`, `implementation_coupled`
//! and `happy_path_only` from those words alone.
//!
//! The vendor's own documentation for the `choice` primitive
//! (<https://docs.typesafe.ai/primitives/choice.md>) states that `criteria`
//! carries a description per label and exists specifically to disambiguate
//! similar or overlapping options. The SDK exposes it as
//! [`typesafe_sdk_questions::choice`], and [`typesafe_sdk_questions::Entry`]
//! accepts rich JSON anywhere a description is accepted, so a criterion can be
//! a `{what, not_for, examples}` object rather than a sentence.
//!
//! `v4` is that one change and nothing else:
//!
//! | | context | primitive | `weakness_kind` question text |
//! | --- | --- | --- | --- |
//! | `v0` | corpus fields only | `choice_of` (all labels `null`) | shipped |
//! | `v4` | corpus fields only, identical to `v0` | `choice` with described `criteria` | shipped, identical to `v0` |
//!
//! Context stays minimal deliberately. `v1` showed full FR context made the
//! shipped question *worse* (46.7% to 33.3%), and the vendor's own jaggedness
//! notes name "Noisy State -- large irrelevant context acts as a distractor"
//! as a known failure mode. Adding context and adding criteria at once would
//! confound the two.
//!
//! **Provenance of the criteria text.** [`weakness_kind_criteria`] is written
//! from two sources and no others: the five `noul` question texts in
//! `question-set.json` (which already state the concepts -- falsifiability,
//! external observability, stated thresholds, restatement, implementation
//! coupling), and `SKILL.md`'s own severity-mapping prose defining each label.
//! Its `examples` are short generic illustrations written from those
//! definitions. **No fixture text, fixture label or fixture rationale was read
//! while writing it**, because criteria fitted to the answer key they are
//! graded against would make any margin meaningless -- the same discipline
//! [`derive_weakness_kind`] was held to.
//!
//! **Stated risk, before running:** described criteria are more tokens in the
//! request, and the jaggedness note above says more text can itself distract.
//! If `v4` scores below `v0`, that is the reading, and it is a result either
//! way.
//!
//! Bars: identical to every prior variant -- agreement beats the constant
//! predictor, defect recall > 0, `sound` cleared >= 3 of 5. `JEV_RUNS`
//! (default 1) repeats the pass; GO would need the bars cleared on the gate
//! pass and in at least 3 of 5 repeats.
//!
//! # `v4` result, 2026-09-21: NO-GO
//!
//! MEASURED against `jev-latest`, N=5 (five full passes; every pass returned
//! the identical score to one decimal place -- as with `v3`, this corpus is
//! not where service variance lives).
//!
//! | variant | agreement | margin | defect recall | `sound` cleared |
//! | --- | --- | --- | --- | --- |
//! | `v0` | 46.7% | -33.3 pp | 2/2 | 0/5 |
//! | `v1` | 33.3% | -46.7 pp | 2/2 | 0/5 |
//! | `v2` | 60.0-66.7% | -13.3 pp | 1/2 | 4/5 |
//! | `v3-derived` | 33.3% | -46.7 pp | 2/2 | 0/5 |
//! | **`v4`** | **60.0%** | **-20.0 pp** | **5/5** | **0/5** |
//!
//! Two of three bars fail, identically on the gate pass and all 5 repeats:
//! `sound` cleared 0 of 5 every time, tying `v0`/`v1`/`v3` at the floor
//! `v2` alone escaped, despite `v2`'s only change being question wording, not
//! the request primitive. Bar 1 (agreement) improves on every bare-`choice_of`
//! variant (`v0`, `v1`) but still trails the 80.0% constant by 20 points.
//!
//! **Where the described criteria helped and where they did not.** Defect
//! recall goes to 5/5 (from 2/2 -- a larger denominator this pass, not a rate
//! comparison) because `restates_requirement` is now over-predicted: 5 of 15
//! rows, against 1 fixture whose primary reading is that label (20% precision,
//! 4 false positives). Given a described criterion for it, Jev applies
//! `restates_requirement` more readily than before, including to three of the
//! five `sound` fixtures. Description disambiguated the label from the
//! model's own confusion enough to raise its recall, but not enough to raise
//! precision -- the same jaggedness the vendor's own notes name, now visible
//! per-label rather than as a single distractor effect.
//!
//! **This settles the instrument-defect hypothesis, in the negative.** `v4`
//! isolated the one variable `v0`-`v3` left untested -- the bare `choice_of`
//! primitive -- and reproduced `v0`'s exact `sound`-cleared failure (0/5)
//! under a request built the way the vendor's own docs say `choice` should be
//! used. The wrong primitive is not what explains 917: `v2`, still on
//! `choice_of`, is the only variant across five that ever recovers `sound`,
//! by changing the question's wording rather than its wire shape. The
//! remaining lever this ticket has not isolated is `v2`'s wording change
//! combined with described criteria together; nothing tried here tests that
//! combination, and the confound this pre-registration deliberately avoided
//! (context + criteria together) is a different, already-ruled-out
//! combination.
//!
//! # `v5`, pre-registered before its first call -- PLAT-979
//!
//! PLAT-979 allows one wording or decomposition retry. Its method: classify
//! every disagreement as wording, missing sub-question, bad option list,
//! threshold or bad label, in that order, and fix the dominant class first.
//!
//! **Step 1: diagnostic `v2` re-run, 2026-09-22, one pass, reported only.**
//! It scored 60.0%, the same as round two, with six disagreements:
//!
//! | fixture | recorded | `v2` said | class |
//! | --- | --- | --- | --- |
//! | `CS-FIX-003` | `implementation_coupled` | `sound` | wording: `v2`'s `sound` definition omits the coupling check |
//! | `CS-FIX-004` | `unfalsifiable` / `sound` | `unmeasurable_threshold` (0.27) | wording: the label is used with no quantity asserted |
//! | `CS-FIX-005` | `restates_requirement` / `implementation_coupled` | `unmeasurable_threshold` (0.17) | wording: same |
//! | `CS-FIX-006` | `unfalsifiable` / `sound` | `restates_requirement` (0.34) | bad label: the AC paraphrases FR-079-CON-1, which ships as context |
//! | `CS-FIX-008` | `unfalsifiable` / `sound` | `restates_requirement` (0.42) | bad label: "correct nesting" restates the Behaviour bullet on nesting |
//! | `CS-FIX-015` | 2 | 2.91, rounded to 3 | threshold: the coverage rubric's top level is read generously |
//!
//! Wording is the dominant class, with 3 of 6.
//!
//! **`v5` is one change from `v2`**: [`V5_CHOICE_QUESTION`] replaces
//! [`V2_CHOICE_QUESTION`]. It keeps `v2`'s neutral framing and defines each
//! label in one line, restating the `noul` question behind it. It fixes
//! the two wording defects: the `sound` definition now includes all five
//! checks, and `unmeasurable_threshold` requires an asserted quantity. The
//! context (full), primitive (`choice_of`) and coverage question are unchanged
//! from `v2`. This is not `v4`. `v4` kept the shipped question, which asks for
//! a weakness before it offers `sound`, and sent long `{what, not_for,
//! examples}` criteria. It over-fired `restates_requirement` and cleared no
//! `sound` criterion.
//!
//! **Fitted to the corpus.** This wording was written after reading the
//! fixtures, their rationales and step 1's per-row answers. A GO would be
//! weak evidence and would need a held-out corpus. A NO-GO is the strong
//! result.
//!
//! **Prediction, stated before the call.** Fixing all three wording rows and
//! keeping v2's nine correct rows gives 12/15 = 80.0%. That ties the constant
//! and fails bar 1, which needs a strictly greater score. `v5` clears bar 1
//! only if it also gets `CS-FIX-006` or `CS-FIX-008` right, or the coverage
//! row, none of which it targets. Expected outcome: NO-GO on bar 1, with the
//! wording rows `CS-FIX-003`/`004`/`005` corrected. If those three stay wrong,
//! the wording hypothesis is refuted as well.
//!
//! Bars: unchanged. Gate pass (`JEV_VARIANT=v5`, the gate test), then
//! `JEV_RUNS=5` repeats. GO needs all three bars on the gate pass and in at
//! least 3 of 5 repeats.
//!
//! # `v5` result, 2026-09-22: NO-GO
//!
//! MEASURED against `jev-latest`: one gate pass, then 5 repeats. M1
//! disagreement was 5.3% (range 0.0-13.3%).
//!
//! | pass | agreement | margin | defect recall | `sound` cleared | all bars |
//! | --- | --- | --- | --- | --- | --- |
//! | gate | 73.3% | -6.7 pp | 1/2 | 2/5 | FAIL |
//! | repeats 1-4 | 73.3% | -6.7 pp | 1/2 | 2/5 | FAIL |
//! | repeat 5 | 80.0% | 0.0 pp (tie) | 1/2 | 3/5 | FAIL (bar 1 needs strictly more) |
//!
//! No pass cleared all three bars, against the 3 of 5 GO requires. 73.3% is
//! the highest agreement of any variant, and the 80.0% repeat is the first
//! pass by any variant to reach the constant. Neither beats it.
//!
//! **The fix worked on the rows it targeted.** `CS-FIX-003` is now
//! `implementation_coupled` (0.94). `CS-FIX-004` is `sound`, an accepted
//! reading. The two bad-label rows, `CS-FIX-006` and `CS-FIX-008`, went to
//! `sound`, which is accepted too. `CS-FIX-014` is `happy_path_only`, the
//! first time any variant returned that primary reading.
//!
//! **The errors moved instead of going away.** The gate pass's four
//! disagreements:
//!
//! | fixture | recorded | `v5` said | class |
//! | --- | --- | --- | --- |
//! | `CS-FIX-001` | `sound` | `implementation_coupled` (0.37) | wording, over-corrected: flags the two `cargo` commands the NFR gates on |
//! | `CS-FIX-002` | `sound` | `implementation_coupled` (0.57) | wording, over-corrected: flags `parse_document`, the API the NFR bounds |
//! | `CS-FIX-005` | `restates_requirement` / `implementation_coupled` | `sound` | wording: "Error Boundary" is not read as a mechanism |
//! | `CS-FIX-015` | 2 | 2.92, rounded to 3 | threshold: unchanged from `v2` |
//!
//! Adding the coupling check to `sound` fixed `CS-FIX-003`. It also flagged
//! two clean `sound` criteria, even though the definition says outright that
//! a public command or API under test is not internal. So bar 3, which `v2`
//! passed, now fails. Across `v0`-`v5` the wording lever trades one error for
//! another. `v0`/`v1`/`v3`/`v4` never clear `sound`. `v2` clears it and
//! misses the coupled defect. `v5` catches the defect and flags sound
//! criteria. Coupling is the dimension no wording has separated:
//! `CS-FIX-001`/`002`/`007` (public surface) against `003`/`005` (internal).
//! The rationales make that call from knowledge of each repo's public API,
//! which the criterion text does not carry.
//!
//! **The NO-GO is final for this corpus and question shape.** PLAT-979's
//! one wording retry has been made, aimed at the dominant class from a
//! per-row diagnostic, fitted to the corpus. It still fails. Beating the
//! constant here needs at most two errors in fifteen. The coverage row
//! `CS-FIX-015` was wrong in both passes whose per-row table was kept
//! (step 1's `v2` and `v5`'s gate pass). That leaves one error for eleven
//! criteria, and the answer key contests seven of their readings.

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
    reason = "every cast in this file converts a request/latency/token count (never \
              realistically exceeding a few thousand) to or from f64 for a printed ratio or a \
              percentile index -- none crosses a wire or persistence boundary, which is what \
              this workspace's cast lints exist to guard. PLAT-838 found these had never \
              actually been checked under `make rust-lint`'s `--all-features`, since `live-api` \
              has no default-gate coverage; fixed here rather than left for the next live-api \
              change to trip over."
)]

mod support;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use quoin_jev::{BoundedContext, ContextPolicy, FrVerdict, JevErrorCode, QuestionSet};
use support::{
    Graded, Verdict, corpus, defect_recall, disagreement, grade_coverage, grade_weakness, report,
    sound_recall, tally, trivial_baseline,
};

/// The certainty thresholds passed to `verdict::extract`.
///
/// Confidence 0.7 is the crate's own unmeasured default, recorded as such in
/// PR #571. Margin 0.0 turns PLAT-981's top-two margin gate off (no gap is
/// strictly below zero), which is the behaviour every run recorded in this
/// file was measured under. Both only set how a finding's `certainty` is
/// annotated; neither suppresses one, so they cannot change which rows this
/// file grades -- only how they print. Resetting them from the calibration
/// curve is M3's job.
const THRESHOLDS: quoin_jev::Thresholds = quoin_jev::Thresholds {
    confidence: 0.7,
    margin: 0.0,
};

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

/// The `weakness_kind` question `v5` sends instead of `v2`'s (PLAT-979).
///
/// `v2`'s question, with a one-line definition for each label. It fixes the
/// wording class found in `v2`'s disagreements: `v2`'s definition of `sound`
/// left out the implementation-coupling check, and no label said that
/// `unmeasurable_threshold` needs a quantity to be asserted first. Every
/// definition restates a `noul` question from `question-set.json`. See the
/// module doc.
const V5_CHOICE_QUESTION: &str = "Which one label best describes this acceptance criterion as \
written? Apply these definitions literally. \
`sound`: all five hold -- a concrete system behaviour could make it false; it names an outcome \
observable from outside; any quantity it asserts has its number stated; it is not the FR sentence \
with `shall` swapped out; and it names no internal symbol, private field or call sequence. \
`unfalsifiable`: no concrete system behaviour could make it false. \
`unmeasurable_threshold`: it asserts a quantity (a duration, size, count, rate or limit) but never \
states the number. A criterion that asserts no quantity is never this label. \
`restates_requirement`: it is an FR sentence with `shall` swapped out and adds no detail of its \
own. \
`implementation_coupled`: it names an internal symbol, private field or call sequence, including \
anything the criterion itself calls internal. The public command, API or file a requirement \
exists to constrain is not internal. \
`happy_path_only`: it covers only the success path, and neither it nor its FR states any error, \
boundary or negative case. \
When none of the five weaknesses applies, answer `sound`.";

/// Which pre-registered variant this run measures. See the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// Corpus fields only, shipped question.
    V0,
    /// Full FR context, shipped question.
    V1,
    /// Full FR context, neutral question.
    V2,
    /// Full FR context, neutral question with every label defined (PLAT-979).
    V5,
}

impl Variant {
    /// Reads `JEV_VARIANT`. Panics on anything but the known ids, so a typo
    /// cannot silently measure the default.
    fn from_env() -> Self {
        match std::env::var("JEV_VARIANT").as_deref() {
            Err(_) | Ok("v1") => Self::V1,
            Ok("v0") => Self::V0,
            Ok("v2") => Self::V2,
            Ok("v5") => Self::V5,
            Ok(other) => panic!("JEV_VARIANT must be v0, v1, v2 or v5; got {other:?}"),
        }
    }

    fn full_context(self) -> bool {
        self != Self::V0
    }

    /// The bounding each variant was measured under (PLAT-983). `v0` sent
    /// the corpus's short fields, which the default policy passes unchanged.
    /// Every full-context variant sent whole sections, so it lifts both caps
    /// -- under the default it would silently stop being the variant it
    /// names.
    fn policy(self) -> ContextPolicy {
        if self.full_context() {
            ContextPolicy::default()
                .with_max_prose_bytes(usize::MAX)
                .with_max_statement_bytes(usize::MAX)
        } else {
            ContextPolicy::default()
        }
    }

    fn questions(self) -> QuestionSet {
        let mut set = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");
        match self {
            Self::V0 | Self::V1 => {}
            Self::V2 => V2_CHOICE_QUESTION.clone_into(&mut set.choice.question),
            Self::V5 => V5_CHOICE_QUESTION.clone_into(&mut set.choice.question),
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
    let noul = quoin_jev::verdict::NoulSignals {
        values: noul_values,
    };
    FrVerdict {
        classifier: "derived-from-noul".to_owned(),
        usage_input_tokens: 0,
        usage_output_tokens: 0,
        findings: vec![quoin_jev::Finding {
            ac_id: ac_id.to_owned(),
            weakness_kind: label.to_owned(),
            severity,
            confidence: 1.0,
            certainty: quoin_jev::Certainty::Confident,
            probabilities: Vec::new(),
            label_sub_question: quoin_jev::SubQuestionCheck::for_label(label, &noul),
            noul,
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
            defect_recall: defect_recall(graded, "sound")
                .expect("the corpus holds unambiguous defects"),
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
    context: &BoundedContext,
    questions: &QuestionSet,
    fixture_id: &str,
) -> (FrVerdict, Duration) {
    let started = Instant::now();
    let verdict = quoin_jev::lens::run(client, context, questions, THRESHOLDS)
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
        }
        .bound(&variant.policy());
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
        }
        .bound(&variant.policy());
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
            &pass.graded,
            "sound"
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
    }
    .bound(&variant.policy());
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

/// The five `noul` answers one response carries for `ac_id`, by question id.
/// A question the response did not answer is absent, never defaulted.
fn noul_answers(
    response: &typesafe_sdk_answers::SystemOneResponse,
    questions: &QuestionSet,
    ac_id: &str,
) -> Vec<(String, f64)> {
    questions
        .noul
        .iter()
        .filter_map(
            |entry| match response.answer(&QuestionSet::noul_key(ac_id, entry)) {
                Some(typesafe_sdk_answers::Answer::Noul(answer)) => {
                    Some((entry.id.clone(), answer.noul))
                }
                _ => None,
            },
        )
        .collect()
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
        let context = fixture.context().bound(&ContextPolicy::default());
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
            quoin_jev::verdict::extract(&response, &questions, &ac_ids, THRESHOLDS);
        direct_graded.push(grade_weakness(fixture, &direct_verdict));

        // v3-derived: the five `noul` answers from the same response, run
        // through `derive_weakness_kind` instead.
        let ac_id = &ac_ids[0];
        let noul_values = noul_answers(&response, &questions, ac_id);
        let label = derive_weakness_kind(&noul_values);
        let derived_verdict = synthetic_verdict(ac_id, &label, noul_values);
        derived_graded.push(grade_weakness(fixture, &derived_verdict));
    }

    // Coverage rows are outside this hypothesis (a `score` question, not
    // `choice`) -- computed once and shared by both columns, so the
    // comparison below isolates only the weakness_kind change.
    let mut coverage_graded = Vec::with_capacity(4);
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let context = fixture.context().bound(&ContextPolicy::default());
        let verdict = quoin_jev::lens::run(&client, &context, &questions, THRESHOLDS)
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
        output_tokens += verdict.usage_output_tokens;
        coverage_graded.push(grade_coverage(fixture, &verdict));
    }
    direct_graded.extend(coverage_graded.clone());
    derived_graded.extend(coverage_graded);

    println!(
        "{}",
        report(
            "criterion-strength v3-direct (fresh v0 sample, same calls as v3-derived)",
            &direct_graded,
            "sound"
        )
    );
    let direct_bars = Bars::of(&direct_graded);
    println!("**GATE v3-direct** {}", direct_bars.line());

    println!(
        "{}",
        report(
            "criterion-strength v3-derived (from noul answers)",
            &derived_graded,
            "sound"
        )
    );
    let derived_bars = Bars::of(&derived_graded);
    println!("**GATE v3-derived** {}", derived_bars.line());
    println!(
        "tokens: {input_tokens} in, {output_tokens} out (one call per weakness fixture, shared by both columns)"
    );

    assert_eq!(direct_graded.len(), 15, "every fixture was graded (direct)");
    assert_eq!(
        derived_graded.len(),
        15,
        "every fixture was graded (derived)"
    );
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

/// `v4`'s described `choice` criteria: one `{what, not_for, examples}` object
/// per label, in the answer space's own order.
///
/// See this file's module doc for where every sentence here comes from and
/// why no fixture was read while writing it.
fn weakness_kind_criteria() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "sound",
            serde_json::json!({
                "what": "No weakness applies. A concrete system behaviour would make this \
                         criterion false; it names an outcome observable from outside rather \
                         than a property of the implementation; any quantity it asserts is \
                         stated as an actual number; and it says something the FR sentence it \
                         belongs to does not already say.",
                "not_for": "A criterion that fails any one of those four checks. Pick the \
                            weakness that applies instead.",
                "examples": [
                    "The command exits with status 2 and prints no output when the input file is absent.",
                    "A report file exists at the configured path within 5 seconds of the run completing."
                ]
            }),
        ),
        (
            "unfalsifiable",
            serde_json::json!({
                "what": "There is no concrete system behaviour that would make this criterion \
                         false. Nothing anyone could observe would contradict it, so a test \
                         matrix row backed by it proves nothing.",
                "not_for": "A criterion that could fail but is vague about a number -- that is \
                            `unmeasurable_threshold`. A criterion that could fail but merely \
                            repeats the FR sentence -- that is `restates_requirement`.",
                "examples": [
                    "The system behaves correctly under load.",
                    "Errors are handled appropriately.",
                    "The output is of acceptable quality."
                ]
            }),
        ),
        (
            "unmeasurable_threshold",
            serde_json::json!({
                "what": "The criterion asserts a quantity -- a duration, rate, size, count or \
                         limit -- but the number itself is never stated, so there is no value to \
                         measure against.",
                "not_for": "A criterion that asserts no quantity at all. If nothing else is \
                            wrong with such a criterion it is `sound`; if nothing could falsify \
                            it at all it is `unfalsifiable`.",
                "examples": [
                    "The response returns quickly.",
                    "Memory use stays within acceptable bounds.",
                    "Retries stop after a reasonable number of attempts."
                ]
            }),
        ),
        (
            "restates_requirement",
            serde_json::json!({
                "what": "This is the FR sentence with `shall` swapped out. It adds no \
                         discriminating power beyond the requirement it belongs to: anything \
                         satisfying the FR satisfies this criterion automatically.",
                "not_for": "A criterion that adds an observable detail, a number, a precondition \
                            or an error case the FR sentence does not itself carry.",
                "examples": [
                    "FR: The system SHALL emit a report after each run. AC: A report is emitted after each run.",
                    "FR: The parser SHALL reject malformed input. AC: Malformed input is rejected."
                ]
            }),
        ),
        (
            "implementation_coupled",
            serde_json::json!({
                "what": "The criterion names an internal symbol, a private field, or a call \
                         sequence. Such a criterion is satisfiable by exactly one implementation \
                         and passes forever once written, surviving refactors that should have \
                         broken it. This is over-specification, a distinct defect from vagueness.",
                "not_for": "A criterion naming something externally observable -- a written file, \
                            an exit code, a response field, a CLI flag, a log line a user can \
                            read. Naming a public artifact is an observable outcome, not \
                            implementation coupling.",
                "examples": [
                    "`Parser::parse_inner` is called before `validate`.",
                    "The private `cache` field is reset to `None` on shutdown.",
                    "The handler invokes `normalize()` then `persist()` in that order."
                ]
            }),
        ),
        (
            "happy_path_only",
            serde_json::json!({
                "what": "The criterion exercises only the successful path. It states what happens \
                         when everything goes right and covers no error case, no boundary or edge \
                         case, and no explicit negative case.",
                "not_for": "A criterion that does state an error, a boundary or a negative \
                            condition. Also not for defects of wording or precision -- those are \
                            the other labels; this one is about which cases are covered.",
                "examples": [
                    "A valid request returns 200 and the created record.",
                    "The file is written successfully when the directory exists."
                ]
            }),
        ),
    ]
}

/// `v4`'s question map for one FR: the shipped five `noul` and the shipped
/// `score`, with the `weakness_kind` question rebuilt through
/// [`typesafe_sdk_questions::choice`] and described criteria.
///
/// Keys are produced by [`QuestionSet`]'s own key functions, so
/// [`quoin_jev::verdict::extract`] reads this response exactly as it reads a
/// shipped one -- there is no second parsing path.
fn v4_questions_for_fr(set: &QuestionSet, ac_ids: &[String]) -> typesafe_sdk_questions::Questions {
    let mut questions = typesafe_sdk_questions::Questions::new();
    for ac_id in ac_ids {
        for entry in &set.noul {
            questions.insert(
                QuestionSet::noul_key(ac_id, entry),
                typesafe_sdk_questions::noul(entry.question.as_str()),
            );
        }
        // The answer space and the criteria map must name the same six labels,
        // or the request would silently offer a different vocabulary than the
        // one `verdict::extract` validates answers against.
        let criteria = weakness_kind_criteria();
        let described: Vec<&str> = criteria.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            described, set.choice.answer_space,
            "v4's criteria must describe exactly the shipped answer space, in its order"
        );
        questions.insert(
            set.weakness_kind_key(ac_id),
            typesafe_sdk_questions::choice(
                set.choice.question.as_str(),
                criteria.into_iter().map(|(label, description)| {
                    (label, typesafe_sdk_questions::Entry(description))
                }),
            ),
        );
    }
    let (key, question) = set.adverse_case_coverage_question();
    questions.insert(key, question);
    questions
}

/// One `v4` pass over the whole corpus, graded.
async fn run_once_v4(client: &Client, set: &QuestionSet) -> (Vec<Graded>, u64, u64) {
    let corpus = corpus();
    let mut graded = Vec::with_capacity(15);
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;

    for fixture in &corpus.weakness_kind_fixtures {
        // `context()`, not `context_full()`: v4 holds v0's minimal context
        // fixed and varies only the primitive.
        let context = fixture.context().bound(&ContextPolicy::default());
        let ac_ids = context.ac_ids();
        let request = typesafe_sdk_client::SystemOneRequest::new(
            typesafe_sdk_questions::Entry::from(&context),
            v4_questions_for_fr(set, &ac_ids),
        );
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
        graded.push(grade_weakness(
            fixture,
            &quoin_jev::verdict::extract(&response, set, &ac_ids, THRESHOLDS),
        ));
    }

    // Coverage rows are a `score` question, untouched by this hypothesis, and
    // are sent exactly as v0 sends them.
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let context = fixture.context().bound(&ContextPolicy::default());
        let verdict = quoin_jev::lens::run(client, &context, set, THRESHOLDS)
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
        output_tokens += verdict.usage_output_tokens;
        graded.push(grade_coverage(fixture, &verdict));
    }

    (graded, input_tokens, output_tokens)
}

/// Provenance: PLAT-917 follow-up. **`v4`, pre-registered before its first
/// call.** The shipped question and v0's minimal context, sent through the
/// `choice` primitive with a described `{what, not_for, examples}` criterion
/// per label instead of `choice_of`'s bare label strings.
///
/// Gates on the same three bars every prior variant was held to; `JEV_RUNS`
/// (default 1) repeats the pass and reports how many cleared.
#[tokio::test]
async fn the_lens_with_described_choice_criteria_v4() {
    let runs: usize = std::env::var("JEV_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    assert!(runs >= 1, "a variant needs at least one pass");

    let client = live_client();
    let set = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");

    let mut passes = Vec::with_capacity(runs);
    let mut cleared = 0;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    for run in 1..=runs {
        let (graded, input, output) = run_once_v4(&client, &set).await;
        input_tokens += input;
        output_tokens += output;
        let bars = Bars::of(&graded);
        println!("run {run} (v4): {}", bars.line());
        if bars.all() {
            cleared += 1;
        }
        passes.push(graded);
    }

    let first = &passes[0];
    println!(
        "{}",
        report(
            "criterion-strength v4 (described choice criteria)",
            first,
            "sound"
        )
    );
    let bars = Bars::of(first);
    println!("**GATE v4** {}", bars.line());
    println!("**Bars cleared** in {cleared} of {runs} run(s) for v4 (GO needs at least 3 of 5).");
    println!(
        "tokens: {input_tokens} in, {output_tokens} out over {runs} pass(es) \
         ({:.0} in/request)",
        input_tokens as f64 / (15.0 * runs as f64)
    );

    // The call worked and the crate understood the answer, before any accuracy
    // bar is read off a possibly-empty denominator.
    assert_eq!(first.len(), 15, "every fixture was graded");
    let unanswered: Vec<&str> = first
        .iter()
        .filter(|row| row.verdict == Verdict::Unanswered)
        .map(|row| row.fixture_id.as_str())
        .collect();
    assert!(
        unanswered.is_empty(),
        "the service left fixtures unanswered: {unanswered:?}"
    );
    let unrecognized: Vec<(&str, &str)> = first
        .iter()
        .filter(|row| row.verdict == Verdict::Unrecognized)
        .map(|row| (row.fixture_id.as_str(), row.actual.as_str()))
        .collect();
    assert!(
        unrecognized.is_empty(),
        "labels outside the declared answer_space: {unrecognized:?}"
    );
    assert!(
        input_tokens > 0,
        "a pass that consumed no input tokens never reached the service"
    );

    assert!(
        bars.all(),
        "v4 does not clear the pre-registered bars: {}",
        bars.line()
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
        let context = fixture.context().bound(&ContextPolicy::default());
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
    println!(
        "| question | compared | agreed | agreement | Jev said true | reader said true | best constant |"
    );
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
        per_question
            .values()
            .all(|(_, compared, _, _)| *compared > 0),
        "a question with nothing compared means the key or the corpus changed, not that Jev agreed"
    );
    assert!(
        !per_question.is_empty(),
        "no noul answer was compared at all"
    );
}
