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
  population: the same command x error-class pairs as MP-250
  sampling: complete census of the pairs, each class enumerated from the binary's own help text
  repetitions: 1
  estimator: proportion
  population_set:
    ids:
      - "advise --module:missing-path"
      - "advise --repo:missing-path"
      - "advise:unknown-flag"
      - "anomaly:unknown-flag"
      - "anomaly check --module:missing-path"
      - "anomaly check scope:missing-path"
      - "anomaly check:unknown-flag"
      - "anomaly refs --scope:missing-path"
      - "anomaly refs:missing-argument"
      - "anomaly refs:unknown-flag"
      - "anomaly report scope:missing-path"
      - "anomaly report:unknown-flag"
      - "anomaly:unknown-subcommand"
      - "assurance:unknown-flag"
      - "catalog list:unknown-flag"
      - "catalog methods:unknown-flag"
      - "catalog show type:unknown-type"
      - "catalog show:unknown-flag"
      - "catalog validate:duplicate-type"
      - "catalog validate:unknown-flag"
      - "catalog:unknown-flag"
      - "catalog:unknown-subcommand"
      - "change-assurance intake:missing-argument"
      - "change-assurance intake:unknown-flag"
      - "change-assurance prepare-attestation:missing-argument"
      - "change-assurance prepare-attestation:unknown-flag"
      - "change-assurance receipt:missing-argument"
      - "change-assurance receipt:unknown-flag"
      - "change-assurance record-digest:missing-argument"
      - "change-assurance record-digest:unknown-flag"
      - "change-assurance recover --repo:missing-path"
      - "change-assurance recover:unknown-flag"
      - "change-assurance schema:unknown-flag"
      - "change-assurance verify-receipt:missing-argument"
      - "change-assurance verify-receipt:unknown-flag"
      - "change-assurance write-record --input:malformed-file"
      - "change-assurance write-record --input:malformed-stdin"
      - "change-assurance write-record:missing-argument"
      - "change-assurance write-record:unknown-flag"
      - "change-assurance:unknown-flag"
      - "change-assurance:unknown-subcommand"
      - "completeness --bundle:missing-path"
      - "completeness --module:missing-path"
      - "completeness --repo:missing-path"
      - "completeness:unknown-flag"
      - "config doctor:unknown-flag"
      - "config edit:unknown-flag"
      - "config get:missing-argument"
      - "config get:unknown-flag"
      - "config set:missing-argument"
      - "config set:unknown-flag"
      - "config:unknown-flag"
      - "config:unknown-subcommand"
      - "discharge:missing-argument"
      - "discharge:unknown-flag"
      - "evidence audit --module:missing-path"
      - "evidence audit --repo:missing-path"
      - "evidence audit:unknown-flag"
      - "evidence gc --repo:missing-path"
      - "evidence gc:unknown-flag"
      - "evidence inspect-mocks:missing-argument"
      - "evidence inspect-mocks:unknown-flag"
      - "evidence record-discharge:missing-argument"
      - "evidence record-discharge:unknown-flag"
      - "evidence record-experiment --input:malformed-file"
      - "evidence record-experiment --input:malformed-stdin"
      - "evidence record-experiment --input:missing-path"
      - "evidence record-experiment:missing-argument"
      - "evidence record-experiment:unknown-flag"
      - "evidence record-operational --input:malformed-file"
      - "evidence record-operational --input:malformed-stdin"
      - "evidence record-operational --input:missing-path"
      - "evidence record-operational:missing-argument"
      - "evidence record-operational:unknown-flag"
      - "evidence record:unknown-flag"
      - "evidence trust --decision:malformed-file"
      - "evidence trust --decision:malformed-stdin"
      - "evidence trust:missing-argument"
      - "evidence trust:unknown-flag"
      - "evidence:unknown-flag"
      - "evidence:unknown-subcommand"
      - "graph change-impact:missing-argument"
      - "graph change-impact:unknown-flag"
      - "graph fan-out:missing-argument"
      - "graph fan-out:unknown-flag"
      - "graph:unknown-flag"
      - "graph:unknown-subcommand"
      - "matrix --module:missing-path"
      - "matrix --repo:missing-path"
      - "matrix:unknown-flag"
      - "measurement intervention --definition:missing-path"
      - "measurement intervention:missing-argument"
      - "measurement intervention:unknown-flag"
      - "measurement operational-release --definition:missing-path"
      - "measurement operational-release:missing-argument"
      - "measurement operational-release:unknown-flag"
      - "measurement record --input:malformed-file"
      - "measurement record --input:malformed-stdin"
      - "measurement record --input:missing-path"
      - "measurement record:missing-argument"
      - "measurement record:unknown-flag"
      - "measurement verify:unknown-flag"
      - "measurement:unknown-flag"
      - "measurement:unknown-subcommand"
      - "module ensure-defaults:unknown-flag"
      - "module install:unknown-flag"
      - "module list:unknown-flag"
      - "module remove:unknown-flag"
      - "module:unknown-flag"
      - "module:unknown-subcommand"
      - "quoin:unknown-flag"
      - "quoin:unknown-subcommand"
      - "release:unknown-flag"
      - "report --portfolio:missing-path"
      - "report --repo:missing-path"
      - "report:unknown-flag"
      - "review:unknown-flag"
      - "risk:unknown-flag"
      - "risk-summary:unknown-flag"
      - "soup promote --module:missing-path"
      - "soup promote --repo:missing-path"
      - "soup promote:missing-argument"
      - "soup promote:unknown-flag"
      - "soup report:missing-argument"
      - "soup report:unknown-flag"
      - "soup:unknown-flag"
      - "soup:unknown-subcommand"
      - "to-plan:unknown-flag"
      - "update:unknown-flag"
      - "validate --repo:missing-path"
      - "validate:unknown-flag"
      - "write --types:unknown-type"
      - "write repo_dir:missing-path"
      - "write:unknown-flag"
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

The same pairs as MP-250, compared with the plan's set by the same test.

## Measure Definition

Each pair is probed as in MP-250. It matches (is a defect) when the process
exits zero, writes nothing to stderr, or writes anything to stdout. The value is
defective pairs over pairs examined, definition `nfr003.silent-failure-v1`; the
target is none.

## Collection Procedure

As MP-250: the same probe outcome, read for the opposite property.

## Interpretation

0% means no probed failure is treated as success, left without a message, or
accompanied by output on stdout. A value above 0% lists the ids that do: the
per-class tests (TC-3700 to TC-3706) name them. Partial output after a later
failure inside a command is not probed here.
