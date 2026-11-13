---
id: SR-20261113051916
title: "Dependency review of quoin risk advise (FR-145..FR-148, NFR-031)"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #845 (git diff origin/main...HEAD at 96566e29): spec/functional/FR-145..FR-148, spec/non-functional/NFR-031, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md"
---

# Dependency review of quoin risk advise (FR-145..FR-148, NFR-031)

## Summary

Ticket: PLAT-1651. Frontmatter `relationships` against each file's Dependencies section and
behaviour. All ix:// targets resolve on `origin/main` (quoin FR-136, FR-144; spec-objects-safety
FR-101, 102, 103, 104, 202, 203, 204, 205, 302, 303, 305). No cycle; FR-145 is the root and
FR-146..148 and NFR-031 hang off it, matching the NFR-030 / FR-141..144 pattern. FR-147's use of
FR-302 needs no quire-rs PLAT-1674 selector, because quoin reads the edges itself, so there is no
hidden blocker. Two prose-declared upstreams have no frontmatter edge. The missing retained-evidence
dependency is recorded once, in the integrity review (FND-003).

Examined (all role examined): FR-145..FR-148 statements and all their criteria, NFR-031
statement and NFR-031-AC-1..AC-7. Context only: spec-objects-safety FR-101..107, FR-202..205,
FR-301..305 on origin/main, quoin FR-136, FR-144, StR-007, StR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Dependencies lists FR-136 as Upstream and the behaviour reads FR-136 anomaly documents, but the frontmatter has no depends_on ix://agent-ix/quoin/FR-136 edge. | FR-145:4-14,62 |
| FND-002 | low | Dependencies lists FR-146 as Upstream and line 36 reuses FR-146's finding tokens, but the frontmatter has no depends_on ix://agent-ix/quoin/FR-146 edge. | FR-148:4-14,36,60 |

## Verdict

Mergeable on dependency grounds after two low edge additions.
