---
id: FR-191
title: "The dossier trace matrix renders the evidence-backed matrix per requirement"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-165"
    type: "references"
---

# FR-191: The dossier trace matrix renders the evidence-backed matrix per requirement

## Description

The trace matrix (`--doc trace-matrix`, document id `DOS-TRACE`) SHALL render
the `quoin matrix` `MatrixOutput` (FR-115, with the design column of FR-165)
per requirement: criterion, statement, design cell, method, static status,
evidence status and criticality.

## Inputs

- The output of `quoin matrix --json` over the scope.

## Outputs

- `dossier/DOS-TRACE.md`, then `Gaps`.

## Behavior

- Rows SHALL equal the criteria of `quoin matrix --json`, one to one, grouped by requirement id, then ordered by criterion id.
- A criterion with no test tag SHALL produce an `unbacked_criterion` gap.
- A criterion whose `evidence_status` is not `bound` SHALL produce a `no_evidence` gap.
- An empty population SHALL render the matrix's `reason` text and be `incomplete`.
- Static-only `quire matrix` (quire-cli FR-043) is not the source.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-191-AC-1 | The rows equal the `quoin matrix --json` criteria for the fixture, one to one | Test |
| FR-191-AC-2 | A criterion with evidence status `undischarged` appears with a `no_evidence` gap | Test |
| FR-191-AC-3 | An empty population renders the `reason` text and status `incomplete` | Test |
| FR-191-AC-4 | A criterion with no test tag appears with an `unbacked_criterion` gap | Test |
| FR-191-AC-5 | Each row shows design cell, method, static status, evidence status and criticality as the matrix reports them | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md), [FR-115](./FR-115-evidence-backed-test-matrix.md), [FR-165](./FR-165-matrix-design-column.md).
- **Downstream**: none.
