---
id: SR-167
title: "Dependency review of FR-115 (quoin#646)"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin@b2b8df657fda0fd86caf75aff45436174223fdac; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-021-launch-ix-flow-runs.md, spec/matrix.md, spec/log.md, spec/functional/index.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

# Dependency review: FR-115 (PR quoin#646)

## Summary

Ticket: PLAT-1080. This review checks the new `relationships:` edges
(StR-004 traces_to; FR-030, FR-032 and quire-rs FR-050 requires; FR-021
references) against the Dependencies section and CR-001. CR-001 correctly
separates enablement (the quire-rs `coverage_matrix` contract and the quoin
repin) from feature work, and it authorizes no interim shape. The one real
enablement gap, the auditor failed-run check, is recorded as base FND-001.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | FR-040 is listed as an Upstream dependency, and `requirement_of` plus the `reason`-on-empty convention are reused from it (AC-7, Outputs). It has no `relationships:` edge, so graph queries for FR-040's dependents miss FR-115. Add `ix://agent-ix/quoin/FR-040` (`requires`). | spec/functional/FR-115-evidence-backed-test-matrix.md:5-17, spec/functional/FR-115-evidence-backed-test-matrix.md:188-192 |

## Verdict

One LOW finding. It is not blocking.

## Dispositions

Round 1, reviewed at agent-ix/quoin@e609508fcf96f3b97d5ec817c5ff051e56653c34.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e609508 |

Round 2, reviewed at agent-ix/quoin@3d058f9af6c53cc47f7c3b8363852046e17f1e59.

No finding in this file was open at round 2, so no rows are added. The round-2 diff does not touch anything this analysis covers; it was re-checked for regressions and none was found.

Round 3, reviewed at agent-ix/quoin@6c0b5d0ece17df9659fcca18bc2139d9a9131efa.

No finding in this file was open at round 3, so no rows are added. The round-3 diff (AC-11 only, plus SR copies) was re-checked for regressions and none was found.
