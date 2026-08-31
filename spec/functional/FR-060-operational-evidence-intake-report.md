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
- The exact retained bytes addressed by every raw-evidence path in the record
- The active authored plan that owns the record's definition version
- The obligation kind, subject, scope, accepted exercise modes, and exact
  `operational_with_clock` start/deadline condition being evaluated
- The evidence store defined by [FR-030](./FR-030-evidence-store.md) and
  [FR-044](./FR-044-plan-governed-measurements.md)

## Outputs

- One canonical standing-capability or exercise entry, or an actionable refusal
- An explicit discharge, non-discharge, or gap for a clocked obligation
- Human and canonical-JSON report views derived from the same report object

## Behavior

- If schema, cross-record, or temporal integrity validation fails, then Quoin SHALL
  refuse the record with stable reason code `invalid_record` plus every failing
  JSON path and reason.
- If a raw-evidence path is unsafe, absent, has a different byte size, or has a
  different content digest, then Quoin SHALL refuse the record with stable reason
  code `raw_evidence_mismatch`, identify the path and mismatch, and write no record.
- If the governing plan is absent, then Quoin SHALL refuse the record by naming the
  requested definition and stable reason code `governing_plan_absent`.
- If the record's definition differs from the governing plan, then Quoin SHALL
  refuse the record with stable reason code `definition_mismatch` by naming the
  expected and observed definitions.
- When validation succeeds, Quoin SHALL publish the complete entry by one atomic
  same-directory no-replace operation.
- When identical canonical bytes already exist for a record id, Quoin SHALL treat
  the intake as idempotent.
- If different canonical bytes already exist for a record id, then Quoin SHALL
  refuse the collision with stable reason code `record_id_collision` without
  replacing the retained entry.
- Quoin SHALL serialize validation and publication across the entire operational
  store so standalone and pair containers cannot concurrently retain one logical
  record id twice. If an intake lock remains after a process death, Quoin SHALL
  fail closed with `intake_busy`; an operator may remove that lock only after
  confirming no writer is active.
- When an exercise outcome is not `succeeded`, Quoin SHALL preserve the exercise as
  queryable evidence.
- When Quoin reads an exercise with clock status `missed` or `open`,
  Quoin SHALL preserve it as queryable evidence.
- When a standing capability is `available`, `quoin report` SHALL label it as
  evidence that the control exists, but SHALL NOT label it as evidence that the
  control was exercised.
- When a standing capability is `unavailable`, `unknown`, or `not_applicable`,
  `quoin report` SHALL render that state as counterevidence or a gap rather than a
  claim that the control exists.
- Quoin SHALL derive clock status from the retained timestamps and `observed_at` and
  refuse any producer-supplied status that disagrees with that derivation.
- Quoin SHALL discharge a clocked obligation only when control kind, subject, scope,
  and exercise mode match the obligation, its clock start and deadline identify the
  obligation's exact clock condition, the exercise outcome is `succeeded`, and its
  clock completion timestamp falls within that condition. Quoin SHALL derive this
  result without trusting the producer's status label.
- When a clocked exercise is failed, partial, aborted, missed, or open,
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
| FR-060-AC-1 | Valid intake writes one complete canonical record by one atomic same-directory no-replace publication. | Test (TC-1232) |
| FR-060-AC-2 | Invalid schema, cross-record, or temporal input returns `invalid_record`; unsafe, missing, wrong-sized, or digest-mismatched raw evidence returns `raw_evidence_mismatch`; both identify every mismatch and write nothing. | Test (TC-1233) |
| FR-060-AC-3 | An absent governing plan returns `governing_plan_absent`; a mismatch returns `definition_mismatch`; both write nothing and name the requested, expected, and observed definitions that apply. | Test (TC-1234) |
| FR-060-AC-4 | Repeating an identical record id and canonical payload is byte-idempotent whether the retained record is standalone or a member of an atomic pair. | Test (TC-1235) |
| FR-060-AC-5 | Reusing a record id for different semantic bytes returns `record_id_collision` without replacing the retained entry, including when another writer publishes after the initial absence check. | Test (TC-1236) |
| FR-060-AC-6 | Standing capabilities and succeeded, failed, partial, and aborted exercises remain independently queryable with their raw-evidence digests. | Test (TC-1237) |
| FR-060-AC-7 | A clocked obligation discharges only from a control-kind, subject, scope, and accepted-mode-matched exercise whose clock start/deadline identify the exact obligation clock condition, whose outcome is `succeeded`, and whose completion falls within that condition; every other case remains a named non-discharge or gap. | Test (TC-1238) |
| FR-060-AC-8 | The report renders only available capabilities and succeeded, clock-satisfying exercises as their distinct claims and evidence; unavailable, unknown, not-applicable, adverse-outcome, missed, or open states render as counterevidence or gaps beside owner and actions. | Test (TC-1239) |
| FR-060-AC-9 | Neither human nor JSON output contains an aggregate trust, confidence, or quality score derived from operational records. | Test (TC-1240) |
| FR-060-AC-10 | Re-rendering an unchanged store is byte-identical, and human and JSON views expose the same claims, evidence, counterevidence, gaps, owners, and actions. | Test (TC-1241) |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-060-CON-1 | Quoin SHALL NOT invoke, drill, or alter an operational control while recording or reporting evidence. | Architecture | Static (TC-1242) |
| FR-060-CON-2 | Existing measurement collections and pre-operational evidence SHALL remain readable without migration. | Compatibility | Test (TC-1243) |

## Dependencies

- [FR-059](./FR-059-operational-evidence-records.md) defines both accepted JSON
  shapes.
- [FR-030](./FR-030-evidence-store.md) defines evidence-store integrity and the
  no-producer-execution boundary.
- [FR-044](./FR-044-plan-governed-measurements.md) defines plan resolution,
  producer provenance, and deterministic reporting.
