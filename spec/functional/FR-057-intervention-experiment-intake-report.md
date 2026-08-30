---
id: FR-057
title: "Intervention-experiment intake and claim-centered reporting"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-015"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-056"
    type: "requires"
---

# FR-057: Intervention-experiment intake and claim-centered reporting

## Description

When a producer submits an intervention-experiment record, Quoin SHALL validate
the record and its governing definition before changing the evidence store.

When an intervention-experiment record is accepted, Quoin SHALL retain its exact
semantic content and raw-evidence links as one atomic store entry.

When a report includes intervention-experiment evidence, `quoin report` SHALL
render the record under claims, evidence, counterevidence, gaps, owner, and action.

## Inputs

- A record conforming to
  [FR-056](./FR-056-intervention-experiment-record.md)
- The active authored plan that owns the record's definition version
- The existing evidence store defined by
  [FR-030](./FR-030-evidence-store.md) and
  [FR-044](./FR-044-plan-governed-measurements.md)

## Outputs

- One canonical intervention-experiment store entry, or an actionable refusal
- Human and canonical-JSON report views derived from the same report object

## Behavior

- If schema validation fails, then Quoin SHALL refuse the record with every failing
  JSON path and reason.
- If the governing plan is absent, then Quoin SHALL refuse the record by naming
  the requested definition.
- If the record's definition differs from the governing plan, then Quoin SHALL
  refuse the record by naming the expected and observed definitions.
- When validation succeeds, Quoin SHALL commit the complete entry by one atomic
  same-directory rename.
- When identical canonical bytes already exist for a record id, Quoin SHALL treat
  the intake as idempotent.
- If different canonical bytes already exist for a record id, then Quoin SHALL
  refuse the collision without replacing the retained entry.
- While a record is failed, inconclusive, or `cause_not_established`, Quoin SHALL
  preserve it as queryable evidence.
- When rendering a causal claim, `quoin report` SHALL place measured effects and
  raw references under evidence.
- When rendering a causal claim, `quoin report` SHALL place uncontrolled or unknown
  interactions and confounders under counterevidence.
- When rendering a causal claim, `quoin report` SHALL place the record's declared
  gaps under gaps.
- `quoin report` SHALL NOT compute or display an overall trust, confidence, or
  quality score from intervention-experiment records.
- Human and canonical JSON output SHALL derive from the same deterministically
  ordered report object.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | Valid intake writes one complete canonical record by one atomic same-directory rename. | Test (TC-1204) |
| FR-057-AC-2 | Invalid schema input writes nothing and reports every failing JSON path and reason. | Test (TC-1205) |
| FR-057-AC-3 | An absent or definition-mismatched governing plan writes nothing and names the requested, expected, and observed definitions that apply. | Test (TC-1206) |
| FR-057-AC-4 | Repeating an identical record id and canonical payload is byte-idempotent. | Test (TC-1207) |
| FR-057-AC-5 | Reusing a record id for different semantic bytes is refused without replacing the retained entry. | Test (TC-1208) |
| FR-057-AC-6 | Completed, failed, inconclusive, and `cause_not_established` records remain independently queryable with their raw-evidence digests. | Test (TC-1209) |
| FR-057-AC-7 | The report renders conclusion statements as claims and measured effects plus raw references as evidence. | Test (TC-1210) |
| FR-057-AC-8 | Uncontrolled or unknown interactions and confounders render as counterevidence; declared gaps render separately beside owner and actions. | Test (TC-1211) |
| FR-057-AC-9 | Neither human nor JSON output contains an aggregate trust, confidence, or quality score derived from experiment records. | Inspection (TC-1212) |
| FR-057-AC-10 | Re-rendering an unchanged store is byte-identical, and human and JSON views expose the same claims, evidence, counterevidence, gaps, owners, and actions. | Test (TC-1213) |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-057-CON-1 | Quoin SHALL NOT execute an experiment or producer while recording or reporting evidence. | Architecture | Static (TC-1214) |
| FR-057-CON-2 | Quoin SHALL NOT infer causality from an effect, confidence label, missing confounder, or empty gap list. | Integrity | Test (TC-1212) |
| FR-057-CON-3 | Existing measurement collections and pre-experiment evidence SHALL remain readable without migration. | Compatibility | Test (TC-1215) |

## Dependencies

- [FR-056](./FR-056-intervention-experiment-record.md) defines the accepted JSON
  shape.
- [FR-030](./FR-030-evidence-store.md) defines evidence-store integrity and
  no-producer-execution boundaries.
- [FR-044](./FR-044-plan-governed-measurements.md) defines plan resolution,
  producer provenance, and deterministic reporting.
