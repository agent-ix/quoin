---
id: FR-162
title: "Design column in the evidence-backed test matrix"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-092"
    type: "requires"
    cardinality: "1:1"
  - target: "ix://agent-ix/quire-cli/FR-042"
    type: "references"
---

# FR-162: Design column in the evidence-backed test matrix

## Description

`matrix.build` ([FR-115](./FR-115-evidence-backed-test-matrix.md)) joins the quire-rs coverage matrix
with the evidence store and the audit report, so a reader sees for each criterion whether a passing,
current run backs it. A requirement also needs a design element that realizes it, and that fact is
reported by quire-rs ([FR-092](ix://agent-ix/quire-rs/FR-092)), not by quoin. `quoin matrix` SHALL carry
that design column through beside the evidence status, so one view reads requirement, design element,
test and result, with the evidence freshness of FR-115 as the result.

Quoin computes no design fact. It takes `design` from the `coverage` entries as quire-rs emits them, adds
no join, no archetype name and no verb, and runs no second walk. The design axis and the evidence axis
are independent: a design block never changes an `evidence_status` or a `static_status`, and evidence
never changes a design block. `matrix.build` adds no freshness comparison of its own for design
(FR-115): whether a design element is current is not asked here.

## Inputs

`coverage` is the quire-rs `design_matrix` payload (FR-092), the FR-115 shape plus, on entries a design
relation obliges, `id`, `repo`, a `design` array, per-criterion `design_elements`, and entries with
`criteria: []`. An entry with no `design` key means no design relation obliges that document.
`quoin matrix` obtains it through a `quire.design_matrix` operation in `quoin-core`, registered beside
`quire.coverage` (which is unchanged, FR-092-CON-2), over the working tree's single corpus member. When
no loaded module declares `design_relations` the payload, and so the output, is FR-115's. Cross-repository
design elements are outside this command: they appear as `dangling` references, as quire-rs reports them.

## Outputs

Each `requirements[]` entry of `MatrixOutput` carries `design`, the entry's `design` array from the
`coverage` document that holds the requirement's criteria, verbatim and in quire-rs order, and absent
when that entry has none. When one document holds criteria of several requirement ids
(FR-115 grouping), each of those requirements carries the document's `design`. Each criterion carries
`design_elements`, the `{id, repo}` references quire-rs emits on the criterion, verbatim; the elements
they name are in `design`. A requirement entry with `criteria: []` is grouped under the entry's document
`id` (quire-rs FR-092), ordered by requirement id among the others, and appears with empty `criteria`
and its `design`. When no criterion exists in any entry, `reason` is present as FR-115 states, and
`requirements[]` holds only those entries (amending FR-115's empty `requirements`).

### Gap

A requirement with an `uncovered` block is a design gap. It is shown as a gap in every render, naming
the block's `check` token, whether or not it has criteria. A gap does not change `evidence_status`.

### Rendering

`quoin matrix` markdown gains a `Design` column between `Criterion` and `Method`, listing each
`design_elements` entry, resolved against `design`, as `id (kind)` (`id` when it has no `kind`) and `no design element` for a criterion under an `uncovered`
block, and an empty cell for an entry with no `design` key. A requirement with no criteria prints its
`##` heading and a line naming the gap. `--json` carries `design` and `design_elements` as above in
the canonical `MatrixOutput`. `quoin matrix` has no `--strict` flag (FR-115-AC-11); the design gate is
`quire matrix --strict` ([quire-cli FR-042](ix://agent-ix/quire-cli/FR-042)), and quoin adds no third
gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-162-AC-1 | Given a `coverage` with a requirement carrying a `covered` block and another carrying an `uncovered` block, `matrix.build` returns each requirement with the `design` array verbatim as supplied, and `design_elements` per criterion verbatim as supplied. | Test |
| FR-162-AC-2 | A supplied criterion `design_elements` list, empty or not, reaches the output unchanged; `matrix.build` does not recompute it from `realized_criteria`. | Test |
| FR-162-AC-3 | A requirement whose `design` block is `uncovered` renders in markdown with `no design element` in the `Design` cell and its `check` token, and its `evidence_status` is the one FR-115 computes for the same criterion without the `design` key. | Test |
| FR-162-AC-4 | A requirement with no `design` key has an empty `Design` cell and no `design` key in `--json`. | Test |
| FR-162-AC-5 | A requirement with `criteria: []` and an `uncovered` block appears in `requirements[]` and in markdown with a gap line, and a `coverage` holding only such entries yields the FR-115 `reason`, those entries in `requirements[]` and exit 0. | Test |
| FR-162-AC-6 | Over a fixture store with one `bound`, one `stale` and one `undischarged` criterion, the evidence statuses equal those of the same run without `design` in `coverage`, and two runs over byte-identical input give byte-identical canonical output. | Test |
| FR-162-AC-7 | The `Design` column sits between `Criterion` and `Method`, and `--json` names each element's `kind`. | Test |
| FR-162-AC-8 | A `coverage` with no `design` on any entry yields output equal to FR-115's. | Test |
| FR-162-AC-9 | `matrix.build` source holds no archetype name, verb or design join, and `quoin matrix` has no `--strict` flag. | Inspection |
| FR-162-AC-10 | `quoin matrix` leaves `spec/` byte-identical in a fixture repository with `design_relations` declared. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-162-CON-1 | `matrix.build` SHALL leave design blocks and `design_elements` as quire-rs supplies them, neither computing, filtering nor reordering any. | Architecture | Inspection |

## Dependencies

- **Upstream**: [FR-115](./FR-115-evidence-backed-test-matrix.md); quire-rs [FR-092](ix://agent-ix/quire-rs/FR-092), [FR-086](ix://agent-ix/quire-rs/FR-086); quire-cli [FR-042](ix://agent-ix/quire-cli/FR-042)
- **Downstream**: none
