---
id: FR-130
title: "Quire Protocol scenario-evidence intake"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "depends_on"
---

# FR-130: Quire Protocol scenario-evidence intake

## Description

`evidence.record_scenario` SHALL accept one typed Quire Protocol scenario-evidence record, validate it, and file it in the evidence store, returning a versioned result or a typed refusal.

## Rationale

Quire Protocol composes one evidence record per integration scenario and is forbidden to keep, index, score or dispose of it. Quoin owns retention, so Quoin publishes the intake the record is passed through. The intake reads a typed value only: it parses no transcript, discovers nothing ambient, and infers no owner, contract or revision from a path or a name. The record is Protocol's value; Quoin's contract is its shape, its refusals and the rule that one scenario files at most one record.

## Inputs

- `repo`: the repository root.
- `document`: the record, with exactly these members.
  - `contract`: the intake contract version the producer composed against.
  - `integration`: `IT-001`, `IT-002` or `IT-003`.
  - `scenario`: the scenario id, beginning with `<integration>-`.
  - `resolution`: one of `full` (`entries`), `partial` (`reached` and `causingEntry` with `reason`) or `legacy-compatibility` (`selection`). An entry is `owner`, `contractVersion` and `revision`.
  - `valueOwners`: per consumed producer value, `value`, `owner`, `contractVersion` and `revision`.
  - `outcome`: the `operation` outcome identity and the ordered `claims`, each a `row`, `refusal` or `resource-incomplete` with its `identity`.
  - `budget`: per dimension, `dimension`, `consumed` and an optional `granted`.
  - `traceBinding`: the acceptance `criterion`.

## Outputs

- On acceptance: the contract version `quoin.protocol-scenario-intake/v1`, the owner `quoin`, the absolute path of the filed record, whether this call created it, and the accepted record.
- On refusal: a bad-request outcome whose diagnostic carries code `QE-E010` and a `refusal` kind of `wrong-contract`, `missing-component`, `duplicate-component` or `cross-wired`.

## Behavior

- The intake SHALL read the `contract` member before any other and refuse a document naming another contract version, whatever else it holds.
- The intake SHALL file a record at `scenarios/<integration>/<scenario>.json`.
- If a record for the same integration and scenario is already filed with different content, then the intake SHALL refuse the new record and leave the first unchanged.
- The intake SHALL admit a `partial` resolution without a full manifest.
- The intake SHALL keep an absent `granted` absent rather than reading it as a grant of zero.
- The intake SHALL decide no sufficiency or score.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-130-CON-1 | The intake SHALL accept a typed document and read no transcript, file or environment. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-130-AC-1 | A complete record is filed under `scenarios/<integration>/<scenario>.json` and the same record again is accepted without a rewrite. | Test (TC-2200, TC-2208) |
| FR-130-AC-2 | A `partial` resolution and a `legacy-compatibility` selection are each admitted. | Test (TC-2201) |
| FR-130-AC-3 | A document naming another contract version, or none, is refused before its shape is read, as `wrong-contract` or `missing-component`. | Test (TC-2202, TC-2208) |
| FR-130-AC-4 | A missing member, an unknown member, a blank component and an empty budget are each refused. | Test (TC-2203) |
| FR-130-AC-5 | A value owner that the resolution state does not hold at the same owner, contract version and revision, a scenario of another integration, a scenario id that is not path-safe, and a causing entry also listed as reached are each refused as `cross-wired`. | Test (TC-2204) |
| FR-130-AC-6 | A repeated resolution entry, a value with two owners and a repeated budget dimension are each refused as `duplicate-component`. | Test (TC-2205) |
| FR-130-AC-7 | The budget is stored per dimension, a scalar budget is refused, and an absent grant stays absent. | Test (TC-2206) |
| FR-130-AC-8 | A second, different record for one integration and scenario is refused and the first is unchanged. | Test (TC-2207) |
| FR-130-AC-9 | An accepted record is answered with the intake contract version and owner, and a refusal exits with the bad-request status carrying `QE-E010` and the refusal kind. | Test (TC-2208) |

## Dependencies

- **Upstream**: [FR-048](./FR-048-append-only-assurance-evidence.md), the append-only record families.
- **Downstream**: `agent-ix/quire-protocol` Task-018 passes its scenario records through this intake.
