---
id: SR-16400
title: EARS review of FR-123 model-table mapping
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quoin; spec/functional/FR-123-model-table-sections-mapping.md; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-071-typed-properties-mapping.md; spec/matrix.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# EARS review of FR-123 model-table mapping

## Summary

Ticket: PLAT-240 (agent-ix/quoin PR #780, head 45b5004a).

Checked every FR-123 behaviour statement and the FR-071/FR-121 edits for EARS form (one SHALL, one subject, testable trigger). Most bullets conform. Two defects; the `owner` statement's missing SHALL is recorded under SR-16399 FND-003.

## Scope

- `FR-123` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-read` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-row` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-refusal` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-dup` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-empty` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#states` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#transitions` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#steps` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#vocabulary` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#no-sourcespan` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#owned-headings` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#emits-persists` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#source` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#owner` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#ubiquitous-language` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#unmapped-members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-071#enumeration` (examined) — spec/functional/FR-071-typed-properties-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The bullet that keeps model-table sections away from FR-121 has no SHALL and is stated in headings (`## Workflow`, `## Ubiquitous Language`), while FR-121 decides ownership by normalized record property name, so the two rules do not meet: `## Workflow` names no property, and `## Ubiquitous Language` normalizes to `ubiquitous_language`, not `vocabulary`. | spec/functional/FR-123-model-table-sections-mapping.md:67 |
| FND-002 | low | The Description says Quoin SHALL publish the mapping 'as golden fixtures that the Quire extractor implements', while CON-1 says the quire-fixtures crate in quire-rs owns and authors them and Quoin holds no copy; Quoin publishes no fixtures. | spec/functional/FR-123-model-table-sections-mapping.md:23 |

## Details

- **FND-001** (FR-123#owned-headings): Fix: restate as 'Quire SHALL NOT read under FR-121 a record key that a declared FR-075 table locator of the object type owns', in key terms, and let FR-121's list reference that rule.

- **FND-002** (FR-123): Fix: 'Quoin SHALL specify the mapping, whose golden fixtures live in quire-rs quire-fixtures'. FR-121's Description carries the same wording (merged) and can take the same fix.

## Verdict

Medium and low findings only; the PR is not mergeable because of SR-16399 and SR-16402.

## Dispositions

Round 1, against head 8ecfb3ef.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
