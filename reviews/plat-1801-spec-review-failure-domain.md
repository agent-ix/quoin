---
id: SR-20261117175268
title: "Spec review — PLAT-1801 failure domain"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-210-retained-proof-integrity-statement.md"
review_set: subset
---

## Summary

Prior failure-domain review found the provider's immutability claim lacked an actual update/delete attempt. AC-4 now tests these operations against each retained object.

## Verdict

**PASS** — repeatable provider integration verification includes destructive-attempt refusal and readback.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Test destructive update/delete behavior against the selected retention provider. | spec/functional/FR-210-retained-proof-manifest.md:174-180 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | FR-210 now permits a second valid tag over the same checkpoint payload and explicitly states that there is no prior object-id pinning without a separate trusted anchor. The requirement under review was narrowed to changed content, mismatch, wrong purpose/scope, or untrusted/bad signatures. |

### Round 1 evidence

FR-210-AC-4 requires update and deletion attempts for each retained object and proves that exact bytes remain retrievable while the provider refuses both operations under the selected policy.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | A second trusted tag object over the same statement can satisfy FR-080, so AC-5 cannot require every tag-object substitution to fail without an independently trusted expected tag object ID. | spec/functional/FR-210-retained-proof-integrity-statement.md:119-124,128-130,181 |

### FND-002 — no independent expected tag object identity

FR-210 asks FR-080 to verify the exact submitted signed tag object and returns the verified object ID, but the request carries no previously trusted expected tag object ID. Replacing the retained tag with a second annotated tag signed by the same trusted signer over the same purpose, statement digest, and scope still verifies the same content and yields a different valid object ID. Therefore the blanket AC-5 refusal for exact-tag-object substitution has no independent expected identity to compare. Either bind the expected object ID to a prior trusted record, or define the refusal as altered/untrusted object content and remove the equivalent-valid-object case from AC-5.


### Round 2 evidence

FR-210 requires parsing and signature verification from the same fetched immutable object, returns the full object ID, rejects parsed-versus-verified object mismatch, and allows a second same-payload trusted tag as a distinct checkpoint. FR-065-AC-24 reports the returned object ID and explicitly makes no prior-ID pinning claim without a separate trusted anchor.
