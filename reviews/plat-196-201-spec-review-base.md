---
id: SR-16393
title: base checklist review of FR-121 and FR-122
type: SpecReview
analysis: base
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# base checklist review of FR-121 and FR-122

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

ID formats, duplicates, link integrity, hash/pin antipatterns and test-coverage rules. FR-121, FR-122, their AC/CON ids and TC-2013..TC-2027 collide with nothing on origin/main (03dc34be) or in open quoin PRs #764/#774/#777, and quire-rs open PRs #611-#620 add none of them. quire validate passes on the changed files. No hash, digest, pin or SHA tracking added. No defects beyond those in the lens reviews.

## Scope

- `FR-121-AC-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-4` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-5` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-6` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-7` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-8` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122-AC-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-3` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-4` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-5` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-6` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-7` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `matrix#FR-121` (examined) — spec/matrix.md
- `matrix#FR-122` (examined) — spec/matrix.md
- `FR-121` (context_only) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122` (context_only) — spec/functional/FR-122-architecture-record-keys-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

No defects for this lens.
