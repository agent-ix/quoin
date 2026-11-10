---
id: SR-16401
title: evidence review of FR-123 acceptance criteria
type: SpecReview
analysis: evidence
scope: agent-ix/quoin; spec/functional/FR-123-model-table-sections-mapping.md; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-071-typed-properties-mapping.md; spec/matrix.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# evidence review of FR-123 acceptance criteria

## Summary

Ticket: PLAT-240 (agent-ix/quoin PR #780, head 45b5004a).

Checked each FR-123 AC and CON for a failing case, a verification method and a TC row with an audit row in spec/matrix.md. AC-1..AC-5 and AC-8 can fail; every AC has one TC (TC-2120..TC-2127) and an AC-table row; TC ids collide with nothing on origin/main or open quoin PRs #764/#774/#777. Four defects.

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
- `FR-123#values-empty` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `matrix#FR-123` (examined) — spec/matrix.md
- `matrix#FR-123-ac` (examined) — spec/matrix.md
- `matrix#TC-2127` (examined) — spec/matrix.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-7's negative halves ('extracts no owner from ## Parent', 'extracts no ubiquitous_language from its table') name no `semantic.mappings` token; on a business-module fixture (no `record-sections`) nothing reads those sections at all, so a do-nothing extractor passes them. | spec/functional/FR-123-model-table-sections-mapping.md:94 |
| FND-002 | medium | AC-6's 'a `## Emits` section is prose' names no token; without `record-sections` no mapping reads any `## Emits` section, so the clause passes vacuously; its emits/persists halves restate quire-rs FR-076-AC-16, already tested there. | spec/functional/FR-123-model-table-sections-mapping.md:93 |
| FND-003 | low | TC-2127 is a Static ownership audit over spec-objects-business schemas, but its status note is copied from the extraction rows ('Executed by the agent-ix/quire-rs extractor ... against goldens'), unlike its FR-122 twin TC-2115 which says it is an ownership audit, not an extraction. | spec/matrix.md:1438 |
| FND-004 | low | The header-only `## Values` behaviour (empty `values`, key `available`, `min_rows` left to the module) has no AC or TC. | spec/functional/FR-123-model-table-sections-mapping.md:57 |

## Details

- **FND-001** (FR-123-AC-7): They can only fail under `record-sections`, where FR-121 would otherwise read `## Ubiquitous Language` as `ubiquitous_language`. Fix: state both under a manifest naming `record-sections`. The positive owner half is FND-003 of SR-16399.

- **FND-002** (FR-123-AC-6): Fix: assert the `## Emits` (and `## Persists`) prose outcome under `record-sections`, where FR-121's typed-key exclusion is what makes it pass.

- **FND-003** (matrix#TC-2127): Fix: reuse TC-2115's note wording.

- **FND-004** (FR-123#values-empty): Add the case to AC-1 or AC-2.

## Verdict

ACs map one-to-one to TCs with audit rows; fix the vacuous clauses.

## Dispositions

Round 1, against head 8ecfb3ef.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
