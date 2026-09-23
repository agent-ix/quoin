---
id: AA-10180
title: PLAT-1018 CLI fixture (no evidence_refs)
type: AssuranceArgument
status: active
owner: release-owner
profile: "ix://example.invalid/widget/AP-1018"
top_claim:
  id: CLAIM-1018-A
  statement: The bounded synthetic widget change is acceptable.
  subject: widget revision 0123456789abcdef
  evidence_refs: []
reasoning:
  - id: ARG-1018-A
    statement: Argue from the explicitly reviewed clause disposition.
    supports: CLAIM-1018-A
    sufficiency_criteria:
      - Every binding synthetic clause has a current disposition.
assumptions:
  - id: ASM-1018-A
    statement: The test environment represents the bounded target.
    owner: release-owner
    status: accepted
    review_by: "2026-09-01T00:00:00.000Z"
participants:
  - id: reviewer-1018
    role: decision reviewer
    authority: may accept or reject this synthetic release
    independence: did not produce the implementation evidence
challenges:
  - id: CH-1018-A
    target: CLAIM-1018-A
    statement: A bounded recovery case needed review.
    status: resolved
    owner: release-owner
    resolution_refs:
      - "evidence://experiment/recovery-1018"
relationships:
  - target: "ix://example.invalid/widget/AP-1018"
    type: references
---

# PLAT-1018 CLI fixture (no evidence_refs)

A `top_claim` with an empty `evidence_refs` array, driven through the real
`quoin assurance --argument` binary (PLAT-1018).
