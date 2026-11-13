---
id: SR-20261113084431
title: "Integrity review of the FR-115 CR-004 note and FR-165 index entry"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #858 (git diff origin/main...plat-1529-spec-quoin at 87825c1e): spec/functional/FR-165-matrix-design-column.md, spec/functional/FR-115-evidence-backed-test-matrix.md (CR-004 note), spec/functional/index.md"
relationships:
  - target: ix://agent-ix/quoin/FR-165
    type: references
---

# Integrity review of the FR-115 CR-004 note and FR-165 index entry

## Summary

Ticket: PLAT-1529. Checks the FR-115 CR-004 amendment, the index entry and FR-165's consistency with FR-115's Inputs, Outputs and Behavior. The index entry is correct and in id order. The CR-004 note covers the `reason`/non-empty `requirements` change, and AC-5 tests it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | CR-004 records only the `reason`/`requirements` change. FR-165 also supersedes two more FR-115 statements. FR-115 Inputs define `coverage` as a bare array of `{document, criteria[]}` under `deny_unknown_fields` (FR-115-AC-2). The `design_matrix` entries add `id`, `repo`, `design` and `design_elements`, which that rule refuses. FR-115 Behavior also says `quoin matrix` assembles its input by "running `quire coverage`", and FR-165 replaces that with `quire.design_matrix`. A reader of FR-115 alone would build a request type that rejects the new payload. | spec/functional/FR-115-evidence-backed-test-matrix.md:162-167,212,234,322-323 |

## Examined scope

```yaml
scope:
- id: FR-115-CR-004
  path: spec/functional/FR-115-evidence-backed-test-matrix.md
  role: examined
  excerpt: '**CR-004 (PLAT-1529):** [FR-165](./FR-165-matrix-design-column.md) adds the design column. With entries that have no criteria, `reason` and a non-empty `requirements` can coexist, as FR-165 states.'
- id: FR-115-IN
  path: spec/functional/FR-115-evidence-backed-test-matrix.md
  role: context_only
  excerpt: '`coverage`: the quire-rs `coverage_matrix` payload as published by FR-050-AC-47..51 — a **bare array** of `{document, criteria[]}` entries (`criteria[].id`/`method`/`binders`/`status`), with no wrapping object'
- id: index
  path: spec/functional/index.md
  role: examined
  excerpt: '- [FR-165: Design column in the evidence-backed test matrix](./FR-165-matrix-design-column.md)'
- id: FR-165-OUT
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: 'Each `requirements[]` entry of `MatrixOutput` carries `design`, the entry''s `design` array from the `coverage` document that holds the requirement''s criteria, verbatim and in quire-rs order, and absent when that entry has none. When one document holds criteria of several requirement ids (FR-115 grouping), each of those requirements carries the document''s `design`. ... A requirement entry with `criteria: []` is grouped under the entry''s document `id` (quire-rs FR-092), ordered by requirement id among the others'
- id: FR-165-IN
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`quoin matrix` obtains it through a `quire.design_matrix` operation in `quoin-core`, registered beside `quire.coverage` (which is unchanged, FR-092-CON-2), over the working tree''s single corpus member.'
```

## Verdict

Changes requested: 0 high, 1 medium. CR-004 records only one of the FR-115 statements FR-165 supersedes.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | Merged quire-rs FR-092 also puts `criticality` and, when `carried_fields` is declared, `carried` (FR-090) on design entries (key order `document, repo, id, criticality, carried, criteria, design`). CR-004 (1) and FR-165 Inputs list only `id`, `repo`, `design` and `design_elements`. Today `CoverageMatrixRequirement` has no `deny_unknown_fields`, so nothing breaks. But an implementer who follows (1) literally and adds the listed keys to a closed entry type would refuse `carried`. List the full key set, or say the entry type ignores keys it does not read. | spec/functional/FR-115-evidence-backed-test-matrix.md:322-327 |

## Dispositions

Round 1, verified at 75df2882.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
