---
id: NFR-030
title: "SOUP promotion and report never invent a fact or a score"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-141"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-143"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "constrains"
---

# NFR-030: SOUP promotion and report never invent a fact or a score

## Statement

The SOUP promotion, advisory join and report SHALL write or print only values that an SBOM, a reviewer-authored record, a retained scan, a stored discharge fact or a supplied public support entry stated.

## Scope

- Applies to: `quoin soup promote` and `quoin soup report`.
- Out of scope: the content of the `soup_record` schema, which `agent-ix/spec-objects-security` NFR-001 governs.

## Rationale

A defaulted support date or a guessed status reads as an assessment nobody made. A failed scan read as clean
discharges a risk nobody reviewed. Each printed value must have a named author.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Printed or written values with no stated origin | 0 | 0 | Test |
| Scores, ranks or classes emitted | 0 | 0 | Test |
| Failed or suppressed scans read as clean | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-030-AC-1 | Over a fixture with every optional input absent, every value in the promoted records and the report is `unknown`, `not stated`, absent, or copied from a named input. | Test |
| NFR-030-AC-2 | Over fixtures with a failed cargo-deny run, a zero-rule scan and a missing scan, no record is shown as free of advisories. | Test |
| NFR-030-AC-3 | The report JSON schema for a fixture run contains no score, rank, class or percentage key. | Test |

## Verification

All criteria are tests over fixtures.

## Dependencies

- **Upstream**: [FR-141](../functional/FR-141-promote-sbom-components-to-soup-records.md), [FR-142](../functional/FR-142-advisory-join-onto-soup-records.md), [FR-143](../functional/FR-143-support-status-and-end-of-support-input.md), [FR-144](../functional/FR-144-soup-report.md)
