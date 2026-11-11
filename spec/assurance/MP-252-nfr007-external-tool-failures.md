---
id: MP-252
title: ix-flow failures reach the caller as a non-zero exit
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr007.external_failure_exit_rate
definition_version: nfr007.external-failure-exit-v1
ground_truth_kind: mechanical
statistical_design:
  population: every workflow command crossed with each way ix-flow can fail, namely absent from PATH, and present but exiting non-zero
  sampling: complete census of the pairs; the workflow commands are those whose `--help` begins `Start the Quoin-owned specification workflow`
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [review:ix-flow-absent, review:ix-flow-exits-nonzero, to-plan:ix-flow-absent, to-plan:ix-flow-exits-nonzero]
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-007
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# ix-flow failures reach the caller as a non-zero exit

Measures NFR-007-M-1: external-tool failures surfaced as a non-zero `quoin`
exit. The only external tool `quoin` runs is `ix-flow`; `quire` is emitted, not
run (NFR-007-M-2), and `quoin update` fetches over HTTP.

## Decision Use

Decide whether `quoin review` and `quoin to-plan` hide an `ix-flow` failure.

## Population

Each workflow command paired with each failure mode, as `command:mode`. The
commands are found by the producer from the binary's own help text; the modes
are the two the producer can stage, and both are in the set. The set is compared
with this plan's by the `tc_3004` test.

## Measure Definition

The producer gives each command a skill directory that satisfies it, and a
`PATH` holding either no `ix-flow` or a stub `ix-flow` that prints to stderr and
exits 7. The pair matches when `quoin` exits non-zero. Value: matching pairs over
pairs examined, definition `nfr007.external-failure-exit-v1`.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` stages and runs each
pair against the built binary on Unix, and records the ids.

## Interpretation

100% means neither failure mode is swallowed. A failure after `ix-flow` has
started a run (invalid JSON, a missing instance id) is a further mode not
staged here.

