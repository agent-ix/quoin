---
id: MP-253
title: Corrupt manifests abort catalog assembly
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr008.corrupt_manifest_abort_rate
definition_version: nfr008.corrupt-manifest-abort-v1
ground_truth_kind: mechanical
statistical_design:
  population: every kind of present-but-unparseable `manifest.yaml` the producer builds
  sampling: complete census of the corrupt-manifest kinds in the producer's table
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [unterminated-flow-sequence, bad-indentation, invalid-utf8, unknown-alias, two-documents, unterminated-quoted-scalar, unterminated-flow-mapping]
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-008
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# Corrupt manifests abort catalog assembly

Measures NFR-008-M-1: corrupt manifests that abort assembly rather than drop
silently.

## Decision Use

Decide whether a broken installed module is surfaced instead of hidden behind a
partial catalog.

## Population

Each kind of manifest that is not a single valid YAML document: an unterminated
flow sequence, a bad indent, invalid UTF-8, an unknown alias, two documents, an
unterminated quoted scalar and an unterminated flow mapping. The producer owns
the table that builds them and the `tc_3005` test compares its ids with this
plan's set. Duplicate keys, tab-indented sequences and an empty file are
accepted by the YAML reader and are not in this population.

## Measure Definition

For each kind the producer writes the manifest into a candidate directory, lists
it ahead of a valid module in `QUOIN_MODULE_PATHS`, and runs
`quoin catalog list --json`. The kind matches when the process exits non-zero and
prints no catalog on stdout. Value: matching kinds over kinds examined,
definition `nfr008.corrupt-manifest-abort-v1`.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` builds and runs every
candidate against the built binary.

## Interpretation

100% means each listed corruption aborts. It says nothing about corruptions the
reader accepts.

