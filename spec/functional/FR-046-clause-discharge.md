---
id: FR-046
title: "Explicit clause discharge accounting"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-040"
    type: "extends"
---

# FR-046: Explicit clause discharge accounting

## Description

Quoin SHALL consume a validated `clause-binding-v1` report and partition every
binding clause into direct evidence, an approved disposition, or open work.
Unresolved applicability SHALL remain a separate population: it is neither a
binding clause nor a discharged clause until its missing context is resolved.

Every discharge fact SHALL identify its source revision and evidence digest,
name the actor and authority making the attestation, and carry an explicit
validity interval. Expired, future-dated, or incoherent attestations leave the
clause open. The report SHALL expose every input population without emitting an
aggregate score.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | The consumer validates input against the pinned `clause-binding-v1` schema before using it. | Test (TC-1125) |
| FR-046-AC-2 | Every binding clause appears exactly once under direct evidence, approved disposition, or open work, and the result contains no score. | Test (TC-1126) |
| FR-046-AC-3 | Unresolved and not-binding clauses remain outside the binding partition; facts supplied for them are reported as unused. | Test (TC-1127) |
| FR-046-AC-4 | Expired, future-dated, or incoherent attestations leave a binding clause open with the reason visible. | Test (TC-1128) |
| FR-046-AC-5 | Duplicate facts and incomplete attestations are rejected rather than resolved by ordering or defaults. | Test (TC-1129) |
| FR-046-AC-6 | `quoin discharge` renders every population as deterministic Markdown or JSON and accepts both binding and fact inputs as files. | Test (TC-1130) |

## Constraints

- Applicability and discharge are separate decisions.
- A disposition is evidence of an authorized decision, not evidence that the
  clause's expected output exists.
- The implementation uses only caller-provided data and an explicit `asOf`
  instant; it does not read the wall clock.
- This feature makes no claim of compatibility or conformance with an external
  assurance-argument format.

## Dependencies

- FR-029 supplies the pinned, runtime-validated Quire contract.
- FR-040 consumes the open/discharged partition when rendering an authored
  argument.
