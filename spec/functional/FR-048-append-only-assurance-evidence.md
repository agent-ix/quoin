---
id: FR-048
title: "Append-only experiment and operational evidence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "references"
---

# FR-048: Append-only experiment and operational evidence

## Description

Quoin SHALL store experiment records and operational evidence records as
content-addressed, append-only JSON artifacts. Each record SHALL carry its own
complete `producer-provenance-v1` tuple: producer identity and version, source
revision and clean/dirty state, executable and configuration digests,
capabilities, and source-artifact digests.

The record identifier SHALL be the SHA-256 digest of canonical record content
before the identifier is attached. Publication SHALL be atomic within the
destination directory. Re-publishing identical bytes is idempotent; different
bytes at an existing immutable path are reported and never overwritten.

This requirement does not retrofit FR-030 run records. The accepted boundary
binds the existing FR-044 producer tuple to measurement collections and the new
producer tuple to these two new record kinds. Run-record evolution requires a
separate versioned decision and migration plan.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | Every experiment and operational record carries the complete closed producer-provenance contract. | Test (TC-1137) |
| FR-048-AC-2 | Canonical content determines the record id; identical publication is idempotent and byte-stable. | Test (TC-1138) |
| FR-048-AC-3 | Invalid or incomplete provenance is rejected before any record is published. | Test (TC-1139) |
| FR-048-AC-4 | A different file at an immutable record path is never overwritten and produces a named integrity error. | Test (TC-1140) |
| FR-048-AC-5 | Operational evidence records bind a subject, environment, ordered time window, observations, interpretations, raw evidence, and outcome. | Test (TC-1141) |
| FR-048-AC-6 | Reads recompute content identity, detect tampering, and do not read or write FR-030 run-record paths. | Test (TC-1142) |
| FR-048-AC-7 | `quoin evidence record-experiment` and `record-operational` accept JSON files or stdin and report the immutable id, path, and whether publication created a file. | Test (TC-1143) |

## Constraints

- The writer reads no wall clock; all timestamps are producer input.
- Observation values are preserved as producer strings with their declared
  unit and interpretation; this layer does not manufacture a quality score.
- Raw evidence is referenced by immutable identifier rather than copied into
  authored prose.

## Dependencies

- FR-030 supplies the evidence-store root and canonical serialization rules.
- FR-044 remains the measurement-collection provenance boundary.
