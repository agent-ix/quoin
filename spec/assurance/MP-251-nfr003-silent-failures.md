---
id: MP-251
title: Commands that fail on a bad argument never fail silently
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr003.silent_failure_rate
definition_version: nfr003.silent-failure-v1
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
    comparator: le
    threshold: 0.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-003
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# Commands that fail on a bad argument never fail silently

Measures NFR-003-M-2: silent or partial-output failures.

## Decision Use

Decide whether any command answers an invalid invocation with success, with no
message, or with output on stdout that a caller could mistake for a result.

## Population

The same commands as MP-250, compared with the plan's set by the same test.

## Measure Definition

For each command the producer runs `quoin <command> --no-such-flag-for-nfr003`.
The command matches (is a defect) when the process exits zero, writes nothing to
stderr, or writes anything to stdout. The value is defective commands over
commands examined, definition `nfr003.silent-failure-v1`; the target is none.

## Collection Procedure

As MP-250: the same probe outcome, read for the opposite property.

## Interpretation

0% means no command treats an unknown flag as success. Partial output after a
later failure inside a command is not probed here.

