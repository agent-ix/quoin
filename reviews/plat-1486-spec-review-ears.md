---
id: SR-20261112024908
title: "EARS conformance review of quoin PR 830 (FR-137..139)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin@3157f4ba; spec/functional/FR-137-detect-and-place-licensed-packs.md, spec/functional/FR-138-never-publish-an-installed-pack.md, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md"
review_set: subset
---

# EARS conformance review of quoin PR 830

## Summary

Ticket: PLAT-1486. Classified the Behavior bullets of FR-137 (14), FR-138 (8) and FR-139 (5).
All are ubiquitous except FR-137's two `If ... then` load-failure bullets. `quire validate`
reports three `ears:non-singular` warnings; none is justifiable, each joins two obligations.

## Verdict

**PASS WITH FINDINGS, one low.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three bullets carry two `SHALL`s: FR-137:40 (decide by `check_publishable` / declare no pack field), FR-138:36 (expose one guard / every publish path calls it), FR-139:32 (name sets as `authority/id@version` / not quote clause text). Split each into two bullets so the three warnings clear | spec/functional/FR-137-detect-and-place-licensed-packs.md:40, spec/functional/FR-138-never-publish-an-installed-pack.md:36, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md:32 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-137, path: spec/functional/FR-137-detect-and-place-licensed-packs.md, role: examined, excerpt: "Behavior lines 40-53; ubiquitous x12, unwanted (If/then) x2"}
- {id: FR-138, path: spec/functional/FR-138-never-publish-an-installed-pack.md, role: examined, excerpt: "Behavior lines 36-43; ubiquitous x8"}
- {id: FR-139, path: spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, role: examined, excerpt: "Behavior lines 28-32; ubiquitous x5"}
```
