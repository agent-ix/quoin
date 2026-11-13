---
id: FR-165
title: "Design column in the evidence-backed test matrix"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-092"
    type: "requires"
    cardinality: "1:1"
  - target: "ix://agent-ix/quire-rs/FR-086"
    type: "references"
  - target: "ix://agent-ix/quire-cli/FR-043"
    type: "references"
---

# FR-165: Design column in the evidence-backed test matrix

## Description

`matrix.build` ([FR-115](./FR-115-evidence-backed-test-matrix.md)) joins the quire-rs coverage matrix
with the evidence store and the audit report, so a reader sees for each criterion whether a passing,
current run backs it. A requirement also needs a design element that realizes it, and that fact is
reported by quire-rs ([FR-092](ix://agent-ix/quire-rs/FR-092)), not by quoin. `quoin matrix` SHALL carry
that design column through beside the evidence status, so one view reads requirement, design element,
test and result, with the evidence freshness of FR-115 as the result.

Quoin computes no design fact. It takes `design` from the `coverage` entries as quire-rs emits them, adds
no join, no archetype name and no realizing verb, and runs no second walk. The design axis and the evidence axis
are independent: a design block never changes an `evidence_status` or a `static_status`, and evidence
never changes a design block. `matrix.build` adds no freshness comparison of its own for design
(FR-115): whether a design element is current is not asked here.

## Inputs

`coverage` is the quire-rs `design_matrix` payload (FR-092), the FR-115 shape plus, on entries a design
relation obliges, `id`, `repo`, `criticality` and, when declared, `carried` (quire-rs FR-090) as for any entry, a `design` array, per-criterion `design_elements`, and entries with
`criteria: []`. An entry with no `design` key means no design relation obliges that document.
`quoin-core` SHALL provide a `quire.design_matrix` operation, registered beside `quire.coverage` with its
own exhaustive `dispatch()` arm, taking `quire.coverage`'s request and its byte ceiling, over the working
tree's single corpus member. `quire.coverage` is unchanged (FR-092-CON-2), and `quoin matrix` SHALL call
`quire.design_matrix` in its place. When no loaded module declares `design_relations` the payload, and so
the output, is FR-115's. Cross-repository design elements are outside this command: they appear as
`dangling` references, as quire-rs reports them.

### Failure

When `quire.design_matrix` fails because a `design_relations` entry names no declared relation
(FR-092-AC-10), the operation SHALL return `CoreErrorCode::Refused` with `context.reason`
`design-matrix-failed` and the quire-rs error text naming the entry, and `quoin matrix` SHALL exit 2 with a
diagnostic naming that cause. Every other failure SHALL give the code `quire.coverage` gives for the same cause, through the same
request and path ceilings, host resolution and `map_error`
(`rust/crates/quoin-core/src/ops/quire/taxonomy.rs`), so `quire.design_matrix` adds no error mapping of its
own. `quoin matrix` SHALL NOT
fall back to the plain `quire.coverage` matrix.

## Outputs

`matrix.build` SHALL carry, on each `requirements[]` entry of `MatrixOutput`, a `design` array, verbatim
and in quire-rs order, absent when no feeding document has one. A requirement id is fed by every `coverage`
document whose criteria group under it (FR-115-AC-8), and by every `criteria: []` entry whose document `id`
equals it. The requirement's `design` is the concatenation of the blocks of its feeding documents, ordered
by `repo` (absent first) then `document`, so a document with criteria of several ids contributes its
blocks to each of them, and an id fed by a criteria-less entry and by a document with criteria carries
both. `matrix.build` SHALL carry on each criterion its `design_elements`, the `{id, repo}` references quire-rs
emits on the criterion, verbatim; the elements they name are resolved by (`repo`, `id`) in the
requirement's `design`. A `criteria: []` entry is grouped under its document `id` (quire-rs FR-092),
ordered by requirement id among the others, and appears with empty `criteria` and its `design`. When no
criterion exists in any entry, `reason` is present as FR-115 states and `requirements[]` holds only those
entries (amending FR-115's empty `requirements`).

### Gap

A requirement with an `uncovered` block is a design gap, whether or not it has criteria. `quoin matrix`
SHALL show it as a gap in markdown by the rendering below, and a gap SHALL NOT change `evidence_status`.

### Rendering

`quoin matrix` SHALL render markdown as follows, using the cell and gap forms of quire-cli
[FR-043](ix://agent-ix/quire-cli/FR-043) so the two views print one payload alike.

- A `Design` column sits between `Criterion` and `Method`. Its cell lists the criterion's resolved
  `design_elements` as `id (kind)` (`id` when the element has no `kind`, and the repo when present), then
  `no design element (<check>)` for each `uncovered` block, then `-> <reference> (dangling)` for each
  `dangling` reference of a `covered` block that has no elements, then `kind mismatch: <id> (<archetype>)`
  for each `kind_mismatch` entry. These markers follow the elements in one cell, so an entry with one
  `covered` and one `uncovered` block prints its elements and the marker together.
- The cell is `n/a` for a requirement with no `design` key, and `(none)` for a criterion with empty
  `design_elements` under a requirement whose blocks are all `covered` and have nothing to append.
- Under the `##` heading, one line `Design gap: no design element (<check>)` per `uncovered` block, so the
  `check` token prints both in the cell and under the heading. A requirement with no criteria prints its
  heading and these lines with no table.
- `--json` carries `design` and `design_elements` as above in the canonical `MatrixOutput`, with each
  element's `kind` where quire-rs supplies one.

`quoin matrix` has no `--strict` flag (FR-115-AC-11); the design gate is `quire matrix --strict`
([quire-cli FR-043](ix://agent-ix/quire-cli/FR-043)), and quoin adds no third gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-165-AC-1 | Given a `coverage` with a requirement carrying a `covered` block and another carrying an `uncovered` block, `matrix.build` returns each requirement with the `design` array verbatim as supplied, and `design_elements` per criterion verbatim as supplied. | Test |
| FR-165-AC-2 | A supplied criterion `design_elements` list, empty or not, reaches the output unchanged; `matrix.build` does not recompute it from `realized_criteria`. | Test |
| FR-165-AC-3 | A requirement whose `design` block is `uncovered` renders in markdown with `no design element (<check>)` in the `Design` cell and a `Design gap: no design element (<check>)` line under its heading. | Test |
| FR-165-AC-4 | A requirement with no `design` key has `n/a` in the `Design` cell and no `design` key in `--json`; a criterion with empty `design_elements` under an all-`covered` requirement has `(none)`. | Test |
| FR-165-AC-5 | A requirement with `criteria: []` and an `uncovered` block appears in `requirements[]` and in markdown with its heading and `Design gap:` line, and a `coverage` holding only such entries yields the FR-115 `reason`, those entries in `requirements[]` and exit 0. | Test |
| FR-165-AC-6 | Over a fixture store with one `bound`, one `stale` and one `undischarged` criterion, the evidence statuses equal those of the same run without `design` in `coverage`. | Test |
| FR-165-AC-7 | The `Design` column sits between `Criterion` and `Method`, and `--json` carries each element's `kind` where the supplied element has one. | Test |
| FR-165-AC-8 | A `coverage` with no `design` on any entry yields output equal to FR-115's. | Test |
| FR-165-AC-9 | `matrix.build` source holds no archetype name, no realizing verb and no design join. | Inspection |
| FR-165-AC-10 | `quoin matrix` leaves `spec/` byte-identical in a fixture repository with `design_relations` declared. | Test |
| FR-165-AC-11 | Two `matrix.build` runs over byte-identical input give byte-identical canonical output. | Test |
| FR-165-AC-12 | `quoin matrix` has no `--strict` flag. | Test |
| FR-165-AC-13 | A requirement with one `covered` and one `uncovered` block prints its elements and `no design element (<check>)` in one cell. | Test |
| FR-165-AC-14 | An element supplied without `kind` renders as its `id` alone. | Test |
| FR-165-AC-15 | A `covered` block with only a dangling reference renders `-> <reference> (dangling)`, and a `kind_mismatch` entry renders `kind mismatch: <id> (<archetype>)`. | Test |
| FR-165-AC-16 | Two documents whose criteria group under one requirement id, and a `criteria: []` entry whose document `id` equals it, give that requirement the blocks of all three, ordered by `repo` then `document`. | Test |
| FR-165-AC-17 | A `design_relations` entry naming no declared relation makes `quire.design_matrix` return `Refused` with `context.reason` `design-matrix-failed`, and `quoin matrix` exits 2 naming the entry and prints no matrix. | Test |
| FR-165-AC-18 | `quire.design_matrix` is in `quoin-core`'s `OPERATIONS` list and dispatched by an exhaustive arm, and `quire.coverage` output is unchanged. | Test |
| FR-165-AC-19 | A `quire.design_matrix` failure other than an undeclared design relation, such as an oversized request, an engine refusal or a missing quire host, gives the error code and exit code `quire.coverage` gives for the same cause, not `design-matrix-failed`, and `quoin matrix` prints no matrix. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-165-CON-1 | `matrix.build` SHALL leave design blocks and `design_elements` as quire-rs supplies them, neither computing, filtering nor reordering any. | Architecture | Inspection |

## Dependencies

- **Upstream**: [FR-115](./FR-115-evidence-backed-test-matrix.md); quire-rs [FR-092](ix://agent-ix/quire-rs/FR-092), [FR-086](ix://agent-ix/quire-rs/FR-086); quire-cli [FR-043](ix://agent-ix/quire-cli/FR-043)
- **Downstream**: none
