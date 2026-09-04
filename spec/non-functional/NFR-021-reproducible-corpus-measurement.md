---
id: NFR-021
title: "Reproducible corpus measurement"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-084"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-090"
    type: "constrains"
---

# NFR-021: Reproducible corpus measurement

## Statement

The corpus measurement SHALL produce byte-identical result artifacts across repeated runs over the
same recorded corpus and module revisions.

## Scope

- Applies to: every artifact the measurement writes except its own run timestamp.
- Operational context: a clean checkout of each pinned repository, on any machine, with no network
  access during the run.

## Rationale

A census that cannot be re-run is an assertion. The promotion gate this feeds has to be able to
re-derive the numbers from the pins, and the later normalization campaign has to be able to tell a
corpus change from a measurement change.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Result artifacts differing between two runs at equal pins | 0 artifacts | 0 artifacts | Repeat run and compare SHA-256 digests |
| Ordering-dependent fields in result artifacts | 0 fields | 0 fields | Inspection of the emitted ordering contract |
| Network requests during a measurement run | 0 requests | 0 requests | Run with networking disabled |

## Verification

Two consecutive runs over the same pinned corpus write their artifacts to separate directories; every
artifact except the run manifest's timestamp field compares digest-equal. A third run executes with
networking disabled and produces the same digests.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-021-AC-1 | Two runs over the same recorded corpus and module revisions produce artifacts whose SHA-256 digests are equal, excluding the run manifest's timestamp field. | Test (TC-1557) |
| NFR-021-AC-2 | No emitted collection depends on filesystem enumeration order; every collection carries a declared ordering key. | Test (TC-1558) |
| NFR-021-AC-3 | A run executed with networking disabled produces the same artifact digests as a run with networking available. | Test (TC-1559) |

## Dependencies

- **Upstream**: [FR-084](../functional/FR-084-pin-and-enumerate-the-governed-corpus.md), [FR-085](../functional/FR-085-resolve-the-completed-module-set.md)
