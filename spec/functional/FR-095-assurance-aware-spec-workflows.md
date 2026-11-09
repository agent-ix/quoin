---
id: FR-095
title: "Assurance-aware specification authoring and review selection"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-014"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "extends"
---

# FR-095: Assurance-aware specification review

## Description

Specification review SHALL use the installed AssuranceProfile and SpecReview
contracts. Profiles SHALL use review_policy.mode and review_policy.operations;
Quoin SHALL NOT interpret the obsolete review_selection field. A required
spec-review operation SHALL require an accepted enforced review run. Operations
code-review and gap-analysis SHALL remain separate from specification lenses.

The user SHALL choose base, all or subset independently of the profile's operation
policy. Base SHALL always be required. All SHALL expand to the seven declared
specification lenses: failure-domain, integrity, dependency, evidence,
risk-complexity, scope-boundary and ears-conformance. The native provider SHALL
check every selected lens against the live SpecReview.analysis vocabulary.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-095-AC-1 | Named installed assurance artifacts obtain their formats from the live authoring pack. | Test |
| FR-095-AC-2 | Ordinary authoring creates no unsolicited assurance artifact. | Test |
| FR-095-AC-3 | The run records applicable active profile paths and their applicability disposition, or an explicit no-profile disposition. | Test |
| FR-095-AC-4 | The provider validates actual profile documents against the live catalog and refuses obsolete or malformed policies. | Test |
| FR-095-AC-5 | All expands to the complete declared specification-lens registry and refuses a truncated caller list. | Test |
| FR-095-AC-6 | Missing, unreadable, malformed, mislabeled or invalid required review documents refuse validation and acceptance. | Test |
| FR-095-AC-7 | A selected lens absent from the installed SpecReview contract is reported as unavailable. | Test |
| FR-095-AC-8 | Base selection requires a real validated base review and explicit final human acknowledgement. | Test |
| FR-095-AC-9 | Changes to registered documents are revalidated at final acceptance. | Test |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-095-CON-1 | Quoin SHALL obtain artifact schemas from installed catalog modules without copying their rubrics or schemas into production workflow assets. | Design | Inspection |
| FR-095-CON-2 | Quoin SHALL NOT automatically acknowledge human acceptance. | Safety | Test |
| FR-095-CON-3 | Profile policy SHALL NOT assign finding severity or invent lens requirements. | Policy | Test |

## Dependencies

- Upstream: [FR-014](./FR-014-emit-authoring-contract.md), [FR-021](./FR-021-launch-ix-flow-runs.md).
- External: installed assurance and SpecReview catalog contracts.
