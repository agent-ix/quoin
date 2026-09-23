---
id: AA-10181
title: PLAT-1018 CLI fixture (unresolved evidence reference)
type: AssuranceArgument
status: active
owner: release-owner
profile: "ix://example.invalid/widget/AP-1018"
top_claim:
  id: CLAIM-1018-B
  statement: The bounded synthetic widget change is acceptable.
  subject: widget revision 0123456789abcdef
  evidence_refs:
    - "evidence://discharge/widget-1018"
reasoning:
  - id: ARG-1018-B
    statement: Argue from the explicitly reviewed clause disposition.
    supports: CLAIM-1018-B
    sufficiency_criteria:
      - Every binding synthetic clause has a current disposition.
assumptions:
  - id: ASM-1018-B
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
  - id: CH-1018-B
    target: CLAIM-1018-B
    statement: A bounded recovery case needed review.
    status: resolved
    owner: release-owner
    resolution_refs:
      - "evidence://experiment/recovery-1018"
relationships:
  - target: "ix://example.invalid/widget/AP-1018"
    type: references
---

# PLAT-1018 CLI fixture (unresolved evidence reference)

A `top_claim` citing `evidence://discharge/widget-1018`, which the CLI's
`--evidence` index never resolves, driven through the real `quoin assurance
--argument` binary (PLAT-1018).
