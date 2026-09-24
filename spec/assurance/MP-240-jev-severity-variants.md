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
| `S1` | `S1` (RTC), `S1-RT`, `S1-RC` | fact `noul`s only (trace correct; test asserts intent; test passes against a stub; test checks every clause; code implements intent; code handles every stated case; code exceeds requirement), with only the present artifact's facts asked. Severity is computed in code by a transcription of the step-5 rubric, one branch per rubric clause | expected level under the fact probabilities, treated as independent |
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

## Environment and Sampling

The whole dev split, one pass per variant version. The live service picks the
model, and the run records which model answered. Repeat runs are
replications: they are reported and not pooled.

Phase 2 may revise variants on dev. Any change to wording or to a derive rule
bumps the variant's `version`. The report states how many versions of each
family were tried on dev. The bars below do not move.

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
- **S1's `code exceeds requirement -> medium` branch comes from step 4 B**
  ("each unstated-but-implemented constraint → `medium` finding"), not from
  step 5. Step 5 takes "the code it governs" from step 4, and step 4 A's
  visibility split (high / medium / low) is not asked.
- **S1 in `RT` and `RC`** applies only the branches whose facts were asked.
  An `RT` row's severity ignores axis (c). An `RC` row's ignores axes (a) and
  (b). This is the rubric applied to what exists, not a claim about the
  missing artifact.
- **S1's ordinal assumes the facts are independent.** They are not: a trace
  mismatch makes every other defect likely. The ordinal is for ranking, not a
  calibrated probability.
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
  greater than S0's on the same RTC rows in the same run. S0 has no RT or RC
  wording, so in RT and RC Bar C is concordance > 0.5, the value a constant
  or a coin scores.

**Minimum slice.** A mode is gated only if its dev slice has at least 30
severity rows, of which at least 10 are labelled `high`. A smaller slice is
reported as "not gateable (n too small)", and no pass or fail is claimed for
it.

**Truth kinds.** The bars are evaluated on the pooled slice. The same three
numbers are also reported on the mechanical + by-construction slice and on
the `AGENT-LABELLED` slice. Suppose a family passes pooled but fails on a
gateable mechanical + by-construction slice (same minimum). That pass is
reported as "passes on agent-labelled truth only", and it does not qualify
for selection.

**Selection rule for the held-out split (decided now).** Only a family that
passes A, B and C on RTC dev qualifies. RTC is the one mode where S0 exists to
beat. Of the qualifying families, the one with the highest RTC dev
concordance index goes to held-out. Ties are broken by higher RTC high recall,
then by higher RTC margin, then by the fixed order S1, S2, S2M, S3. The
winning family's registered versions at selection time run once on held-out,
in every mode it covers, under the harness's held-out seal and run log. The
same three bars are reported there once. If no family qualifies, nothing runs
on held-out and the PLAT-1028 result is "no variant beats S0 on dev".

Compare only like variant version, corpus revision and label revision. An
edit to the corpus's severity labels is a new definition version.
