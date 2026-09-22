---
id: MP-233
title: Gap-analysis battery agreement against targeted-mutation and static-check truth
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.battery_truth_agreement
definition_version: jev.battery-truth-v1
relationships: []
---

# Gap-analysis battery agreement against targeted-mutation and static-check truth

## Decision Use

Decide whether the gap-analysis lens's answers to the six questions MP-232 did
NOT grade are grounded in checkable facts, rather than in wording that merely
looks thorough. MP-232 graded `assertion_vacuous` alone. A GO on one question
of seven is a GO on one question of seven; this plan states what the ground
truth is for the rest, and where there is none.

## Population

Four populations, each with its own denominator. A row is never defaulted into
a class it was not measured in.

**P1, `test_asserts_intent`.** One row per *violating* targeted mutant: a
source substitution that contradicts the specific behaviour the triple's
requirement states, leaving the rest of the symbol intact. The lens is shown
the UNMUTATED triple. A triple with no violating mutant is out of the
population.

**P2, `code_implements_intent`.** Two rows per violating mutant whose owning
test FAILED under it: the shipped body, and the mutated body. A violating
mutant the owning test did not catch contributes no negative row — without a
test failure there is no mechanical proof the mutated body violates the
requirement — and is listed as excluded, exactly as MP-232 excludes a triple
whose return type admits no safe stub.

**P3, `code_exceeds_requirement`.** Two rows per *additive* mutant whose owning
test still PASSED: the shipped body, and the mutated body. An additive mutant
the test caught changed asserted behaviour, so it is not purely additive and is
excluded.

**P4, `tests_only_its_own_mock`.** The 29 corpus triples plus 8 rows
constructed for this evaluation, labelled `CONSTRUCTED` in their own
`test_file` and `note`, in the convention `GAP-28`/`GAP-29` already use.

## Measure Definition

Agreement between the lens's answer, thresholded at 0.5, and the mechanical
outcome, definition `jev.battery-truth-v1`:

- **P1** truth: the owning test failed under the violating mutant, so the test
  asserts the behaviour the requirement states and not merely its existence.
- **P2** truth: the shipped body implements the requirement (yes); the
  confirmed-violating body does not (no).
- **P3** truth: the shipped body implements nothing beyond its requirement
  (no); the additive body does (yes). **The negative class is weaker than the
  positive one.** That a shipped, reviewed, `Trace:`-tagged body exceeds
  nothing is an assumption about this corpus; the positive class is code
  written to add behaviour the requirement text does not contain, with the
  owning test rerun to confirm it constrains none of it.
- **P4** truth: a static check over the test's own source text — every
  assertion's expected operand carries no `::`-qualified path and no literal
  absent from the test's setup. No model is consulted.

`divergence_kind` is measured two ways over P2 and P3's rows with a
mechanically derived label (`aligned`, `test_weaker_than_requirement`,
`code_short_of_requirement`, `code_exceeds_requirement`): asked directly as a
six-way choice, and derived from the same response's own `noul` answers by a
rule fixed before the first live call. Two of the six labels —
`test_stronger_than_requirement` and `requirement_ambiguous` — have no row in
this population and are not claimed to have been tested.

`severity` is measured as **discrimination, not accuracy**: over the P2 pairs,
the share where the confirmed-defective mutant is scored strictly more severe
than the shipped body it was made from. A constant answer wins no pair, so the
null is 50%.

## Collection Procedure

Apply each mutation as an exact source substitution in a scratch, uncommitted
edit of the worktree; rerun exactly the one owning test; record pass or fail;
revert before any commit — nothing mutated is ever committed. Retain the file,
the replaced text, the replacement, the test name and the outcome per mutation,
in `rust/crates/quoin-jev/tests/fixtures/gap-battery-mutants.json`. Run the
lens's `live-api` battery through `quoin_jev::client::production` over one
request per distinct row. Dimension every observation by `lens` and `variant`.

## Environment and Sampling

The targeted-mutation campaign is a sampled subset of the 29-triple corpus, not
a full mutation-testing run, and the sample is stated by row rather than
summarised. The static check runs over all 29 test bodies.

## Interpretation

P1, P2 and P4 are mechanical: checked against the compiler, the test runner, or
the test's own source text, not a labeller. P3's positive class is mechanical
and its negative class is an assumption, stated. `severity` has no ground truth
at all and the number reported for it is a discrimination rate, which is not
comparable with an agreement figure and must not be quoted as one.

The static check's finding that ZERO of the 29 real tests assert only against
their own configuration is itself a result about this corpus: it means a
constant `no` predictor is already perfect there, and that the balanced P4
number depends on constructed rows. Both are reported; neither substitutes for
the other.

## Comparison and Enforcement

This plan assigns no verdict. MP-230 states gap-analysis's bar on the
`assertion_vacuous` metric; the bars for these six are pre-registered in
`rust/crates/quoin-jev/tests/live_gap_battery.rs`'s module doc and committed
before the first live call that measures them. Compare only like lens, variant,
corpus revision and mutation log.
