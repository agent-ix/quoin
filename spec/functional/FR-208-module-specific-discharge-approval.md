---
id: FR-208
title: "Apply module-specific approval requirements to clause discharge"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-029"
    type: "requires"
---

# FR-208: Apply module-specific approval requirements to clause discharge

## Description

Quoin SHALL apply approval requirements declared by the installed clause set
when it discharges facts against a validated binding report. The clause set's
`approvalRoles` metadata is authoritative for whether a direct fact requires
an approval reference; FR-046 continues to allow a direct fact without one
when no module-specific requirement applies.

## Inputs

- A validated `clause-binding-v1` report.
- Discharge facts.
- The installed modules that declare the exact clause-set identity in the
  binding report.

## Outputs

- The FR-046 discharge report with approval-required clauses classified as
  direct evidence only when the required approval reference is present.
- Open work with a reason when a required approval reference is missing.

## Behavior

- Quoin SHALL resolve the binding report's exact clause-set identity against
  the installed modules that declare clause sets.
- If more than one installed module declares the exact clause-set identity,
  then Quoin SHALL reject the discharge rather than select a module by load
  order.
- When the binding report resolves to one clause-set owner, Quoin SHALL
  validate that clause set against vocabulary declared by all installed
  modules.
- Quoin SHALL treat a module that contributes vocabulary used by the clause
  set as a vocabulary provider only, not as the owner of the clause set or its
  approval policy.
- For each binding clause, Quoin SHALL read its `approvalRoles` metadata from
  the uniquely resolved clause set.
- When a binding clause declares one or more `approvalRoles`, Quoin SHALL
  classify its direct fact as open work when the fact has no `approvalRef` or
  its reference is empty or whitespace-only.
- When the declared approval requirement calls for a named human approver, the
  `approvalRef` on a fact SHALL resolve to a signed approval verified under
  FR-211, and the verified approver is that person.
- When a binding clause declares no `approvalRoles`, Quoin SHALL retain the
  FR-046 behavior for a direct fact without `approvalRef`.
- When a direct fact satisfies a declared approval requirement, Quoin SHALL
  retain it as direct evidence and preserve its `approvalRef`.
- For an approval-class clause Quoin SHALL verify the approval reference
  (FR-211); for a clause with empty `approvalRoles` Quoin SHALL treat it as a
  caller-supplied reference without claiming that its target, approver
  identity, role, or authorization has been authenticated.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-208-CON-1 | A clause set's approval requirement applies only to facts for the exact clause-set identity and clause ID declared by that set. | Functional | Test |
| FR-208-CON-2 | When the Quoin runtime processes approval metadata, it SHALL NOT calculate a risk score, apply a risk threshold, or make an automated risk-acceptability decision. | Security | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-208-AC-1 | A binding clause with non-empty `approvalRoles` remains open when its direct fact has no `approvalRef`, or has an empty or whitespace-only reference. | Test |
| FR-208-AC-2 | A binding clause with non-empty `approvalRoles` that require a named human approver is direct only when its direct fact has a non-blank `approvalRef` that FR-211 verifies as that person's approval; the report retains the reference without changing the direct classification. | Test |
| FR-208-AC-3 | A binding clause with empty `approvalRoles` continues to accept a direct fact without `approvalRef` as specified by FR-046. | Test |
| FR-208-AC-4 | When multiple installed modules declare the same exact clause-set identity, discharge fails without selecting either module's approval policy. | Test |
| FR-208-AC-5 | For an approval-class clause the report shows the approver verified under FR-211; for a clause with empty `approvalRoles` it preserves the supplied reference and does not claim to authenticate the approver, role, or authorization it represents. | Test |
| FR-208-AC-6 | When the uniquely resolved clause set uses an edge type declared by another installed module, `record-discharge` validates the clause set using vocabulary from all installed modules. | Test |
| FR-208-AC-7 | When a module supplies vocabulary for a uniquely resolved clause set, that provider's `approvalRoles` do not affect discharge classification; only the clause-set declaring module's `approvalRoles` do. | Test |
| FR-208-AC-8 | When the uniquely resolved clause set uses an edge type declared by no installed module, discharge fails with a diagnostic identifying the unknown edge type. | Test |

## Dependencies

- **Upstream**: [FR-211](./FR-211-discharge-resolves-evidence-and-approvals.md) defines approval verification and evidence resolution; [FR-046](./FR-046-clause-discharge.md) defines the discharge populations and generic approval-reference behavior; [FR-029](./FR-029-consume-the-quire-json-contract.md) supplies the runtime-validated Quire contract.
