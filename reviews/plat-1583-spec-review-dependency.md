---
id: SR-17084
title: Dependency review of quoin PR 872 (PLAT-1583, FR-175)
type: SpecReview
analysis: dependency
scope: agent-ix/quoin@5fbc1576 (PR 872); spec/functional/FR-175-anomaly-check-fix-verification-evidence.md, spec/functional/index.md
review_set: subset
date: '2026-11-13'
---

# Dependency review of quoin PR 872 (PLAT-1583, FR-175)

## Summary

Ticket: PLAT-1583. Checked the seven `relationships` edges of FR-175 and its Dependencies section against the target documents.

## Verdict

Clean. `traces_to` StR-009 fits: the check reads results that the run emitted, never annotations. `implements` spec-artifacts-process US-003 matches FR-136's own edge. The `depends_on` edges on FR-136, FR-030, FR-120 and spec-artifacts-process FR-015 are the real upstreams, and all of them resolve on main. `references` NFR-035 is the correct weak edge, not `constrained_by`, because NFR-035's scope is `quoin anomaly report` only. FR-175 restates the no-verdict and no-write obligations itself (lines 47 and 85, AC-10) instead of borrowing NFR-035's. All edge types are already used in the corpus (`depends_on` 161, `traces_to` 97, `references` 64, `implements` 52). The missing quire-rs FR-053-AC-11 dependency is part of the base finding SR-17082 FND-001, not a separate edge defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
