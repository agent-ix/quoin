---
id: MP-250
title: Commands that fail on a bad argument say why and exit non-zero
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr003.actionable_failure_rate
definition_version: nfr003.actionable-failure-v1
ground_truth_kind: mechanical
statistical_design:
  population: every command listed in the COMMANDS block of `quoin --help`
  sampling: complete census of the top-level commands, enumerated from the binary itself
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [advise, assurance, catalog, change-assurance, completeness, config, discharge, evidence, graph, matrix, measurement, module, report, review, to-plan, update, validate, write]
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-003
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# Commands that fail on a bad argument say why and exit non-zero

Measures NFR-003-M-1: failure paths that emit an actionable message or a
non-zero exit.

## Decision Use

Decide whether argument validation, one of the failure paths NFR-003 names,
surfaces a failure to the caller rather than hiding it.

## Population

Every command in the COMMANDS block of `quoin --help`. The list is read from the
binary by the producer and compared with `statistical_design.population_set` by
the `tc_3003` test, so a command added or removed without updating this plan
fails that test. Nested subcommands, unknown types and catalog conflicts are not
in this population.

## Measure Definition

For each command the producer runs `quoin <command> --no-such-flag-for-nfr003`.
The command matches when the process exits non-zero and writes a message to
stderr. The value is matching commands over commands examined, definition
`nfr003.actionable-failure-v1`. The message is checked to be present, not to be
correct.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` runs the probes against
the built `quoin` binary and records `population.examined_ids` and
`population.matched_ids`. Set `NFR_COLLECTION_OUT` to a directory to keep the
collection it admits.

## Interpretation

100% means every top-level command refuses an unknown flag out loud. It does not
show that a message names the fix.

