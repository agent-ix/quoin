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
    ids: [unterminated-flow-sequence, bad-indentation, invalid-utf8, unknown-alias, two-documents, unterminated-quoted-scalar, unterminated-flow-mapping, empty-file, null-document, bare-string, top-level-sequence, tab-indented-sequence]
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

Each kind of `manifest.yaml` that is not a single YAML mapping: an unterminated
flow sequence, a bad indent, invalid UTF-8, an unknown alias, two documents, an
unterminated quoted scalar, an unterminated flow mapping, an empty file, `~`, a
bare string, a top-level sequence and a tab-indented sequence. The producer owns
the table that builds them and the `tc_3003` test compares its ids with this
plan's set. Duplicate keys are not in the population: the reader keeps the last
value and the module is listed, so the catalog is not partial (see NFR-008).

## Measure Definition

The producer seeds an offline home, lists a valid companion module alone first
and requires it to be listed, then for each kind writes the manifest into a
candidate directory, lists it ahead of the companion in `QUOIN_MODULE_PATHS`, and
runs `quoin catalog list --json`. The kind matches when the process exits
non-zero, prints no catalog on stdout and names the candidate's `manifest.yaml`
on stderr. Every process runs under a deadline. Value: matching kinds over kinds
examined, definition `nfr008.corrupt-manifest-abort-v1`.

## Collection Procedure

`rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs` builds and runs every
candidate against the built binary.

## Interpretation

100% means each listed corruption aborts. It says nothing about corruptions the
reader accepts.

