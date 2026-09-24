---
id: MP-240
title: Jev severity variants S1-S3 against S0 on corpus v2
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.severity_variant_agreement
definition_version: jev.severity-variants-v1
relationships: []
---

# Jev severity variants S1-S3 against S0 on corpus v2

## Decision Use

Decide which way of asking the gap-analysis lens for `severity`, if any,
carries signal where the as-asked question (`S0`) did not (PLAT-1028, parent
PLAT-1024). MP-234 measured `S0` at 34.5% against a constant predictor's
69.0%. It answered `high` on 1 of 20 rows labelled `high`, and gave
`low`/`medium` on 21 of 29. That is collapse to the middle, not noise.
Severity also sets the order findings are reported in, so this plan grades
ordering as well as exact buckets.

This plan picks at most one variant to run once on the held-out split. It
makes no wiring decision. MP-230 governs wiring and this plan does not change
it.

## Population

The corpus v2 **dev** split (`split: dev`): the in-repo corpus
`rust/crates/quoin-jev/tests/fixtures/eval-v2/corpus.json` (PLAT-1025) plus the
external by-reference corpus (PLAT-1026) when it is present. Only rows that
carry `truth.severity` count. They are broken down by mode:

- `RTC`: requirement, test and code.
- `RT`: requirement and test, with no code.
- `RC`: requirement and code, with no test.

`R` rows (requirement alone) are outside this plan: every variant here asks
about a test or code, and none runs on `R`. The held-out split is not read
until the selection rule below has picked a variant.

## Measure Definition

Definition `jev.severity-variants-v1`. It is computed by
`rust/crates/quoin-jev/tests/eval_v2_support/metrics.rs` on top of the shared
maths in `tests/support/grading.rs`. The variants are in
`tests/eval_v2_support/variants/severity.rs`, every one at version 1:

| family | registry ids | how severity is obtained | ordinal graded for ordering |
| --- | --- | --- | --- |
| `S0` (baseline) | `S0` (RTC only) | `FullBatteryV1`'s severity `score`, rounded to the nearest level | the raw score |
| `S1` | `S1` (RTC), `S1-RT`, `S1-RC` | fact `noul`s only, each rubric tier asked as its own fact: trace correct (no -> `high`); the test would still pass with the stated behaviour broken (`high`); the test would pass against a stub (`high`); the test checks only some stated clauses (`medium`); the code contradicts the requirement (`high`); the code misses or mishandles a stated case (`medium`). Only the present artifact's facts are asked. Each fact is thresholded at the `noul` decision threshold τ = 0.5 on its own (a fact whose defect answer has probability ≥ τ is a defect), and the level is the most severe tier with any defect; `none` when no fact reaches τ. Its confidence is that level's mass under the fact probabilities treated as independent | expected level under that independence distribution |
| `S2` | `S2` (RTC), `S2-RT`, `S2-RC` | a severity `score` whose four levels are concrete situations, with invented worked examples in the instruction (three for `high`, two for each other level, per mode) | the expected score |
| `S2M` | `S2M` (RTC), `S2M-RT`, `S2M-RC` | S2's identical request. `high` when the probability mass on `high` is at least 0.25 (a uniform prior's share); otherwise S2's answer | P(`high`) |
| `S3` | `S3` (RT, RC, RTC) | a severity `score` whose levels are review actions (no action / backlog / fix before release / block merge), mapped to `none`/`low`/`medium`/`high` by position | the expected score |

Per variant, per mode slice, and per truth-kind group:

1. **Bucket agreement.** The answer is compared with the primary truth label,
   at the exact rubric level. A contested alternative counts as agreement for
   the variant and the constant predictor alike. An unanswered row stays in
   the denominator. The **margin** is agreement minus the best constant
   predictor on the same rows, in percentage points.
2. **High recall** (FAIL-class recall). Of rows whose primary label is `high`,
   the share answered `high`. Primary reading only.
3. **Ordering concordance.** Take every pair of rows whose true levels
   differ. The index is `(C + T/2) / pairs`, where C counts pairs the ordinal
   orders the same way as the truth and T counts tied pairs. 0.5 is what a
   constant or a coin scores. Goodman-Kruskal gamma and the raw C/D/T counts
   are reported beside it.
4. **Coverage curve.** At confidence thresholds 0.5, 0.6, 0.7, 0.8, 0.9 and
   0.95: the share of rows answered at or above the threshold, and the
   accuracy on them. Reported only, not gated.

5. **Paired mutant contrast** (pending `mutation.source_id`, PLAT-1025, and
   the shared `metrics::paired_contrast`, PR #625, PLAT-1029). Over
   (source row, mutant) pairs linked by `mutation.source_id`: successes,
   failures and ties under Bar D's rule below, and the one-sided sign-test
   p-value over the non-tie pairs.

ECE, no-defect recall and the per-class table are reported and not gated.

## Collection Procedure

`rust/crates/quoin-jev/tests/live_eval_v2.rs` (`live-api` feature), with
`QUOIN_JEV_VARIANTS=S0,S1,S1-RT,S1-RC,S2,S2-RT,S2-RC,S2M,S2M-RT,S2M-RC,S3` and
`QUOIN_JEV_SPLIT=dev`, through a recording cassette with the model pinned
(`QUOIN_JEV_MODEL`). The runner sends each distinct request once per run.
`S2M` reuses `S2`'s answer, so it costs nothing extra. The offline tests
`tc_1028_severity_variants.rs` run in the default gate. They check the S1 rule
branch by branch, the S2/S2M distribution reading, S3's mapping, and the
wording rule in every mode each variant claims.

A malformed `score` answer (a key that is not a level, a level spelled twice,
a negative or non-finite mass, masses not summing to 1 within 0.01, or no
distribution) aborts the run with the row, the variant and the raw response
in the error. With a recording cassette the response is already on file
before the check runs, so a paid answer is never lost.

## Environment and Sampling

The whole dev split, one pass per variant version. The live service picks the
model, and the run records which model answered. Repeat runs are
replications: they are reported and not pooled.

Phase 2 may revise variants on dev. Any change to wording or to a derive rule
bumps the variant's `version`. Every `variant@version` carries a
request-digest pin (`REQUEST_DIGEST_PINS` in
`tests/eval_v2_support/preflight.rs`): the sha256 of the questions it sends,
instructions and labels included, on a fixed canonical row per mode. The
runner's preflight (`preflight::authorize_run`, called by
`live_eval_v2.rs` before its first request) refuses a `variant@version`
whose wording does not match its pin, and a bumped version that has no pin
yet. The offline gate `tc_1027_request_digest_is_pinned_per_variant_version`
also holds the pin table equal to the registry's digests.

**Dev iteration is capped at 5 versions per family** (v1 to v5), and the cap
is hard: the preflight refuses to run any version past v5, on dev or
held-out. The report states how many versions of each family were tried on
dev. **Only a family's final version is eligible** for selection: the one
registered when the selection is committed, which is the only version the
runner can run, since the registry holds one version per id. Results of
earlier versions are reported as dev history and never selected. The bars
below do not move.

## Interpretation

**Agent-labelled truth is reported separately from mechanical and
by-construction truth, always.** The harness names the agent slice
`AGENT-LABELLED` in every table. A restatement of any number from this plan
must say which slice it comes from. A number that only holds on the
agent-labelled slice is an agreement with another agent's reading of the
rubric, not a measurement against ground truth.

Known properties of the variants, stated before any live call:

- **S1 never answers `low`.** Step 5 names conditions for `high` and `medium`
  only, and S1 transcribes step 5. Its `low` recall is 0 by construction. On
  rows labelled `low` it can only score through a contested alternative.
- **S1 has no "code exceeds the requirement" branch. It was dropped before
  any live call (PR #620 review).** v1 as first committed asked
  `FullBatteryV1`'s exceeds question and mapped `yes` to `medium`. MP-234
  measured that question as a coin flip: every probability sat between 0.50
  and 0.73. At the 0.5 threshold it would have turned clean `RC` and `RTC`
  rows `medium`, which is the collapse to the middle this plan exists to
  fix. Neither step 5 nor the corpus's `rules.severity` has an exceeds
  clause. S2's levels and examples teach no exceeds `medium` either.
- **S1 asks each rubric tier separately.** Step 5 puts "test does not
  validate intent" and "code contradicts the requirement" at `high`, and
  "partial validation, meaningful edge cases unchecked, minor drift" at
  `medium`. Each is its own fact, so the model never places the line
  between tiers.
- **S1 in `RT` and `RC`** applies only the branches whose facts were asked.
  An `RT` row's severity ignores axis (c). An `RC` row's ignores axes (a) and
  (b). This is the rubric applied to what exists, not a claim about the
  missing artifact. S3's single wording tells the model that an artifact
  missing from the row is expected and is not itself a mismatch.
- **S1's level thresholds each fact alone; its ordinal is biased toward
  `high`.** The level is the rubric over each fact thresholded at τ = 0.5.
  The ordinal, used for ordering (Bar C) and for the paired contrast
  (Bar D), is the expected level when the facts are treated as independent.
  They are not independent (a trace mismatch makes every other defect
  likely), and the expectation is biased upward: `high` is the union of up
  to four high-tier facts (the trace check and three high-tier defects in
  `RTC`), so its mass grows with the number of facts asked even when each
  is unlikely. On the probe row with trace correct at 0.9 and every defect
  fact at 0.15, every fact reads "no", yet the distribution is none 0.399,
  medium 0.153, high 0.447, and the expected level is 1.65. That is why the
  level is not the distribution's most likely level (the rule first
  registered, replaced before any live call, PR #620 re-review): it would
  have answered `high` on that row. The ordinal keeps the bias; it ranks
  rows with the same facts asked consistently, but an `RTC` ordinal sits
  higher than an `RT` or `RC` one for the same evidence, so ordinals are
  compared within a mode only, as every bar here already does.
- **S2's worked examples are invented** (coupons, withdrawals, uploads,
  schedulers). None is a corpus row or one of this repository's requirements.
  They follow the corpus labels' reading of the rubric (MP-234 `rules.severity`):
  a trace mismatch is `high`. A corpus labelled under a different reading would
  disagree with the examples by design.
- **Lyon's notebook 14 caveat applies to S2 and S3.** A score cannot agree
  with a rubric the labeller did not share, and confidence gating does not
  catch definitional disagreement. S1 is the variant built to test that: it
  moves the rubric into code, and asks the model only facts.

## Comparison and Enforcement

**Bars, set on 2026-09-23 before any live call of any variant here.** Each
family is gated per mode, on the dev split, pooled across truth kinds:

- **Bar A, margin > 0 pp.** Bucket agreement exceeds the best constant
  predictor on the same rows.
- **Bar B, high recall ≥ 50%.** Of the rows labelled `high`, at least half
  are answered `high`.
- **Bar C, concordance above S0.** On RTC, the concordance index is strictly
  greater than S0's, both computed over only the rows BOTH the variant and
  S0 answered in the same run (`metrics::paired_concordance`). Dropping each
  side's unanswered rows separately would let a variant that abstains on
  hard rows be ranked over an easier set. Each side's abstention count is
  reported. S0 has no RT or RC wording, so in RT and RC Bar C is
  concordance > 0.5, the value a constant or a coin scores. **In every mode,
  a variant that leaves more than 10% of its rows unanswered fails Bar C**,
  whatever its concordance.

- **Bar D, paired mutant contrast.** *Pending dependencies, not yet
  present on `main` when this plan is registered: the corpus field
  `mutation.source_id` (PLAT-1025) and the shared paired-contrast helper
  `metrics::paired_contrast` (PR #625, PLAT-1029). Bar D is computed by that
  helper and no other code: this plan carries no local Bar D
  implementation, and until both land Bar D is reported as "not computable
  (pending dependency)", never approximated another way. Wiring the helper
  in is phase 2, before any live call.* The rule is the one shared by every
  PLAT-1024 plan, with severity's δ:
  - **Pairing.** Pairs come only from `mutation.source_id`, which names each
    mutant's unmutated source row, the two sharing a split. One pair per
    mutation id; where a mutation id has rows in more than one mode, the
    `RTC` row is the one paired.
  - **Exclusion.** A pair whose source row is already on the defect side is
    excluded: here, every source whose primary severity truth label is at or
    above `medium` (`medium` or `high`), as `paired_contrast` excludes on the
    source's truth.
  - **Success.** The mutant's ordinal crosses τ = 2.0 (`medium`) upward, that
    is, it is at or above 2.0 while the source's is below it, AND it rose
    from the source's by at least δ = 0.5 levels.
  - **Fail.** The ordinal moved by at least δ the other way (fell by 0.5 or
    more). A pair in which the variant leaves either row unanswered also
    fails, so abstaining cannot win Bar D.
  - **Tie.** Every other pair, excluded from the test.

  D passes on a one-sided sign test at α = 0.05 over the non-tie pairs
  (successes > failures, binomial p = 0.5). It is gateable only with at
  least 10 non-tie pairs; fewer is "not gateable (n too small)". D is
  computed per mode, and reported per mutant kind as a breakdown. Its truth
  is by-construction (the mutation made the row worse), so D is known truth.

**Minimum slice.** A mode is gated only if its dev slice has at least 30
severity rows, of which at least 10 are labelled `high`. A smaller slice is
reported as "not gateable (n too small)", and no pass or fail is claimed for
it.

**Truth kinds.** Bars A, B and C are evaluated on the pooled slice. The
same three numbers are also reported on the mechanical + by-construction
slice and on the `AGENT-LABELLED` slice. A family qualifies for selection on
known truth by either path, on RTC:

1. its mechanical + by-construction slice is itself gateable (same minimum)
   AND passes A, B and C there, and A, B and C also pass pooled; or
2. Bar D is gateable on RTC and passes.

Otherwise it does not qualify. A family that
passes pooled but fails on that slice is reported as "passes on
agent-labelled truth only". One whose mechanical + by-construction slice is
not gateable, and whose Bar D is not gateable either, is reported as "not
selectable (insufficient mechanical truth)". Neither is a pass.

**Selection rule for the held-out split (decided now).** Only a family that
qualifies on known truth on RTC dev, by either path above, qualifies. RTC is the one mode where S0 exists to
beat. Of the qualifying families, the one with the highest RTC dev
concordance index goes to held-out. Ties are broken by higher RTC high recall,
then by higher RTC margin, then by the lower Bar D sign-test p-value,
then by the fixed order S1, S2, S2M, S3. The
winning family's registered versions at selection time run once on held-out,
in every mode it covers, under the harness's held-out seal and run log. The
same four bars are reported there once. The choice is committed first as
this plan's single entry in `fixtures/eval-v2/heldout-selection.json`
(`rust/crates/quoin-jev/tests/fixtures/eval-v2/heldout-selection.json`)
(variant, version, the commit it was selected at, and the dev evidence), with
`S0@v1` as its baseline. The runner's preflight
(`preflight::authorize_run`, exercised offline by
`tc_1027_the_preflight_runs_heldout_only_for_the_committed_selection`)
refuses a held-out run of any variant version that file does not cover, and
the file refuses a second entry for this plan.

If no family qualifies, nothing runs on held-out. What is reported then
depends on why. If at least one family was gateable on RTC by either
known-truth path and none passed, the PLAT-1028 result is "no
variant beats S0 on dev". If no family was gateable, it is "no claim": the
corpus was too small to judge, and that says nothing about the variants.

Compare only like variant version, corpus revision and label revision. An
edit to the corpus's severity labels is a new definition version.
