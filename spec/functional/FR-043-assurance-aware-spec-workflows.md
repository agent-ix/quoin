---
id: FR-043
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

# FR-043: Assurance-aware specification authoring and review selection

## Description

The specification skills SHALL compose installed assurance artifacts and profile-selected
reviews with the existing authoring and review lifecycle.

The specification skills SHALL preserve the live catalog and `SpecReview` schemas as the
format authority, ordinary low-impact behavior, one review artifact per analysis, and the
human confirmation boundary.

An assurance profile recommendation is a default a user may change. A required selection is
different: the guided review records the profile and SHALL refuse an intake selection that does
not equal the profile set. Neither mode changes finding severity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-1 | When a request names an installed assurance artifact, `/specify` obtains its skeleton and schema through the same `quoin write` authoring pack and links the artifact using only admitted relationships. | Test (TC-277) |
| FR-043-AC-2 | When an ordinary request names no assurance need, `/specify` creates no unsolicited assurance artifact and retains its confirmation boundary. | Test (TC-278) |
| FR-043-AC-3 | When an applicable profile recommends analyses, the review workflow records the recommendation while preserving the user's base, all, or subset choice. | Test (TC-279) |
| FR-043-AC-4 | When an applicable profile requires analyses, the intake gate refuses a selected set that differs from the profile set or omits the profile path. | Test (TC-280) |
| FR-043-AC-5 | When the all set is selected, the workflow derives the complete declared analysis vocabulary and cannot accept a truncated caller-supplied list. | Test (TC-281) |
| FR-043-AC-6 | When a selected analysis has no recorded review document, the final gate names it and refuses acceptance. | Test (TC-282) |
| FR-043-AC-7 | When a profile selects an analysis absent from the installed review contract, intake reports it as unsupported instead of authoring an invalid document. | Test (TC-283) |
| FR-043-AC-8 | When no assurance profile applies and base is selected, the existing zero-analysis review path remains accepted. | Test (TC-284) |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-043-CON-1 | The skills SHALL NOT copy assurance rubrics or artifact schemas into Quoin; installed modules remain authoritative. | Design | Inspection |
| FR-043-CON-2 | The workflow SHALL NOT automatically acknowledge its human acceptance gate. | Safety | Inspection |
| FR-043-CON-3 | A profile selection SHALL NOT assign finding severity. | Policy | Test (TC-279, TC-280) |

## Dependencies

- **Upstream**: [FR-014](./FR-014-emit-authoring-contract.md), [FR-021](./FR-021-launch-ix-flow-runs.md)
- **External**: the installed assurance artifact module and `SpecReview.analysis` schema
