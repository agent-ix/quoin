---
id: FR-060
title: "Operational evidence intake, clocked discharge, and reporting"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-016"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-059"
    type: "requires"
---

# FR-060: Operational evidence intake, clocked discharge, and reporting

## Description

When a producer submits operational evidence, Quoin SHALL validate the record and
its governing definition before changing the evidence store.

When operational evidence is accepted, Quoin SHALL retain the standing capability
or exercise as one atomic store entry without converting one shape into the other.

When an obligation declares `operational_with_clock`, Quoin SHALL discharge it only
from a scope-matched exercise whose retained clock state demonstrates completion
within the declared deadline.

When a report includes operational evidence, `quoin report` SHALL render the record
under claims, evidence, counterevidence, gaps, owner, and action.

## Inputs

- A record conforming to
  [FR-059](./FR-059-operational-evidence-records.md)
- The active authored plan that owns the record's definition version
- The obligation kind, subject, scope, and clock condition being evaluated
- The evidence store defined by [FR-030](./FR-030-evidence-store.md) and
  [FR-044](./FR-044-plan-governed-measurements.md)

## Outputs

- One canonical standing-capability or exercise entry, or an actionable refusal
- An explicit discharge, non-discharge, or gap for a clocked obligation
- Human and canonical-JSON report views derived from the same report object

## Behavior

- If schema validation fails, then Quoin SHALL refuse the record with every failing
  JSON path and reason.
- If the governing plan is absent, then Quoin SHALL refuse the record by naming the
  requested definition.
- If the record's definition differs from the governing plan, then Quoin SHALL
  refuse the record by naming the expected and observed definitions.
- When validation succeeds, Quoin SHALL commit the complete entry by one atomic
  same-directory rename.
- When identical canonical bytes already exist for a record id, Quoin SHALL treat
  the intake as idempotent.
- If different canonical bytes already exist for a record id, then Quoin SHALL
  refuse the collision without replacing the retained entry.
- When an exercise outcome is not `succeeded`, Quoin SHALL preserve the exercise as
  queryable evidence.
- When Quoin reads an exercise with clock status `missed`, `open`, or `unknown`,
  Quoin SHALL preserve it as queryable evidence.
- `quoin report` SHALL label a standing capability as evidence that a control exists.
- `quoin report` SHALL NOT label a standing capability as evidence that a control
  was exercised.
- When a clocked exercise is failed, partial, aborted, missed, open, or unknown,
  `quoin report` SHALL render the outcome as counterevidence or a gap.
- When rendering operational evidence, `quoin report` SHALL preserve the record's
  declared gaps, owner, and actions as separate sections.
- `quoin report` SHALL NOT compute or display an overall trust, confidence, or
  quality score from operational evidence.
- Human and canonical JSON output SHALL derive from the same deterministically
  ordered report object.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | Valid intake writes one complete canonical record by one atomic same-directory rename. | Test (TC-1231) |
| FR-060-AC-2 | Invalid schema input writes nothing and reports every failing JSON path and reason. | Test (TC-1232) |
| FR-060-AC-3 | An absent or definition-mismatched governing plan writes nothing and names the requested, expected, and observed definitions that apply. | Test (TC-1233) |
| FR-060-AC-4 | Repeating an identical record id and canonical payload is byte-idempotent. | Test (TC-1234) |
| FR-060-AC-5 | Reusing a record id for different semantic bytes is refused without replacing the retained entry. | Test (TC-1235) |
| FR-060-AC-6 | Standing capabilities and succeeded, failed, partial, and aborted exercises remain independently queryable with their raw-evidence digests. | Test (TC-1236) |
| FR-060-AC-7 | A clocked obligation discharges only from a control-kind, subject, and scope-matched exercise whose clock status is `met`; every other clock status remains a named non-discharge or gap. | Test (TC-1237) |
| FR-060-AC-8 | The report renders capabilities and successful exercises as their distinct claims and evidence; failed, partial, aborted, missed, open, or unknown exercises render as counterevidence or gaps beside owner and actions. | Test (TC-1238) |
| FR-060-AC-9 | Neither human nor JSON output contains an aggregate trust, confidence, or quality score derived from operational records. | Inspection (TC-1239) |
| FR-060-AC-10 | Re-rendering an unchanged store is byte-identical, and human and JSON views expose the same claims, evidence, counterevidence, gaps, owners, and actions. | Test (TC-1240) |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-060-CON-1 | Quoin SHALL NOT invoke, drill, or alter an operational control while recording or reporting evidence. | Architecture | Static (TC-1241) |
| FR-060-CON-2 | Existing measurement collections and pre-operational evidence SHALL remain readable without migration. | Compatibility | Test (TC-1242) |

## Dependencies

- [FR-059](./FR-059-operational-evidence-records.md) defines both accepted JSON
  shapes.
- [FR-030](./FR-030-evidence-store.md) defines evidence-store integrity and the
  no-producer-execution boundary.
- [FR-044](./FR-044-plan-governed-measurements.md) defines plan resolution,
  producer provenance, and deterministic reporting.
