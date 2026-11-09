---
id: SR-16403
title: gap analysis of PR 780 (FR-123)
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; spec/functional/FR-123-model-table-sections-mapping.md; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-071-typed-properties-mapping.md; spec/matrix.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# gap analysis of PR 780 (FR-123)

## Summary

Ticket: PLAT-240 (agent-ix/quoin PR #780, head 45b5004a).

Planless. Plan completion: not assessed. Spec-only diff, no production code. `quire matrix --scope .` (quire 0.36.2) lists FR-123-AC-1..AC-8 and CON-1 untagged and CON-2 method-without-symbol, as expected for Pending rows executed in quire-rs. One defect.

## Scope

- `FR-123-AC-1` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-2` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-3` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-4` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-5` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-6` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-7` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-8` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-CON-1` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-CON-2` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#unmapped-members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#owner` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#ubiquitous-language` (examined) — spec/functional/FR-123-model-table-sections-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-8 passes a property owned by a named requirement or 'listed as unmapped in this requirement', but FR-123 lists only domain and population `members` as unmapped; `owner` (NestedEntity) and `ubiquitous_language` (Domain) are neither owned (FR-123 says it does not extract them) nor listed unmapped, so AC-8's outcome on today's schemas is undefined. | spec/functional/FR-123-model-table-sections-mapping.md:95 |

## Details

- **FND-001** (FR-123-AC-8): Fix: list `owner` and `ubiquitous_language` explicitly, as unmapped or as owned by a named requirement, together with the resolution chosen for SR-16399 FND-003/FND-008.

## Verdict

Pending rows consistent; fix the AC-8 owner list.
