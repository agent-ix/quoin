---
id: MP-243
title: Jev criterion-soundness checklist variants K1-K2 on corpus v2
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.criterion_soundness_checklist
definition_version: jev.criterion-soundness-checklist-v2
ground_truth_kind: agent-labelled
protected_apparatus:
  - rust/crates/quoin-jev/tests/eval_v2_support/variants/soundness.rs
  - rust/crates/quoin-jev/tests/eval_v2_support/variant.rs
  - rust/crates/quoin-jev/tests/eval_v2_support/metrics.rs
  - rust/crates/quoin-jev/tests/eval_v2_support/corpus.rs
negative_controls:
  - kind: apparatus-edit
    description: every wording or derive change bumps the variant's version, so two reports never share a label for different instruments
  - kind: suppressed-observation
    description: a transport failure, a missing, mis-typed or out-of-range answer to an asked check, or a broken mutant/source link aborts the run and is never scored, so no row can silently become an abstention or leave bar D's denominator
relationships: []
---

# Jev criterion-soundness checklist variants K1-K2 on corpus v2

## Decision Use

Decide whether named yes/no defect checks can do what the holistic
criterion-strength question could not: tell a sound acceptance criterion from
an unsound one better than a constant answer. If a variant is selectable on
dev (see Comparison and Enforcement), one variant is run once on held-out.
This plan does not decide wiring into any skill. That needs its own ticket.

The holistic lens failed in PLAT-917 (MP-228). As shipped (`v0`) it cleared 0
of the 5 criteria labelled sound (46.7%, -33.3 pp). No later wording beat the
constant predictor. The last retry, `v5`, scored -6.7 to 0 pp. The EARS lens
passed (MP-229, `v6`: +18.6 pp). It asks named, narrow pattern questions and
derives the label in code. The TypeSafe material points the same way. Every's
writing checks replace "is it good" with named pass/fail checks. TypeSafe's
autoresearch cookbook reports that many narrow questions feeding a model beat
one holistic score. TypeSafe's limits page says Jev reads literally, so
boundary cases belong in the criteria text.

## Population

The `dev` split of corpus v2 (PLAT-1025, plus any PLAT-1026 external rows),
mode `R` rows only, at the corpus revision whose held-out seal is committed.
Two kinds of row:

- **Natural rows.** Criteria as they ship, with two independent agent
  labels. Where the two disagree, the label is contested and records the
  alternative.
- **Mutant rows.** A natural criterion with one defect injected by a text
  mutation. On each, the injected key is `yes` and `criterion_sound` is `no`
  by construction. `mutation.source_id` (pending, from PLAT-1025) names the
  natural row it came from.
  The corpus builder makes each mutant to meet the injected check's
  definition (see The shared labelling rules) and records how in the
  mutation's description. A `missing_trigger` mutant leaves a dangling
  reference (`then`, `it`, `the event`) with no stated trigger.

**Resolution.** In-repo corpus v2 holds about 20 dev mode `R` natural rows,
so one row is about 5 pp there. External rows are pooled with them. The
report prints the pooled natural count and the pp per row on every bars
block. A margin of one row is within that resolution and is read as such.

## Measure Definition

Definition `jev.criterion-soundness-checklist-v2`. The variants are in
`rust/crates/quoin-jev/tests/eval_v2_support/variants/soundness.rs`, which
holds every question's full text:

- **K1, the checklist.** One `noul` per named defect. Each asks whether the
  defect is present. The yes and no criteria state the exact condition and
  the boundary cases.
- **K2.** K1 plus four synthetic worked examples per check, two per outcome.
  The examples are appended to each instruction, and the criteria are K1's.
  They were written for this module. None is taken from a corpus row.

Both run in mode `R`. The state is the criterion (the AC, or the
requirement's statement when there is no AC) plus the requirement's
statement as context. This is the context C0 sends, and the corpus labelling
brief uses the same.

An earlier draft had a K3, which added EARS keyword-confusion checks. It is
withdrawn and is not registered. On this corpus its checks could only add
false positives against the labels, and the EARS lens already covers keyword
confusion.

### The shared labelling rules

The five defect definitions are the `defect` and `clear` texts of each
entry in `CHECKS`, in
`rust/crates/quoin-jev/tests/eval_v2_support/variants/soundness.rs`. This
plan does not restate them; the code is the text.

Corpus v2's labels come from the shared definitions module,
`rust/crates/quoin-jev/tests/eval_v2_support/criterion_defects.rs`, which
PLAT-1025 adds. It holds each check's defect and clear text word for word as
in `CHECKS`, and PLAT-1025 and PLAT-1026 label to it. Once that module is on
`main`, `CHECKS` reads its text from there, so the questions and the labels
share one source.

### Derivation, with τ = 0.5

A defect key is `yes` when its probability is 0.5 or more.
`criterion_sound` is `no` when any defect key is `yes`, and its confidence is
then the highest defect probability. Otherwise it is `yes`, and its
confidence is 1 minus the highest. τ is fixed here and is not tuned on dev.

**A missing or mis-keyed answer fails loudly.** Every asked check must come
back as a `noul` holding a finite probability in `[0, 1]`. A missing key, a
key answered as another question type, or a value outside that range aborts
the run, naming the row and the key. It is never turned into an abstention,
so a K variant cannot abstain on a row it ran on.

### Credit

A truth's readings are its primary label and every alternative,
deduplicated. An answer among `k` readings earns `1/k`, and any other answer,
or none, earns 0. An uncontested row is worth 1 to the right answer. A
contested `yes`/`no` row is worth 0.5 to either answer. The constant
predictor is credited by the same rule, and the best of `yes` and `no` is
used. A margin is the variant's credit minus the constant's. An exact tie,
including one made of half credits, is not a win.

The harness report (`metrics.rs`) still prints its full-credit agreement.
That number is reported and not gated.

## Collection Procedure

```bash
cd rust && QUOIN_JEV_VARIANTS=C0,K1,K2 QUOIN_JEV_SPLIT=dev \
  cargo test -p quoin-jev --features live-api --test live_eval_v2 -- --nocapture
```

After the harness report, `live_eval_v2` prints an "MP-243 bars" block for
each K variant, computed by `soundness::render_bars`. Cost is one request
per row for each of K1, K2 and C0.

`tc_1031_criterion_soundness.rs` runs in the default gate. It checks:

- the derivation, and the loud failures;
- the request shapes and the wording rule;
- credit, and bars A-D, including every broken-pair panic;
- the jointly-answered C0 comparison;
- K1 and K2 end to end over a fake Jev.

No live call is made before this plan is committed.

**Pending dependencies.** None of these is on `main` when this plan is
committed. Each is a dependency, not a present fact:

- **`mutation.source_id` values in the corpus (PLAT-1025).** This PR adds
  the optional schema field and the pairing code that reads it. Corpus v2's
  mutant rows get their `source_id` from PLAT-1025.
- **The shared defect definitions module (PLAT-1025).**
  `eval_v2_support/criterion_defects.rs` holds the five definitions word for
  word as `CHECKS` states them, and the corpus is labelled to it. Until it
  merges, `CHECKS` is the only copy of the text.
- **The shared paired helper (#625).** Bar D's pairing is implemented in
  `soundness::pairs` for now. When #625 lands, this module moves to the
  shared helper, and the rules stated under bar D below stay the same: one
  pair per mutation id, and a success crosses τ, with the source below 0.5
  and the mutant at or above, and a rise of at least δ.
- **The held-out selection file and digest pinning (#620).** These are
  introduced by PLAT-1028's #620 review fixes:
  - `rust/crates/quoin-jev/tests/fixtures/eval-v2/heldout-selection.json`,
    with one entry per MP, and a runner that refuses any held-out variant
    not listed there;
  - pinned per-version request digests, so a wording change without a
    version bump fails a test.

**Phase 2 of PLAT-1031 is blocked until these merge.** Phase 2 then pins
K1@v1 and K2@v1.

## Environment and Sampling

The whole `dev` split's mode `R` rows. There is no subsample. One pass per
variant. The service chooses the model and the report records it. A cassette
(`QUOIN_JEV_CASSETTE`) may pin the answers so that re-grading costs nothing.
Repeat passes are reported as replications and are not pooled.

## Interpretation

**Agent-labelled numbers are not human ground truth.** Two agent passes wrote
the natural rows' labels. Bars A-C read only those rows. A mutant row never
contributes to A-C, not even through a label it inherited from its source.
Bar D reads mutant/source pairs, whose difference is true by construction.
The frontmatter says `agent-labelled` because A-C are gated on
agent-labelled rows.

A pass shows that the checklist separates sound from unsound criteria on this
corpus better than a constant does. It shows nothing about defect kinds the
checklist does not name, such as implementation coupling or a happy-path-only
criterion set.

## Comparison and Enforcement

**Bars, set before the first live call of any K variant.** A bar that is
not gateable makes no claim. That is not a fail.

On the natural rows (agent-labelled):

- **(A)** The `criterion_sound` margin is strictly above 0. This is gateable
  when the natural rows labelled `criterion_sound` hold both classes by
  primary label.
- **(B)** The no-defect recall for `criterion_sound` is above 0%. At least
  one natural row labelled sound is answered `yes`. This is gateable when
  such a row exists. It catches the flag-everything failure.
- **(C)** A check qualifies when its natural rows hold both classes by
  primary label. C passes when a strict majority of the qualifying checks
  each beat their own constant predictor. With fewer than 2 qualifying
  checks, C is not gateable, and that alone is not a fail.

On mutant/source pairs (by-construction):

- **(D) Paired detection.** A pair is a mode `R` dev mutant with one
  injected checklist key, and the row named by its `mutation.source_id`.
  K variants run in mode `R` only, so a mutant in any other mode is not
  paired. There is one pair per mutation id; when a mutation has more than
  one mode `R` row, the first in corpus order is used.
  - **Pair link.** The harness panics when a checklist mutant has no
    `source_id`, or when its source is missing from the run's rows, is
    itself a mutant, is not mode `R`, or is in another split. It also panics
    when a mutant's truth marks more than one check as injected.
  - **Exclusion.** A pair whose source is already labelled `yes` on the
    injected key is excluded.
  - **Mutant must meet the definition.** A pair counts only if its mutant
    meets the injected check's definition. The corpus's by-construction
    truth on the injected key is that record: the corpus builder writes it
    only for a mutant made to meet the definition, and the harness pairs
    only a mutant with exactly one by-construction `yes` checklist key. The
    mutation description is provenance for a reader and is not parsed.
  - **The probability compared.** For K1 and K2, the injected check's
    probability is that check's `noul` probability, kept as the
    prediction's ordinal.
  - **Outcome.** A pair is a *success* when the answer crosses τ: the
    source's probability is below 0.5, the mutant's is at or above 0.5, and
    it rose by at least δ = 0.10 from the source. It is a
    *failure* when it fell by at least δ. A pair where the variant gave no
    probability on either row is also a *failure*. Anything else is a
    *tie*, and ties are excluded.
  - **Pass.** D passes on a one-sided sign test at α = 0.05 over the
    non-tie pairs. D is gateable only with at least 10 non-tie pairs.

**Selectable.** A variant is selectable only when all of these hold:

- D is gateable and passes.
- A and B pass.
- C passes or is not gateable.

A variant that passes only on the natural rows is **not selectable**. If D
is not gateable, no K variant is selectable, and phase 2 reports "no claim"
for this question shape.

**Reported and not gated:**

- The harness report's full-credit agreement.
- Per-check defect and no-defect recall.
- Per-class recall and precision.
- ECE.
- The coverage/accuracy curve.
- Every number, split by slice.

**Comparisons against C0.** C0 is the criterion-strength lens as shipped.
A K-versus-C0 comparison uses only the natural rows both variants answered.
The bars block states each variant's abstention count: labelled rows it gave
no answer for. The abstention ceiling is 5% of the labelled natural rows.
Above it, that comparison is marked not interpretable. C0 is not gated.

**Dev iteration.** Each variant id (K1, K2) is capped at 5 versions on dev.
Every version tried is reported with its numbers. Only the **final** version
of each id is eligible for selection. A new id in the K family needs an
amendment to this plan, committed before its first live call.

**Selecting one variant for held-out.** This is fixed now:

1. Only the final version of an id, and only if it is selectable, is
   eligible.
2. Among eligible variants, pick the one with the highest `criterion_sound`
   margin on the natural rows.
3. On an exact credit tie, pick the one with more D successes.
4. If still tied, pick K1, the cheaper wording.
5. If no variant is eligible, nothing runs on held-out. Phase 2 reports "no
   claim" if D was not gateable, and NO-GO otherwise.

The selected variant and version are written as MP-243's one entry in
`heldout-selection.json` and committed before the held-out run. The run uses
`QUOIN_JEV_SPLIT=heldout QUOIN_JEV_HELDOUT=1`, and it is logged before any
number prints. The same bars and selectability rule apply to the held-out
result. That result is reported once, whatever it is.

**A possible K4, not registered here.** The autoresearch cookbook supports
feeding the five checks' probabilities into a fitted model instead of a fixed
τ. That needs no new live calls, because it reads K1's answers. It is fitted,
so it would have to be cross-validated within dev, and the current harness
has no step for that. It is not eligible for held-out under this plan. If it
is wanted, it gets its own definition version and bars, committed before it
is fitted.

Compare only like variant version, corpus revision and label revision. A
change to any question text, example or derive rule bumps the variant's
version, and that is a new instrument.
