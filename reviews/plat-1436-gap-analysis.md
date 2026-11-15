---
id: SR-205
title: "Gap analysis of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR #924 @ ab633975: FR-188..FR-192, NFR-039, US-024; cited sources rust/crates/quoin-quire/src, rust/crates/quoin-assurance/src/matrix.rs, rust/crates/quoin-assurance/src/discharge/rmm.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: references
  - target: "ix://agent-ix/quoin/FR-190"
    type: references
---

## Summary

Ticket: PLAT-1436. A planless gap analysis of a spec-only change.

## Coverage

Plan completion: not assessed.

`quoin matrix --json` (quoin 0.34.0) lists FR-188 (11 criteria), FR-189 (5),
FR-190 (5), FR-191 (5), FR-192 (6) and NFR-039 (7 rows). Every one is
`untagged` with `no run evidence`. This is expected. The PR adds no production
code, and the implementation belongs to PLAT-1614 (command), PLAT-1437 (SRS and
architecture) and PLAT-1438 (trace matrix and RMM), which this ticket blocks.
The untagged criteria are the work those tickets have to do. They are not a
defect of this PR.

Reverse gaps: no production code changed, so there is no code without an owning
requirement.

The source claims the spec makes about existing code were checked by reading it:

- `rust/crates/quoin-quire/src/design_join.rs` and `design_matrix.rs` exist, as
  FR-190 states. This corrects the ticket's older claim that they were not
  exposed.
- `quoin-quire` exposes no `PartRecord`, `PortRecord` or `ConnectionRecord`
  (grep finds none), as FR-190's dependency-gap bullet states and assigns to
  PLAT-1437.
- `rust/crates/quoin-assurance/src/discharge/rmm.rs`, `pack_rmm.rs` and
  `rust/crates/quoin-cli/src/discharge.rs` exist, as FR-192 cites.
- The fields that `quoin matrix --json` actually emits differ from what FR-191
  requires. That finding is recorded in SR-209 (dependency), where it belongs.

The optional semantic review was not run.

## Verdict

PASS for gap analysis. Coverage gaps are expected until the implementation
tickets land, and the code claims the spec makes are accurate apart from the
matrix-field issue recorded in SR-209.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
