---
id: MP-242
title: Jev test_asserts_intent variants T1-T2 and the trace check TC on corpus v2
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.test_intent_and_trace_margin
definition_version: jev.test-intent-trace-v1
ground_truth_kind: agent-labelled
relationships: []
---

# Jev test_asserts_intent variants T1-T2 and the trace check TC on corpus v2

## Decision Use

Decide which wording of `test_asserts_intent`, if any, carries signal on
corpus v2, and whether a separate trace check can tell a wrong trace from a
real semantic failure. The result picks at most one variant for the single
held-out run. This plan assigns no GO/NO-GO for wiring anything into the
gap-analysis lens. That decision is MP-230's.

PLAT-839 asked `test_asserts_intent` as one `noul` and it tied its constant
predictor at 75.0%. 18 of that corpus's 29 rows cited a requirement that was
not about the code, so the question mostly measured whether the trace was
right. Corpus v2 (PLAT-1025, pending) samples by a written rule, carries
mutation truth, and includes requirement-plus-test rows with no code (`RT`),
so a spec and its tests can be checked before the code exists (PLAT-1024
scope ruling).

The variants, registered in
`rust/crates/quoin-jev/tests/eval_v2_support/variants/intent.rs`:

| id | modes | asks | graded key |
| --- | --- | --- | --- |
| `T0@v1` | RTC | `FullBatteryV1` as shipped (baseline, PLAT-1027) | `test_asserts_intent` |
| `T0-RT@v1` | RT | `FullBatteryV1`'s `test_asserts_intent` question, asked alone; its wording differs from T0 only by "the code" -> "the system" (twice) | `test_asserts_intent` |
| `T1@v1` | RT, RTC | a `noul` `exercises_criterion` ("does the test drive the behaviour the criterion names at all") and a seven-label `choice` `check_kind` ("what do its assertions check") | `test_asserts_intent`, derived |
| `T2@v1` | RT, RTC | T1 with 5 and 7 synthetic worked examples in the two instructions | `test_asserts_intent`, derived |
| `TC-RT@v1`, `TC-RC@v1`, `TC-RTC@v1` | one mode each | a `noul` "does the cited requirement describe the behaviour exercised by this test / implemented by this code / of this test and/or code" | `trace_correct` |

**T0 and T0-RT differ in context as well as wording.** T0 asks its question
inside the seven-question `FullBatteryV1` request; T0-RT asks it alone. A
difference between the two mixes the one-word edit with the missing battery
context, and is not read as the effect of either.

There is no T3. The other shape the research points to is asking the
question as the mechanical truth's counterfactual ("if the behaviour were
broken, would this test fail?"). That is `FullBatteryV1`'s wording, which T0
already is, and it tied. A variant with no new evidence behind it would only
add a candidate to the selection below.

The three TC variants are one instrument. They differ only in naming what the
row's mode carries, and are pooled as "TC" below. In RTC, the trace is right
when the requirement describes the behaviour of the test, of the code, or of
both, matching the corpus's `trace_correct` rule ("test and/or code").

## Dependencies

Bar D and the `mutation.source_id` field it pairs by are implemented on this
branch (`intent::bar_d`), because no shared paired-contrast helper is on
`main`. Pinning the request digests and the first dev run are blocked until
both of these are on `main`. Neither is on `main` today:

- The corpus itself, `rust/crates/quoin-jev/tests/fixtures/eval-v2/corpus.json`,
  from PLAT-1025 (pending).
- The held-out selection file
  `rust/crates/quoin-jev/tests/fixtures/eval-v2/heldout-selection.json` and
  per-variant request digest pinning, introduced by PLAT-1028's PR #620 review
  fixes (pending).

**Bar D qualification (path 2) also needs PLAT-1025's mutant top-up.** The
corpus as first sealed has fewer than the 10 `yes`-sourced dev mutants Bar D
needs, for both `test_weakening` and `trace_swap`, and no RT-sourced
`test_weakening` mutants at all. PLAT-1025 is adding more dev pairs of both
kinds, RT-sourced `test_weakening` among them. Until that top-up is sealed,
Bar D is computed and reported, and path 2 is not gateable for any variant:
it cannot qualify one. Path 1 is unaffected.

## Population

The dev split of corpus v2 as sealed by PLAT-1025, in-repo rows plus any
external rows loaded by reference. The run records the held-out seal digest
of every source it read.

- `test_asserts_intent`, T1 and T2: every dev row in mode `RT` or `RTC` that
  carries truth for the key.
- `test_asserts_intent`, T0 and T0-RT: T0 on the `RTC` subset of the same
  rows, T0-RT on the `RT` subset. Together they are the pre-registered baseline
  for T1 and T2 across both modes. Neither is gated, and neither is a candidate
  for selection.
- `trace_correct`, TC: every dev row in mode `RT`, `RC` or `RTC` that carries
  truth for the key.

**Natural rows only for agent-written truth.** A mutant row whose label for a
key is agent-written (inherited from its source row) never counts, in any
slice. So the AGENT-LABELLED slice and the all-rows slice hold natural rows'
agent labels plus mutants' own mechanical or by-construction labels, and
nothing inherited. The rule applies to every key, `trace_correct` included.
The split-by-trace section of the report applies it (`intent::counted`) and
holds every table MP-242 is judged on:

- For each of T0, T0-RT, T1 and T2, a table headed
  `<variant> | counted, pooled`: that variant's rows over all its modes,
  unsplit, with lines for all rows, each mode and each truth kind. Bars A and
  B are read from it.
- For TC, a table headed `TC-RT@v1 + TC-RC@v1 + TC-RTC@v1 | counted, pooled`:
  the three TC variants pooled over RT, RC and RTC, with a line per mode
  (each mode's own TC variant) and per truth kind. Bar C is read from it.

The harness's generic per-variant tables, printed before that section, do not
apply the rule. No MP-242 number is read from them.

No row is dropped after a live call. A row a variant abstains on stays in its
denominator.

## Measure Definition

Definition `jev.test-intent-trace-v1`, computed by
`rust/crates/quoin-jev/tests/eval_v2_support/metrics.rs` over the shared maths
in `rust/crates/quoin-jev/tests/support/grading.rs`:

- **Agreement**: the share of rows whose answer matches the primary truth label
  or a recorded contested alternative.
- **Constant predictor**: the best single label for that population, computed
  from the labels, with the same contested credit. **Margin** is agreement
  minus the constant predictor, in percentage points.
- **Per-class recall**: of rows labelled `yes`, the share answered `yes`; of
  rows labelled `no`, the share answered `no`.
- **ECE**: expected calibration error over stated confidences, mean confidence
  per decile against accuracy. Rows with no confidence are counted and shown.
- **Split by trace** (T0, T0-RT, T1, T2): the same measures on three slices,
  split by TC's answer on the row: trace right, trace wrong, no trace
  judgment, printed when TC is in the run. They are printed again, whether or
  not TC ran, split by the row's own `trace_correct` label into trace right,
  trace wrong, contested and no label. That label is often agent-written, so
  the split is marked AGENT-LABELLED where it applies. It is not ground truth.

Derive rules, fixed before any dev data:

- TC: `trace_correct = yes` iff its `noul` is at least 0.5.
- T1 and T2: `P(yes) = min(P(exercises_criterion), P(asserts_required_outcome))`,
  and `test_asserts_intent = yes` iff `P(yes) >= 0.5`. The confidence is
  `P(yes)` for `yes` and `1 - P(yes)` for `no`, so it always agrees with the
  answer. Bar D reads the same `P(yes)`. The threshold is 0.5 because every
  other `noul` in the harness uses it, because a threshold tuned on dev would
  be a free parameter this plan cannot account for, and because Lyon
  (notebooks 10/12) found that a misread question returns a confident wrong
  answer, which no threshold repairs.

`check_kind`'s labels, in precedence order. When a test's assertions fit
several, the earliest wins ("the strongest thing any assertion checks"), and
the instruction says so:

1. `asserts_required_outcome`
2. `asserts_outcome_too_loosely`: checks the named outcome, but accepts wrong
   values, a weaker bound, or only its presence
3. `asserts_unrelated`
4. `asserts_only_that_it_runs`
5. `asserts_only_own_setup`
6. `asserts_constant_or_restatement`: a hard-coded literal or a value that
   would hold whatever the behaviour is
7. `cannot_tell`

The order is how much of the system's real behaviour an assertion pins down.
Every label but the first maps to `no`, through `P(asserts_required_outcome)`.

**Abstention and malformed responses are different things.** T1 or T2
abstains only when the rule cannot decide: `check_kind = cannot_tell` while
`P(exercises) >= 0.5`. With `P(exercises) < 0.5` the answer is `no` whatever
`check_kind` says. An abstention stays in every denominator, and the report
counts it. A response the derive rule cannot read is not an abstention. That
covers a missing answer, a `check_kind` label or probability key outside the
seven labels, no probability for `asserts_required_outcome`, and a
probability outside [0, 1]. On any of these the run stops and names the row.
A malformed response is never scored as unanswered.

## Collection Procedure

`rust/crates/quoin-jev/tests/live_eval_v2.rs` (`live-api` feature) with
`QUOIN_JEV_VARIANTS=T0,T0-RT,T1,T2,TC-RT,TC-RC,TC-RTC` and
`QUOIN_JEV_SPLIT=dev`, with `QUOIN_JEV_CASSETTE` and `QUOIN_JEV_MODEL` set. The
runner refuses to start a live run that selects any of MP-242's own variants
(T0-RT, T1, T2 and the three TC variants) without a cassette
(`intent::require_cassette`). T0 alone does not need one: it is PLAT-1027's
shared baseline and other experiments run it. The cassette appends each
answer as it arrives, so if a malformed response stops the run, the rerun
replays every answer already received and pays only for the rest. The report
prints every measure above by mode and by truth kind, then the split-by-trace
section.

A held-out run is entered on the held-out log before its first request
(`corpus::HeldoutSpend`). If it stops part way, on a malformed response or a
transport error, the entry is still written, with no models, and a line on
stderr says so. A second attempt is then refused without
`QUOIN_JEV_HELDOUT_RERUN`, exactly as after a finished run.

The offline test `rust/crates/quoin-jev/tests/tc_1030_intent_variants.rs` runs
in the default gate. It checks:

- T1's derive rule against worked values and over a grid, for agreement
  between answer, confidence and `P(yes)`.
- When `cannot_tell` abstains, and when it does not.
- That each kind of malformed response is an error that stops the runner and
  names the row.
- The label precedence order and the constant label's wording.
- That the paired comparison with the baseline counts only rows both
  answered.
- That inherited agent labels on mutants are not counted, for
  `test_asserts_intent` and for `trace_correct` in TC's pooled table, and
  that contested trace labels are their own slice.
- That MP-242's own variants need a cassette and T0 alone does not.
- That a held-out run stopped by a malformed response is still on the
  held-out log.
- Bar D: each pair outcome against worked values, the sign test against
  hand-computed binomial tails, and pairing by `mutation.source_id` (RTC row
  preferred, sources not labelled `yes` and null sources left out, an
  abstention a failure, a missing source or a mutant source an error). For
  TC: only `trace_swap` mutants pair, and the three TC variants pool.
- That T1 and T2 send identical questions in `RT` and `RTC` and never name the
  code.
- That each TC variant names exactly its mode's artifacts: RT's question
  never mentions code, and RC's never mentions a test, a check or coverage.
- That T0-RT is T0's wording with only the code references removed.
- That T2's examples are present and cite no quoin FR, AC or corpus id.
- The split-by-trace report end to end, with and without TC, over a scripted
  fake Jev.

## Environment and Sampling

The whole dev split, one pass per variant. The model is pinned by
`QUOIN_JEV_MODEL`, and the run records which model answered every request.
Replications are reported as replications and are not pooled.

## Interpretation

**Truth kinds are reported apart.** Mechanical and by-construction truth come
from mutations whose effect was checked by running the owning test, or which
are true by the edit that made the row. Natural rows carry two independent
agent labels (`agent_dual`, or `agent_contested` with both readings kept).
Every slice with agent-written truth is marked AGENT-LABELLED in the report,
and a restatement of any number here that drops that mark is a misreport.
Selection is decided on known truth only (below). `ground_truth_kind` in this
plan's header is `agent-labelled` because the reported populations contain
agent truth, and the header has no spelling for "mechanical plus
by-construction".

**The TC slices are chosen by the model's own answer.** A variant's numbers on
"TC says trace right" rows are not a random subset. They show how the variant
and TC behave together, which is how a gated lens would run. The label split
shows the same variant against the corpus's label instead, which carries its
own labelling errors where it is agent-written.

**T1's exercise question and TC overlap.** TC asks whether the requirement is
about the behaviour exercised at all. T1's `noul` asks whether the test drives
the situation the criterion names. On a mis-traced row both should say no.
They are asked in separate requests, so neither answer sees the other.

**Small slices.** Each slice reports its own row count. A margin on a slice of
under 20 rows is shown and not read.

## Comparison and Enforcement

Bars, pre-registered here before the first live call of any variant in this
plan. Each is judged on the dev split, per variant, over that variant's own
population:

1. **Bar A, margin > 0 pp** (T1, T2): `test_asserts_intent` agreement exceeds
   the constant predictor, over RT and RTC pooled.
2. **Bar B, both class recalls > 0%** (T1, T2): at least one `yes` row answered
   `yes` and at least one `no` row answered `no`.
3. **Bar C, TC margin > 0 pp**: `trace_correct` agreement over RT, RC and RTC
   pooled exceeds its constant predictor.
4. **Bar D, paired contrast** (T1, T2 on `test_asserts_intent`; TC on
   `trace_correct`), implemented by `intent::bar_d`. Pairs come only from
   `mutation.source_id`: a dev mutant and its unmutated source row, which
   share a split. A mutant whose `source_id` names a row that is not in the
   run, or is itself a mutant, stops the report.
   - T1 and T2 pair mutants with `mutation.kind == test_weakening` only;
     `requirement_text` mutants are excluded. TC pairs mutants with
     `mutation.kind == trace_swap` only.
   - One pair per mutation id, never one per row. When a mutation has an RTC
     row and the variant runs in RTC, the RTC row is used; otherwise the
     mutation's first row in a mode the variant runs in.
   - A mutation is unpairable, and counted as such, when its `source_id` is
     null or the variant never runs in its source row's mode. A `trace_swap`
     mutant shares its source's mode. A `test_weakening` mutant may not: an
     RT mutant can be made from an RTC source. T0-RT runs in RT only, so it
     has pairs only from RT-sourced `test_weakening` mutants, and has none
     until the corpus carries some.
   - TC is one instrument (above), so its D pools TC-RT, TC-RC and TC-RTC:
     each trace-swap pair is answered by its mode's TC variant, and the pairs
     from all three modes are tested together.
   - P(yes) should drop from source to mutant. P(yes) is each prediction's
     ordinal: for T1 and T2 the derived
     `min(P(exercises), P(asserts_required_outcome))`, and for T0, T0-RT and
     TC the `noul`.
   - A pair whose source row is not labelled `yes` for the key (already on the
     defect side, or unlabelled) is excluded and counted.
   - A pair **succeeds** iff P(yes) crosses τ = 0.5 downwards (the source at
     or above τ, so answered `yes`; the mutant below τ, so answered `no`) AND
     fell by at least δ = 0.10. It **fails** iff P(yes) rose by at least δ.
     Anything else is a tie and is excluded.
   - A pair where the variant abstains on either row counts as a failure, so
     abstaining can never help D.
   - D passes a one-sided sign test at α = 0.05 over the non-tie pairs
     (successes against failures, null p = 0.5, pass iff p <= α). It is
     gateable with at least 10 non-tie pairs. τ, δ and α are fixed now and
     are not tuned.
   - D is computed for T0 and T0-RT too, reported and not gated. The report
     prints each variant's D after its paired line, and TC's pooled D at the
     end of the split-by-trace section.

**Which rows the bars are judged on.** A variant qualifies only on known
truth: Bars A to C on its mechanical and by-construction slice when that slice
is gateable, or Bar D. For Bars A to C, gateable means the slice has at least
20 rows, including at least one row labelled `yes` and one labelled `no`. The
bars are also computed on all rows and on the AGENT-LABELLED slice (natural
rows only, as above), and both are reported. All three slices are lines of
the `counted, pooled` tables named under Population: the `all` line, the
`mechanical` and `by-construction` lines, and the `AGENT-LABELLED` line.
Three outcomes are possible:

- **Qualifies:** by either of two known-truth paths. Path 1: the slice is
  gateable and the variant's bars (A and B, or C for TC) hold on it. Path 2:
  Bar D is gateable and passes. Mutant pairs carry by-construction truth, so D
  is known truth.
- **Not selectable:** the bars hold on agent-labelled rows or all rows, but
  neither known-truth path holds.
- **No claim:** neither the mechanical and by-construction slice nor Bar D is
  gateable. The numbers are reported, and nothing is claimed from them.

**Abstention ceiling.** A T variant that abstains on more than 10% of its
population is not selectable, whatever its bars say. Every report states each
variant's abstention count.

**Comparisons with the baseline.** The baseline is T0 on RTC rows and T0-RT on
RT rows, and it is not gated. Where T1 or T2 is compared with it, the
comparison uses only the rows both answered, and states each side's
abstention count on the shared rows. The split-by-trace section prints this
paired line whenever T0 or T0-RT is in the run.

Reported and not gated: T0 on its RTC population and T0-RT on its RT
population; each variant per mode and per truth kind; TC's per-class recall
and ECE; every trace slice; the coverage/accuracy curve.

**Selecting one variant for the held-out run.** The held-out run will cover
exactly one variant: the one listed for MP-242 in the held-out selection file
introduced by PLAT-1028's PR #620 review fixes (pending), which will hold one
entry per MP and have the runner refuse any other variant. The candidates are
T1 and T2, and only a family's final version is eligible. Qualifiers by path 1
rank ahead of qualifiers by path 2 only. Among path-1 qualifiers, the one with
the larger dev margin on the mechanical and by-construction slice is selected,
and on equal margins the one whose smaller class recall is larger wins. Among
path-2-only qualifiers, the smaller sign-test p-value wins. If still equal,
T1 is selected, since it sends fewer tokens. If neither T variant qualifies
and TC does, TC is the entry. If none qualifies, MP-242 gets no entry, nothing
from this plan runs on held-out, and the dev result is the reported result.
The held-out run happens once, under the harness's seal and log
(`QUOIN_JEV_HELDOUT=1`), and its numbers are reported against the same bars,
judged the same way.

**Versions and tuning.** PLAT-1024 allows tuning on dev, within these limits:

- Any change to a variant's wording or derive rule bumps its version, and the
  bumped version is a new instrument, judged against the same bars.
- Each variant+version will carry a pinned request digest, using the pinning
  introduced by PLAT-1028's PR #620 review fixes (pending); a wording change
  without a version bump will then fail a test. The digests for `T0-RT@v1`,
  `T1@v1`, `T2@v1`, `TC-RT@v1`, `TC-RC@v1` and `TC-RTC@v1` are pinned in phase
  2. No version of any of these has been sent to Jev, so v1 is the wording on
  this branch as merged.
- Dev iteration is capped at 5 versions per family. The families are T1, T2
  and TC, where one TC version covers all three of its modes.
- The report lists every version run on dev, with its numbers, so a selection
  is always shown with how many versions were tried to reach it.

Compare only like variant version, corpus seal and label revision. A change to
the corpus labels is a new definition version of this plan.

## Round 2: T3, assertion selection

Pre-registered here, and committed, before any T3 call.

**Why.** Dev run 1 on corpus v2 (`jev-1.13.0`): T0, T1 and T2 did not move on
weakened-test mutants. Bar D was 15 of 15 pairs tied for T0 and 22 of 22 for
T2; T1 had 1 success in 22. Jev does not notice when a test's decisive
assertion is removed or loosened. What worked elsewhere in that run was small,
concrete facts (K1, S1); worked examples (T2) added nothing.

**As pre-registered (v1).** Code lists the test's assertion statements and
Jev answers one `choice`: which listed assertion checks the behaviour the
requirement states, or `none`. `P(any) = 1 - P(none)`, and
`test_asserts_intent` is `yes` iff `P(any) >= 0.5`. The v2 rule below
replaced this one after v1's dev answers were seen: that is dev tuning inside
the version budget, and T3's result carries it.

**What T3 asks (final rule, v3).** T3 runs on RT and RTC. Code, not Jev,
reads the test body and lists its assertion statements. Rust: `assert!`,
`assert_eq!`, `assert_ne!`, `assert!(matches!(..))`, `debug_assert*!`,
`prop_assert*!`, a statement that calls `.unwrap_err()` or
`.expect_err(..)`, and a failure point: a `panic!(..)` or a proptest
`Err(TestCaseError::fail(..))`, listed with the match arm, `let .. else` or
`if` it fails in. Python: `assert` at a line's start or after a `:` on the
same line, `self.assert*(..)`, `pytest.raises(..)`, and a dotted
`assert_*(..)` call (a mock's `assert_called*`, `np.testing.assert_*`).
Comments and string literals are masked first. A statement ends at its `;`,
at a `,` outside brackets (a match arm), at the `}` closing the block it is
the tail of, or at a Python line break; a brace-delimited macro ends at its
own `}`. Whitespace is collapsed outside comments and literals only, so a
literal is listed verbatim. The list goes into the state as
`test_assertions` (`A1` .. `An`, at most 20, the overflow joined into the
last). Jev answers one strict `noul` per listed assertion: on its own, would
it fail if the system produced a different outcome from the one the
requirement states. The question names the test and the requirement only,
never the code, so the same wording runs in RT and RTC.

**`.unwrap()` and `.expect(..)` are not assertions.** They check only that a
call returned a success value, which T3's own question answers `no` for, and
nearly every Rust test calls them, so listing them would ask Jev a question
the rule already settles. A `panic!` inside a closure
(`.unwrap_or_else(|e| panic!(..))`) is the same check spelled out and is not
listed either. A test whose only check is one of these (EV2-0111:
`validator().expect(..)`) has no listed assertion, so it is `no` with
`P(any) = 0` and confidence 1, derived in code with no call.

**Derive rule (final, v2 and v3).** `P(any)` is the highest per-assertion
`P`. `test_asserts_intent` is `yes` iff `P(any) >= 0.5`; the confidence is
`P(any)` for `yes` and `1 - P(any)` for `no`; `P(any)` is the ordinal Bar D
reads. A test with no assertion is `no` with `P(any) = 0`, derived in code
with no call. A missing answer, an answer that is not a `noul` or not a
probability in [0, 1], or an answer to an assertion that was not asked stops
the run.

**Dev only.** The held-out split was spent in held-out run 1
(`fixtures/eval-v2/heldout-runs.jsonl`). T3 has no held-out run and gets no
entry in the held-out selection file; its dev result is its reported result.

**Bars.** The bars above, unchanged, over T3's own population. The headline is
**Bar D** on the known-truth pairs: `test_weakening` mutants paired with their
source by `mutation.source_id`, one pair per mutation, RTC first; a success
crosses 0.5 downward and falls by at least 0.10; an abstention on either row
is a failure; one-sided sign test at alpha 0.05, gateable at 10 or more
non-tie pairs. T0 is the comparator, on the rows both answered.

**Versions.** `REQUEST_DIGEST_PINS` holds the pin of T3's current version
only, now `T3@v3`; each bump replaced the previous version's pin. The pin
covers the question text on the canonical row, not the state, so v3 (a
wider assertion list, the same question) has v2's digest under a new label.
T3 is its own family with the 5-version dev cap. This round was
pre-registered at 3 versions; v3 is the one version past that, allowed by the
cap and run after PR #630's review. Every version run is reported with its
numbers.

### Round 2 dev results

Dev split, `jev-1.13.0`, cassette-recorded. T0 is the comparator; T3's
population is RT plus RTC, T0's is RTC only. Bar D pairs are `test_weakening`
mutants.

| Version | What changed | Bar D (succ / fail / tie, p) | Mechanical slice: agreement, constant, margin, `no` recall, `yes` recall | All rows margin |
| --- | --- | --- | --- | --- |
| T0@v1 | baseline | 0 / 0 / 15, p = 1.0 | 59.0%, 59.0%, +0.0pp, 12.5%, 91.3% (39 rows) | +17.9pp (67) |
| T3@v1 | one `choice` over the listed assertions plus `none` | 3 / 1 / 18, p = 0.3125 | 55.4%, 55.4%, +0.0pp, 24.0%, 80.6% (56 rows) | +16.7pp (108) |
| T3@v2 | one strict `noul` per assertion, highest wins; match-arm assertions no longer merge | 6 / 0 / 16, p = 0.0156 | 66.1%, 55.4%, +10.7pp, 56.0%, 74.2% (56 rows) | +17.6pp (108) |
| T3@v3 | wider assertion list (`panic!` and `TestCaseError::fail` failure points, Python mock / `np.testing` `assert_*`, `assert` after `:`); literals listed verbatim | 6 / 0 / 16, p = 0.0156 | 66.1%, 55.4%, +10.7pp, 56.0%, 74.2% (56 rows) | +17.6pp (108) |

T3@v3 is the final version. Its wider list changed some rows' probabilities
(the confidence curve and ECE moved), but every number in its row equals
v2's. Bar D is not gateable (6 non-tie pairs,
fewer than 10). On the mechanical slice (56 rows, both labels present, so
gateable) Bars A and B hold: margin +10.7pp and both class recalls above 0%.
T3@v3 abstained on no row. By MP-242's rule it qualifies on dev by path 1;
there is no held-out run to confirm it, and the v2 rule it runs was chosen
after v1's answers were seen. On the 52 RTC rows both answered, T3@v3 was
right on 39 and T0 on 34.

## Measured outcome

Not yet measured. The first dev run is PLAT-1030 phase 2, blocked on the two
dependencies above.
