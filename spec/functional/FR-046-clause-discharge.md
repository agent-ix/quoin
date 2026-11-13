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

Every discharge fact SHALL name the actor and authority making the attestation, and carry an explicit
validity interval. Expired, future-dated, or incoherent attestations leave the
clause open. The report SHALL expose every input population without emitting an
aggregate score.

A disposition fact SHALL carry the explicitly chosen decision, its owner and
approval reference. A `temporary_exception` fact SHALL additionally carry a
top-level `decisionExpiresAt`, distinct from `attestation.expiresAt`; the former
limits the decision and the latter limits the attestation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | The consumer validates input against the `clause-binding-v1` schema before using it. | Test |
| FR-046-AC-2 | Every binding clause appears exactly once under direct evidence, approved disposition, or open work, and the result contains no score. | Test |
| FR-046-AC-3 | Unresolved and not-binding clauses remain outside the binding partition; facts supplied for them are reported as unused. | Test |
| FR-046-AC-4 | Expired, future-dated, or incoherent attestations leave a binding clause open with the reason visible. | Test |
| FR-046-AC-5 | Duplicate facts and incomplete attestations are rejected rather than resolved by ordering or defaults. | Test |
| FR-046-AC-6 | `quoin discharge` renders every population as deterministic Markdown or JSON and accepts both binding and fact inputs as files. | Test |
| FR-046-AC-7 | A disposition fact uses exactly one decision from `delegated`, `accepted_risk` or `temporary_exception`, names its owner and approval reference, and carries a valid human-supplied actor, authority and validity interval; a `temporary_exception` also carries `decisionExpiresAt` while other decisions omit it. | Test |
| FR-046-AC-8 | A `temporary_exception` fact discharges its binding clause only while `asOf` is earlier than `decisionExpiresAt` and its attestation is current; when `asOf` equals or exceeds `decisionExpiresAt`, the clause is open with reason `temporary exception expired`. | Test |

## Constraints

- Applicability and discharge are separate decisions.
- A disposition is evidence of an authorized decision, not evidence that the
  clause's expected output exists.
- The decision, owner and approval reference are authored explicitly by a human. An approval reference and a `scope_exclusion` assertion are references/assertions only; neither independently proves authorization.
- `temporary_exception` is bounded by its authored expiry and the attestation's validity interval. The authored expiry is copied to the fact's `decisionExpiresAt`; the fact remains open at that instant even if `attestation.expiresAt` is later.
- The implementation uses only caller-provided data and an explicit `asOf`
  instant; it does not read the wall clock.
- This feature makes no claim of compatibility or conformance with an external
  assurance-argument format.

## Dependencies

- FR-029 supplies the runtime-validated Quire contract.
- FR-040 consumes the open/discharged partition when rendering an authored
  argument.
