---
id: SR-20261113084433
title: "EARS conformance review of FR-165"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #858 (git diff origin/main...plat-1529-spec-quoin at 87825c1e): spec/functional/FR-165-matrix-design-column.md, spec/functional/FR-115-evidence-backed-test-matrix.md (CR-004 note), spec/functional/index.md"
relationships:
  - target: ix://agent-ix/quoin/FR-165
    type: references
---

# EARS conformance review of FR-165

## Summary

Ticket: PLAT-1529. Checks the FR statement, the Outputs, Gap and Rendering normative prose, every AC and CON-1 against EARS patterns and atomicity. The Description SHALL and CON-1 are well-formed ubiquitous statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The obligations in Outputs, Gap and Rendering are written as indicative description ("carries", "is shown as a gap", "gains a `Design` column", "prints its `##` heading"), with no SHALL. The only SHALLs are the Description sentence and CON-1. FR-115 puts these rules in a Behavior section as SHALL statements. | spec/functional/FR-165-matrix-design-column.md:44-69 |
| FND-002 | low | AC-9 is compound: a source inspection, plus a CLI check that `quoin matrix` has no `--strict`. Its "verb" is unqualified, so any function name would fail it literally. FR-092-AC-13 says "realizing verb". | spec/functional/FR-165-matrix-design-column.md:83 |
| FND-003 | low | AC-6 is compound: evidence statuses unchanged by `design`, plus byte-identical output across two runs. These are independent properties. | spec/functional/FR-165-matrix-design-column.md:80 |

## Examined scope

```yaml
scope:
- id: FR-165
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`quoin matrix` SHALL carry that design column through beside the evidence status, so one view reads requirement, design element, test and result, with the evidence freshness of FR-115 as the result.'
- id: FR-165-OUT
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: 'Each `requirements[]` entry of `MatrixOutput` carries `design`, the entry''s `design` array from the `coverage` document that holds the requirement''s criteria, verbatim and in quire-rs order, and absent when that entry has none. When one document holds criteria of several requirement ids (FR-115 grouping), each of those requirements carries the document''s `design`. ... A requirement entry with `criteria: []` is grouped under the entry''s document `id` (quire-rs FR-092), ordered by requirement id among the others'
- id: FR-165-GAP
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: A requirement with an `uncovered` block is a design gap. It is shown as a gap in every render, naming the block's `check` token, whether or not it has criteria. A gap does not change `evidence_status`.
- id: FR-165-REN
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`quoin matrix` markdown gains a `Design` column between `Criterion` and `Method`, listing each `design_elements` entry, resolved against `design`, as `id (kind)` (`id` when it has no `kind`) and `no design element` for a criterion under an `uncovered` block, and an empty cell for an entry with no `design` key. A requirement with no criteria prints its `##` heading and a line naming the gap.'
- id: FR-165-AC-1
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: Given a `coverage` with a requirement carrying a `covered` block and another carrying an `uncovered` block, `matrix.build` returns each requirement with the `design` array verbatim as supplied, and `design_elements` per criterion verbatim as supplied.
- id: FR-165-AC-2
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: A supplied criterion `design_elements` list, empty or not, reaches the output unchanged; `matrix.build` does not recompute it from `realized_criteria`.
- id: FR-165-AC-3
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: A requirement whose `design` block is `uncovered` renders in markdown with `no design element` in the `Design` cell and its `check` token, and its `evidence_status` is the one FR-115 computes for the same criterion without the `design` key.
- id: FR-165-AC-4
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: A requirement with no `design` key has an empty `Design` cell and no `design` key in `--json`.
- id: FR-165-AC-5
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: 'A requirement with `criteria: []` and an `uncovered` block appears in `requirements[]` and in markdown with a gap line, and a `coverage` holding only such entries yields the FR-115 `reason`, those entries in `requirements[]` and exit 0.'
- id: FR-165-AC-6
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: Over a fixture store with one `bound`, one `stale` and one `undischarged` criterion, the evidence statuses equal those of the same run without `design` in `coverage`, and two runs over byte-identical input give byte-identical canonical output.
- id: FR-165-AC-7
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: The `Design` column sits between `Criterion` and `Method`, and `--json` names each element's `kind`.
- id: FR-165-AC-8
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: A `coverage` with no `design` on any entry yields output equal to FR-115's.
- id: FR-165-AC-9
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`matrix.build` source holds no archetype name, verb or design join, and `quoin matrix` has no `--strict` flag.'
- id: FR-165-AC-10
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`quoin matrix` leaves `spec/` byte-identical in a fixture repository with `design_relations` declared.'
- id: FR-165-CON-1
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`matrix.build` SHALL leave design blocks and `design_elements` as quire-rs supplies them, neither computing, filtering nor reordering any.'
```

## Verdict

Changes requested: 0 high, 3 low. Wording only.
