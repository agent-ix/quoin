---
id: FR-091
title: "Account for known tool defects as cited, distinct states"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-089"
    type: "depends_on"
---

# FR-091: Account for known tool defects as cited, distinct states

## Description

The corpus measurement SHALL record each known tool defect that distorts it as a declared exclusion
citing the defect's repository and issue number, and SHALL report the documents that exclusion covers
in a state that is neither `pass` nor `fail`.

## Rationale

A document that cannot be read because the reader is broken is not a document that failed a contract.
Four such defects are known to be live while this measurement runs; folding them into either bucket
would produce a corpus rate that is wrong in a direction nobody could later recover. Naming them as
declared exclusions also tells the promotion gate exactly how much of the corpus the measurement
could not speak for.

## Inputs

- A declared tool-defect ledger: each entry carrying an identifier, the owning repository, the issue
  number, the effect it has on the measurement, and the scope it covers.

## Outputs

- A tool-defect record per entry: the identifier, the citation, the affected document or row count,
  and the check the entry suppressed.
- A coverage statement giving the share of the population the declared exclusions cover.

## Behavior

- The measurement SHALL refuse a tool-defect ledger entry that carries no repository and issue
  number.
- Where a document falls inside a declared tool-defect scope, the measurement SHALL report that
  document `could-not-run` citing the entry's identifier.
- The measurement SHALL classify every failure covered by a declared tool-defect entry as
  `tool-defect` in the partition of FR-089.
- The measurement SHALL report the affected count per tool-defect entry as a count of documents and,
  where the entry's effect is on rows, additionally as a count of rows.
- The measurement SHALL publish the tool-defect coverage statement beside the corpus aggregate rate.
- The measurement SHALL NOT treat an undeclared failure as a tool defect.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-091-CON-1 | A tool-defect entry SHALL cite a repository and issue number that a reader can open. | Interface | Inspection |
| FR-091-CON-2 | The tool-defect ledger SHALL NOT suppress a finding outside the scope its entry declares. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | A ledger entry without a repository and issue number is refused with the entry named. | Test (TC-1545) |
| FR-091-AC-2 | A document inside a declared scope reports `could-not-run` citing the entry identifier, and is absent from the pass numerator and denominator. | Test (TC-1546) |
| FR-091-AC-3 | A failure covered by a declared entry is classified `tool-defect` in the partition and carries the citation. | Test (TC-1547) |
| FR-091-AC-4 | An entry declaring a row-level effect reports both an affected document count and an affected row count. | Test (TC-1548) |
| FR-091-AC-5 | A document outside every declared scope that fails is not classified `tool-defect`. | Test (TC-1549) |
| FR-091-AC-6 | The published report states the share of the population covered by declared tool-defect exclusions beside the aggregate rate. | Test (TC-1550) |

## Dependencies

- **Upstream**: [FR-089](./FR-089-partition-every-failure.md)
