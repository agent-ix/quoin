---
id: NFR-022
title: "Bounded, read-only measurement run"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/FR-092"
    type: "constrains"
---

# NFR-022: Bounded, read-only measurement run

## Statement

The corpus measurement SHALL complete a full run over the pinned governed corpus within 15 minutes of
wall-clock time on a developer workstation while opening every corpus file read-only.

## Scope

- Applies to: a full run over the whole pinned corpus, on one machine, from a warm filesystem cache.
- Operational context: the campaign owner re-running the census inside a working session, repeatedly,
  until every failure has an owner.

## Rationale

The campaign requires re-running until every failure is dispositioned. A census that takes an hour
gets run once, and a census run once is a census whose unknowns never get resolved.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Full-corpus wall-clock run time | 5 minutes | 15 minutes | Timed run over the pinned corpus |
| Peak resident memory | 1 GiB | 4 GiB | Resident-set sampling during a timed run |
| Corpus files opened for writing | 0 files | 0 files | Syscall-level or wrapper-level open-mode assertion |

## Verification

A timed run over the pinned corpus records wall-clock duration and peak resident memory; a run under
an open-mode assertion records the mode of every corpus and module file the measurement opens.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-022-AC-1 | A full run over the pinned corpus completes within 15 minutes of wall-clock time. | Test (TC-1560) |
| NFR-022-AC-2 | Peak resident memory during a full run stays at or below 4 GiB. | Test (TC-1561) |
| NFR-022-AC-3 | Every corpus and module file the measurement opens is opened in a read-only mode. | Test (TC-1562) |

## Dependencies

- **Upstream**: [FR-092](../functional/FR-092-stay-advisory-and-read-only.md)
