---
id: FR-092
title: "Stay advisory and read-only over the corpus"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
---

# FR-092: Stay advisory and read-only over the corpus

## Description

The corpus measurement SHALL complete with a success exit status whatever it finds, writing no file
outside its declared output directory.

## Rationale

The campaign's merge gate says no corpus repository is edited by this ticket and new constraints
remain advisory throughout it. Those are properties of the tool, not promises in a document: a
measurement that can fail a build is a measurement somebody will be tempted to make quieter, and a
measurement that can write is a measurement that can destroy the evidence it was run to collect.

## Inputs

- A declared output directory.

## Outputs

- The measurement artifacts, written only under the declared output directory.
- A process exit status.

## Behavior

- The measurement SHALL exit with status `0` when it completes, whatever the measured rates and
  failure counts are.
- The measurement SHALL open every corpus file and every module file for reading only.
- The measurement SHALL write its artifacts only under the declared output directory.
- If the declared output directory is inside an enumerated corpus repository other than the one the
  measurement runs from, then the measurement SHALL refuse to start naming that repository.
- The measurement SHALL record, in its own manifest, a digest of every artifact it wrote.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-092-CON-1 | No corpus repository's working tree SHALL differ before and after a measurement run. | Safety | Test |
| FR-092-CON-2 | The measurement SHALL exit non-zero only when it fails to run, never for what it measured. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-092-AC-1 | A run over a corpus in which every document fails exits with status `0`. | Test (TC-1551) |
| FR-092-AC-2 | The Git status of every enumerated corpus repository is byte-identical before and after a run. | Test (TC-1552) |
| FR-092-AC-3 | Every file written by a run is under the declared output directory. | Test (TC-1553) |
| FR-092-AC-4 | A declared output directory inside another enumerated corpus repository is refused before any file is read. | Test (TC-1554) |
| FR-092-AC-5 | The run manifest carries a SHA-256 digest for every artifact written. | Test (TC-1555) |
| FR-092-AC-6 | A measurement that cannot resolve its declared module set exits non-zero, distinguishing a tool failure from a measured failure. | Test (TC-1556) |

## Dependencies

- **Upstream**: [FR-084](./FR-084-pin-and-enumerate-the-governed-corpus.md)
