---
id: SR-20261113084430
title: "Spec review of the matrix design column (quoin FR-165)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #858 (git diff origin/main...plat-1529-spec-quoin at 87825c1e): spec/functional/FR-165-matrix-design-column.md, spec/functional/FR-115-evidence-backed-test-matrix.md (CR-004 note), spec/functional/index.md"
relationships:
  - target: ix://agent-ix/quoin/FR-165
    type: references
---

# Spec review of the matrix design column (quoin FR-165)

## Summary

Ticket: PLAT-1529. Reviews new FR-165 against FR-115 and the companion specs quire-rs FR-092 (PR 670) and quire-cli FR-043 (PR 188). Cross-references use the renumbered ids (quire-cli FR-043, quoin FR-165 on both companions; no FR-042 or FR-162 left). The FR keeps quoin a pass-through: it takes `design` and `design_elements` verbatim and infers nothing. It names no archetype in engine-facing prose, adds no compatibility layer, and reproduces no paywalled text. quoin's markdown render has no cell truncation, so quire-cli's truncation finding does not recur, and `quoin matrix` has no `--scope`/`--repo` pair, so the exit-3 finding does not apply. The empty-cell ambiguity found on quire-cli recurs here (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The empty `Design` cell has more than one meaning. The FR uses it for an entry with no `design` key (out of scope, AC-4). But under a covered entry, a criterion whose `design_elements` is empty also renders empty: FR-092 says this is not a gap, and AC-2 requires the empty list to pass through. A covered block of a `to: []` relation that holds only a dangling edge renders empty too. quire-cli FR-043 already settled both cases. It prints `-> <reference> (dangling)` for the dangling-only block, and its review flagged the same collision. | spec/functional/FR-165-matrix-design-column.md:63-65 |
| FND-002 | medium | The cell for an entry with one `covered` and one `uncovered` block is undefined (FR-092-AC-6 allows several blocks). The text says `no design element` "for a criterion under an `uncovered` block", but that criterion can also carry elements from the covered block. Two implementers could print the elements alone, the marker alone, or both. The marker also differs from quire-cli FR-043, which prints `no design element (<check>)` per uncovered block (its AC-17), plus a `Design gap: ...` line under the heading. The two views of one PLAT-1529 payload would then print the same gap differently. | spec/functional/FR-165-matrix-design-column.md:63-66 |
| FND-003 | medium | The FR does not say which `design` a requirement carries when more than one document feeds its id. Under FR-115-AC-8, criteria regroup by `requirement_of(criterion.id)`, so criteria with one prefix can come from two documents with different `design` arrays. The text names "the `coverage` document that holds the requirement's criteria" as if there were only one. A `criteria: []` entry is grouped under its document `id`, and that id can equal a requirement already formed from another document's criteria. Neither the merge nor the choice is stated. A document whose criteria all carry another prefix also puts its own design gap under that other requirement's id. | spec/functional/FR-165-matrix-design-column.md:46-53 |
| FND-004 | medium | The FR states no failure path for `quire.design_matrix`. FR-092 (Errors, AC-10) makes `design_matrix` fail when a `design_relations` entry names no declared relation, while `coverage` still succeeds. FR-165 moves `quoin matrix` from `quire.coverage` to `quire.design_matrix`, so a module fault that `quoin matrix` used to survive now aborts it. The FR does not name the `CoreErrorCode`, the exit code or what is printed, and no AC covers it. The new op's own contract is also missing: request shape, byte ceiling and its `dispatch` entry beside `quire.coverage`. | spec/functional/FR-165-matrix-design-column.md:39-42 |
| FND-005 | low | The FR says a gap is shown "naming the block's `check` token" in every render. For a requirement that has criteria, it does not say where the markdown prints the token: in the cell, on a line under the heading, or both. AC-3 ("`no design element` in the `Design` cell and its `check` token") has the same gap. | spec/functional/FR-165-matrix-design-column.md:58-59,63-66 |
| FND-006 | low | Markdown never renders `kind_mismatch`, and renders `dangling` only through FND-001's empty cell. A block that is uncovered because of a kind mismatch or a dangling edge reads only `no design element`. `--json` carries both lists verbatim. quire-cli's review raised the same point (its FND-008). | spec/functional/FR-165-matrix-design-column.md:63-66 |
| FND-007 | low | AC-7 says `--json` "names each element's `kind`", but FR-092-AC-2 omits `kind` for an element with no `type`, and FR-165 renders such an element as plain `id`. Read literally, AC-7 fails on that input. No AC covers the `id`-only render. | spec/functional/FR-165-matrix-design-column.md:81 |

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
- id: FR-165-IN
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: '`quoin matrix` obtains it through a `quire.design_matrix` operation in `quoin-core`, registered beside `quire.coverage` (which is unchanged, FR-092-CON-2), over the working tree''s single corpus member.'
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

Changes requested: 0 high, 4 medium, 3 low. The pass-through principle, independence of the two axes, determinism (AC-6) and no-`--strict` stance are sound. The rendering rules need the cases quire-cli FR-043 already settled (mixed blocks, dangling-only covered blocks, empty-cell meaning), the requirement grouping needs a rule for a requirement fed by more than one document, and the `design_matrix` failure path needs stating.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | "or for any other reason ... SHALL return `CoreErrorCode::Refused`" collapses the exit taxonomy every quire op uses. `ops/quire/taxonomy.rs` `map_error` maps a malformed payload or path escape to `BadRequest` (3), I/O, an oversized payload or an unmapped code to `Io`/Internal (4), and a missing quire host to Internal (4). It always attaches `quire_code`. Under this sentence, `quire.design_matrix` must report all of these as `Refused` (2) with `reason: design-matrix-failed`, unlike `quire.coverage` with the same request. Scope the `Refused` + `design-matrix-failed` rule to the FR-086 undeclared-relation error, and route every other failure through the existing `map_error` taxonomy. | spec/functional/FR-165-matrix-design-column.md:48-54 |

## Dispositions

Round 1, verified at 75df2882.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

Round 2, verified at 5bfd0925.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The list that replaced FND-008 gets `quire.coverage`'s actual mapping wrong, and AC-19 inherits the error. (a) It leaves out `map_error`'s `Refused` (2) class, the engine refusals (`ModuleLoad`, `ArchetypeUnknown`, `TraceabilityModelUndeclared` and others) that carry `quire_code`. "Every other failure" then reads as exit 3 or 4 for them. (b) An oversized request is not a `map_error` case: `read()` returns `refusal(op, MAX_REQUEST_BYTES, ..)` and `check_bound` returns `Refused` for an oversized path, both exit 2 (`ops/quire/mod.rs:263-264,298-300`). Only the engine's output `PayloadTooLarge` maps to Internal. (c) A missing quire host comes from `host()`, not `map_error`, and carries no `quire_code`. AC-19's "an oversized payload ... not `Refused`" therefore contradicts what `quire.coverage` does for an oversized request. Fix: say "the same codes `quire.coverage` gives for the same cause (its request and path ceilings, `host()`, and `map_error`)" without restating the list, and in AC-19 replace "not `Refused`" with "not `design-matrix-failed`". | spec/functional/FR-165-matrix-design-column.md:53-57 |
