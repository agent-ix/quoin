---
id: MP-241
title: code_exceeds_requirement variants E1-E4 against the as-asked question on corpus v2
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.code_exceeds_variant_agreement
definition_version: jev.code-exceeds-variants-v1
ground_truth_kind: agent-labelled
relationships: []
---

# code_exceeds_requirement variants E1-E4 against the as-asked question on corpus v2

## Decision Use

Decide whether asking about `code_exceeds_requirement` one code unit at a
time makes the answer carry signal. Asked about a whole code body (E0), it
tied the constant predictor in PLAT-1014 (MP-234: 64.3% vs. 64.3%), with
every probability between 0.50 and 0.73. This plan pre-registers PLAT-1029's
variants and their bars before any of them is asked a live question. It
picks at most one variant for the held-out split. It assigns no GO/NO-GO for
wiring a variant into the gap-analysis lens: MP-230 governs that and is not
changed here.

## Population

Corpus v2 (PLAT-1025 in-repo, PLAT-1026 external by reference), **dev split
only** for everything but the one held-out run in Comparison and Enforcement.
The population is every dev row that:

- is in mode `RC` (requirement plus code) or `RTC` (the full triple), and
- carries truth for `code_exceeds_requirement`.

No row is excluded by hand. A row the loader excludes (digest mismatch, patch
that does not apply) is listed with its reason at the top of the report and is
not in the population. The corpus had not landed when this plan was written,
so its size and class balance are unknown here. **A truth group with fewer than 10 rows of a class a bar needs is not
gateable for that bar** (Comparison and Enforcement): at fewer than 10, one row
moves a class recall by more than 10 pp.

Truth comes in two groups, reported apart in every table:

- **Known truth**: `mechanical` and `by-construction` rows. For this key these
  are mostly `additive_code` mutations, which are `yes` by construction.
- **AGENT-LABELLED**: `agent_dual` and `agent_contested` rows (two independent
  agent labels; a disagreement stays contested with both readings recorded).

## Measure Definition

Definition `jev.code-exceeds-variants-v1`. Grading reuses
`rust/crates/quoin-jev/tests/eval_v2_support/metrics.rs`, which calls the
shared maths in `tests/support/grading.rs`:

- **Agreement**: primary plus contested matches. The shared report counts
  unanswered rows as misses. The bars use the diagnostics' answered-rows
  table, with abstentions counted beside it (Comparison and Enforcement).
- **Constant predictor**: the best single label the population admits; the
  margin is agreement minus it, in percentage points.
- **Yes-class recall**: of rows whose readings are all `yes`, the share
  answered `yes`. An unanswered row found nothing.
- **No-class recall**: of rows whose primary reading is `no`, the share
  answered `no`.
- **Coverage curve**: at confidence floors 0.5-0.95 (the shared report) and,
  for E3, at target coverages of 100/90/80/70/60/50%.

The variants, all in
`rust/crates/quoin-jev/tests/eval_v2_support/variants/exceeds.rs`:

| id | modes | asks | row answer |
| --- | --- | --- | --- |
| `E0@v1` | RTC | `FullBatteryV1`'s whole-body `noul`, as asked | `yes` at `p >= 0.5` |
| `E0-RC@v1` | RC | the same `code_exceeds_requirement` question alone | `yes` at `p >= 0.5` |
| `E1@v1` | RC, RTC | per non-trivial unit: a 4-level `task_relation` score and a `unit_kind` choice cross-check | `yes` when a unit puts a strict majority of both answers' mass on "not needed" |
| `E2@v1` | RC, RTC | per non-trivial unit: which requirement clause it serves, or `none` | `yes` when a unit's `P(none) >= 0.5` |
| `E4@v1` | RC, RTC | E1's request, re-read (no extra call) | `yes` on `task_relation` alone |
| E3 | as its base | no request of its own | its base, abstaining below a confidence floor |

Thresholds, fixed now and not tuned:

1. "Most of its mass" is a strict majority, `> 0.5`. `task_relation` levels 0
   ("no visible connection") and 1 ("same area, not needed") are "not needed";
   `unit_kind` labels `adds_behaviour` and `unrelated` are.
2. E2's tau is 0.5, inclusive, as PLAT-1029 states it. The tau sweep
   (0.3-0.9) is reported on dev and gates nothing.
3. **Parking.** A unit whose `task_relation` and `unit_kind` disagree on "not
   needed" is parked and counted. It cannot make the row `yes`. A `no` given
   while any unit is parked carries confidence at most 0.5.
4. **Circuit breaker.** When a strict majority of a row's non-trivial units
   put a strict majority of their mass on level 0, the requirement is not about
   this code. The row is reported as trace-suspect and left **unanswered**:
   an abstention, counted against the ceiling, and never scored as
   exceedance.
5. **Trivial units** are dropped before any call: an empty or comment-only
   unit, or a `match` arm / `if`/`else` block whose body is a pass-through
   (`_ => Ok(())`, `None => None`). A whole function is never trivial.
6. **Clauses** (E2) come from the statement then the AC. They are split into
   list items, then into sentences at `;` and at a sentence-ending `.`, `!` or
   `?`. A piece under three words joins the clause before it. A repeated
   clause is dropped. The count is capped at 12, and the overflow joins the
   last clause. `and` never splits.
7. **E3's floor** for a target coverage is the confidence of the row at rank
   `ceil(target x rows)`, highest first. Ties at the floor are kept.

Every threshold is a constant in the module, and changing one is a new
variant version.

**An unreadable answer stops the run.** A missing answer, an empty
distribution, a key outside the question's levels or labels, or a total
outside 1 +/- 0.05 panics and names the row and unit. It never becomes zero
mass or a quiet abstention.

## Collection Procedure

`rust/crates/quoin-jev/tests/live_eval_v2.rs` (`live-api` feature) with
`QUOIN_JEV_VARIANTS=E0,E0-RC,E1,E2,E4`, recorded to a cassette (`QUOIN_JEV_CASSETTE`,
`QUOIN_JEV_MODEL` pinned). It prints the shared report, then
`exceeds::render_diagnostics`: E1's and E4's unit counts (trivial, asked,
parked, exceeding) and breaker rows with their truth, E2's tau sweep, and E3's
gated curve over E0, E1, E2 and E4, with bar C's line per mode. The offline
test `tc_1029_exceeds_variants.rs` runs in the default gate. It checks the
clause splitter, the pre-filter, every roll-up rule above, the sweep and the
gated curve against hand-computed answers, and runs the variants end to end
over a fake Jev.

## Environment and Sampling

The whole dev population, one pass. The service chooses the model, and the run
records it. A cassette pins it for re-grading. Replications are reported and
not pooled.

## Interpretation

**Agent-labelled numbers are labelled as such wherever they appear.** A pooled
number that includes agent-labelled rows says how many. The known-truth group
for this key is expected to be almost all `yes` rows, so the constant
predictor scores 100% on it, and margin and no-class recall cannot be read
there. What it measures is **detection**: the yes-class recall on injected
exceedances.

The circuit breaker trades answers for honesty. On a row whose requirement is
about other code, MP-234's labelling rule (and corpus v2's "judged against the
requirement SHOWN only") makes `code_exceeds_requirement` `yes`. E1 and E4
leave those rows unanswered rather than claim exceedance, and each breaker
row costs agreement. The diagnostics list the breaker rows' truth so that cost
is visible. PLAT-1030's trace check is the question those rows need. When a
variant answering `trace_correct` ran on the same rows, the diagnostics
cross-tabulate breaker rows against its answers. That is reported, not gated.

**E0-RC**, the as-asked baseline on RC rows. E0 sends `FullBatteryV1`'s seven
questions, six of which concern the test or other keys, so it cannot run
without a test. E0-RC sends only the `code_exceeds_requirement` question,
taken from the battery and byte-identical ("Does the code implement behaviour
no requirement states?"). The one-line diff: seven questions become one.

E1 requires two answers to agree before it says `yes`, and William Lyon's
notebook 11 found that such a second gate costs recall. E4 reads the same
answers without it, at no extra cost. **Prediction, stated before any run:**
E4's yes-class recall is at least E1's, and E1's no-class recall is at least
E4's. Both follow from the rule, since a unit that exceeds under E1 also exceeds
under E4. The question is the size of the gap.

jev-code (the source of E1's shape) reports no accuracy figure, so nothing
here rests on its results.

## Comparison and Enforcement

**Bars, pre-registered here before any E1, E2 or E4 live call.**

**Where bars are read.** Each bar is read on each of two truth groups:

- **Known truth**: mechanical plus by-construction rows.
- **AGENT-LABELLED**: agent-labelled rows.

Each group is pooled over RC and RTC and repeated per mode. A group is
**gateable** for a bar when it holds at least 10 rows of each class the bar
needs.

**Answered rows and abstention.** Bars A and B are read on the rows the
variant answered. The constant predictor is computed on those same rows. A
comparison between two variants, including the as-asked baseline (E0 on RTC
and E0-RC on RC), uses only the rows both answered. Every table states each variant's abstention count: breaker rows,
rows with no clause, and rows with nothing asked.

- **Abstention ceiling: 20%.** A variant that leaves more than 20% of a
  group's rows unanswered is not selectable on that group, whatever its bars.
- For E3, abstention is the mechanism, so coverage is defined explicitly:
  coverage = rows kept / every row in the population. The denominator
  includes the base variant's own abstentions.

The bars:

1. **Bar A, margin > 0 pp:** on its answered rows, agreement exceeds the
   constant predictor on the same rows.
2. **Bar B, both class recalls > 0%:** yes-class recall and no-class recall are
   both above zero on the answered rows.
3. **Bar C, E3:** for E3 over a base variant, accuracy at >= 60% coverage (as
   defined above) is above the as-asked baseline's accuracy on the rows of
   that mode it answered: E0 on RTC, E0-RC on RC. Bar C is read for E3 over
   E0 and E0-RC, and over the variant selected below.
4. **Bar D, paired contrast (the rule PLAT-1028..1031 share).**
   - **Pairs.** Each `additive_code` mutation on dev is paired with its
     unmutated source row, named by `mutation.source_id`. Pairs come from
     that field and nothing else. There is one pair per mutation id, taken
     only from rows in the modes the variant runs on: among those, its RTC
     row is used, else the row in the richest mode (RC before RT). A row in a
     mode the variant does not run on is never paired. The run stops if a
     mutant names no source, or if its source is missing, is not a natural
     row, sits in another split, or is in a mode the variant does not run on,
     or if an answer's P is not a finite number.
   - **Exclusions.** A pair is left out, and counted, when its source is
     already labelled `yes` on `code_exceeds_requirement`, since nothing was
     injected there.
   - **Verdicts.** Let P be the variant's probability for the key: E0's and
     E0-RC's P(yes), E1's weaker "not needed" mass, E2's highest P(none), and
     E4's highest "not needed" mass. A pair **succeeds** when it crosses
     **tau = 0.5** upward (source P < 0.5, mutant P >= 0.5) and P rose by >=
     **delta = 0.10**. It **fails** when P fell by >= delta, or when either
     in-scope row went unanswered: an abstention never helps a variant, and abstentions are
     counted apart. Every other pair is a **tie** and is excluded.
   - **Test.** Bar D passes on a **one-sided sign test at alpha = 0.05** over
     the non-tie pairs: `P(X >= successes)` for `X ~ Binomial(n, 1/2)` is at
     most 0.05. It is gateable at **10 or more** non-tie pairs.
   - **Why a sign test.** The #620 review found that a strict majority of
     pairs passes by noise 38% of the time. At n = 10, 8 successes (p = 0.055) fails and 9 (p = 0.011)
     passes.
   - **Why this delta and tau.** Delta keeps sampling noise out of the count,
     and crossing tau requires the answer to actually flip to `yes`.
   - **Pending dependency.** `mutation.source_id` is pending in PLAT-1025's
     corpus schema. The shared helper (`metrics::paired_contrast`) is written
     against that field name and tested on synthetic rows. The exceeds
     module's `source_id` reader is a stub until the field lands, and a run
     that holds an `additive_code` mutant stops at bar D until then. Bar D is
     not measured before the field exists.
   - Bar D needs no `no` label, because the source row is the control.

**What a result means.**

- A variant **qualifies** by either of two paths, each under the ceiling:
  - **Bar D**: it is gateable (at least 10 non-tie pairs), and the variant
    passes it.
    An `additive_code` mutant's truth is by construction, so bar D is a
    known-truth measure.
  - **Bars A and B**: the known-truth group is gateable for them, and the
    variant passes both there.
- A pass on the AGENT-LABELLED group alone is reported as **not selectable**.
- A group that is not gateable yields **no claim**, not a failure.
- The known-truth group for this key is expected to be almost all `yes` rows
  (additive mutations). If that holds, A and B are not gateable there and bar D
  is the qualifying path. If bar D also has fewer than 10 non-tie pairs, the
  result is "no claim".

**Iteration budget.** At most 5 versions per variant family (E1, E2, E4) may
be run on dev. Every version tried is reported with its numbers, including
those not selected. Retuning E2's tau or any other threshold after seeing dev
creates a new version, and it counts against the budget.

**Request identity.** Each variant version's request shape is pinned by a
request digest in `REQUEST_DIGEST_PINS` (`tests/eval_v2_support/preflight.rs`,
from PLAT-1028's #620); E0, E0-RC, E1, E2 and E4 are pinned at v1. The live
runner's preflight refuses a variant whose wording no longer matches its pin,
and a version with no pin. The pins cover wording only: a change to a derive
rule or a threshold (E2's tau, E3's floor) changes no request, so its version
bump is enforced by review.

**Selection for the held-out run (one variant, one run).**

1. Candidates are E1, E2 and E4, each at its **final** dev version (the last
   one run within the iteration budget), if that version qualifies. An
   earlier version is never eligible, even if it scored better.
2. Pick the smallest bar D p-value. On a tie, pick the largest
   known-truth margin on answered rows. If still tied, pick the larger of
   min(yes-class recall, no-class recall), then the lower id. A variant that
   qualifies only through A/B ranks by margin, after every variant that
   qualifies through bar D.
3. Write the choice into `fixtures/eval-v2/heldout-selection.json`, one entry
   for MP-241. The runner's preflight (#620) refuses a held-out run for any
   variant not listed there, and a second held-out run of the same selection.
4. The held-out run is that variant plus E0 and E0-RC, once, under
   `QUOIN_JEV_HELDOUT=1`.
5. E3 over the chosen variant uses the floor that reached 60% coverage on
   dev. That floor is frozen as a number in this document before the run.

If no candidate qualifies, there is no held-out run, and the result is
reported as "no claim" or negative, whichever applies.

Compare only like variant version, corpus revision and label revision. An
edit to the corpus truth is a new definition version.

## Round 2: E5, outcome necessity per unit

Pre-registered here, and committed, before any E5 call.

**Why.** Dev run 1 on corpus v2 (`jev-1.13.0`): on the known additive mutants
(by construction, all `yes`), E1 and E4 reached 19.2% recall and E2 34.6%,
below E0's 40.0%. The per-unit task relation and clause choice did not help.
What worked elsewhere in that run was small, concrete facts (K1, S1).

**What E5 asks.** E5 runs on RC and RTC. Code splits the code body into units
with `units.rs`, exactly as E1 does. For each unit E5 does not skip, Jev
answers one `noul`: if this part were deleted, would the code fail to do
something the requirement states? The state carries the whole body in
`symbol_body` and the unit in `code_unit_text`, so the deletion is judged
against the code around it. An additive unit is unnecessary and should answer
`no`.

**Trivial units, skipped in code with no call.** E1's pre-filter (empty, or a
pass-through branch), plus three structural rules on a branch unit's masked
condition and body statements: every statement is a log or print call
(logging); apart from logging, one statement that passes on an error it was
given, such as `Err(e)`, `return Err(e.into())`, `raise`, `raise X from e`
(error plumbing); an upper-bound condition against a literal or constant whose
one statement refuses, such as `if x.len() > MAX { return Err(..) }` (a size
cap). A refusal that builds a new error without such a bound is asked about.

**Derive rule.** `code_exceeds_requirement` is `yes` iff some asked unit has
`P(necessary) < 0.5`. The ordinal is the highest `1 - P(necessary)` (0 when
nothing is asked); the confidence is the ordinal for `yes` and one minus it
for `no`. **Breaker**, as E1's: with two or more units asked and every one
unnecessary, no part of the code does anything the requirement states, so the
row abstains as trace-suspect and counts against the abstention ceiling. A
missing or out-of-range answer stops the run.

**Dev only.** The held-out split was spent in held-out run 1
(`fixtures/eval-v2/heldout-runs.jsonl`). E5 has no held-out run and gets no
entry in the held-out selection file; its dev result is its reported result.

**Bars.** The bars above, unchanged. The headline is **Bar D** on the
known-truth pairs: `additive_code` mutants paired with their source, a
success crosses 0.5 upward and rises by at least 0.10, an abstention is a
failure, one-sided sign test at alpha 0.05, gateable at 10 or more non-tie
pairs. E0 (RTC) and E0-RC (RC) are the comparators.

**Versions.** `E5@v1` is pinned in `REQUEST_DIGEST_PINS`. E5 is its own family
with the 5-version dev cap; this round runs at most 3 versions, and every
version run is reported with its numbers.

### Round 2 dev results

Dev split, `jev-1.13.0`, cassette-recorded. Bar D pairs are `additive_code`
mutants. Agreement and margin are on the rows the variant answered.

| Version | What changed | Bar D (succ / fail / tie, p) | Known truth: `yes` recall | All rows: agreement, constant, margin | Abstained |
| --- | --- | --- | --- | --- | --- |
| E0@v1 (RTC) | baseline | 4 / 0 / 10, p = 0.0625 | 40.0% (15) | 59.6%, 57.4%, +2.1pp (47) | 0 |
| E0-RC@v1 (RC) | baseline | 2 / 0 / 9, p = 0.25 | 81.8% (11) | 72.7%, 68.2%, +4.5pp (22) | 0 |
| E5@v1 | `units.rs` units, breaker | 4 / 0 / 20, p = 0.0625 | 38.5% (26) | 56.1%, 59.1%, -3.0pp (66) | 3 of 69 |
| E5@v2 | statement-level units, statements that only log skipped | 9 / 5 / 10 (4 failures are abstentions), p = 0.2120 | 68.2% (22 answered of 26) | 66.1%, 57.6%, +8.5pp (59) | 10 of 69 |
| E5@v3 | breaker dropped | 9 / 1 / 14, p = 0.0107 | 73.1% (26) | 68.1%, 60.9%, +7.2pp (69) | 0 |

E5@v3 is the final version, and Bar D holds for it: 10 non-tie pairs, the
minimum, with 9 successes. **Caveat:** v3 changes only the derive rule. Its
requests are v2's, so its Bar D was read off answers that were already seen,
and the breaker was dropped because v2's abstentions were 4 of its 5 Bar D
failures. That is dev tuning inside the budget. The held-out split is spent,
so there is no independent confirmation. Treat the pass as provisional until a
fresh split exists.

## Measured outcome

Not yet measured. Phase 1 (PLAT-1029) commits the variants, their offline
tests and this plan. No E1, E2 or E4 answer existed when it was written, and
corpus v2 had not landed.
