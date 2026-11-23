---
id: FR-161
title: "Per-class obligations report: the activities and evidence a requirement's software safety class demands"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-160"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-090"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-086"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-iso/FR-009"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-091"
    type: "references"
---

# FR-161: Per-class obligations report: the activities and evidence a requirement's software safety class demands

Tracked by Linear PLAT-1526 (implemented by PLAT-1580).

## Description

`quoin class obligations <scope> --class-policy <file> [--format markdown|json]` **SHALL** report, for each requirement the Quire
coverage payload lists, which of the activities its software safety class demands are met, unmet or not available. A higher class
demands activities beyond the per-obligation floors of [FR-160](./FR-160-per-class-evidence-floors.md), such as a trace from the
requirement to the design elements that realise it and unit-level verification evidence.

The report is per **requirement**, using the class the requirement carries. That class has a named person and a rationale
([spec-artifacts-iso FR-009](ix://agent-ix/spec-artifacts-iso/FR-009)). The class a person assigns to a software part is not read here;
it is used only by the Quire field-consistency check ([quire-rs FR-091](ix://agent-ix/quire-rs/FR-091)), and quoin makes no
item-level report, holds no allocation and no part rows.

### Inputs, all existing payload keys

- **Requirements and their class.** The `requirements[]` entries of the coverage matrix and the `carried` object on each
  ([quire-rs FR-090](ix://agent-ix/quire-rs/FR-090)) under the policy's `class_field`, validated against the top-level
  `carried_vocabulary` as FR-160 does. A requirement without the class is listed as unassigned with no rows.
- **Obligations.** The obligation records of the coverage matrix, joined to their requirement entry as FR-160 joins them.
- **Design trace.** The design-element join of [quire-rs FR-086](ix://agent-ix/quire-rs/FR-086): one row per requirement a design
  relation obliges, with `state: covered` or `uncovered`, `linked` design elements and, per resolved element, `realized_criteria`. A
  corpus with no `design_relations` yields `{"relations": []}` (FR-086-AC-9).

### Declared as data

The policy's per-class `obligations` list holds entries with an `id` and one of five kinds quoin knows:

- `design-trace`: the requirement has an FR-086 join row with `state: covered`. FR-086 emits one row per requirement per design
  relation, so a requirement can have several rows; the precedence across them is `unmet` if any row is `uncovered`, else
  `not-available` if the join is absent or empty or the requirement has no row, else `met`.
- `evidence` with a catalog `method`: every obligation derived from the requirement has bound **passing** evidence whose run
  `evidence_kind` equals the method's catalog `evidence_kind` (FR-160 join, including its passing-run rule; methods of one kind are
  interchangeable).
- `detailed-design`, `soup-requirements` and `segregation`: each reads the same design-element join, selected by what the entry
  declares. Quoin knows no relation name, no archetype and no meaning for any of the three beyond the reading below; the policy
  author says which relation of the loaded modules, and which linked-element archetypes, carry the activity:
  - `relation`: the `name` of a design relation in the join ([quire-rs FR-086](ix://agent-ix/quire-rs/FR-086)). Required for
    `soup-requirements` and `segregation`; optional for `detailed-design`, which then reads every design relation.
  - `element_archetypes`: archetype names a linked element must have. Required, non-empty, for `detailed-design`; optional for the
    other two.

  The requirement's rows in the selected relation are read as `design-trace` reads them, with one addition: a row counts as satisfied
  only when it is `covered` and, if `element_archetypes` is declared, links at least one element whose `archetype` is in the list.
  The status is `unmet` if any selected row is unsatisfied, `not-available` if the join is absent or empty or
  the requirement has no row in the selected relation, else `met`. A `relation` that no loaded module declares as a design relation (a typo, or a module that is not loaded) is a
  configuration error: the command **SHALL** refuse the policy naming the class, the entry and the relation, and produce no report,
  exactly as it refuses an `evidence` entry naming an undeclared method. A requirement that has no row in a declared relation is not
  a gap and reads `not-available`.

Which activities a class demands is the policy author's data. The five kinds are the only closed vocabulary, and a new kind is a quoin
change. The report is per requirement: a part-level reading (a unit-level design element per part, a segregation rationale on a child
part) is expressed by the module's design relation and read here only as the requirement's own rows.

### Status

Each (requirement, obligation entry) row has a status: `met`, `unmet` (the row lists the obligations lacking evidence, for `evidence`),
or `not-available` when the input the kind reads is absent. For `design-trace`: a join row with `state: covered` is `met`; a join row
with `state: uncovered` is `unmet`; an absent or empty join (`{"relations": []}`), or a requirement with no row in any design
relation, is `not-available`, because a requirement no design relation obliges has not been asked for a design element, so its
absence is not a gap. `not-available` names the missing input. The command reports; it records nothing, and the absence of `unmet`
rows is not an approval of the requirement.

### Unmet activities can gate

When the policy file sets `gate: true` ([FR-160](./FR-160-per-class-evidence-floors.md)), `quoin class obligations` **SHALL** exit `Partial`
(exit 1) when any row is `unmet`, and **SHALL NOT** change its report. A `not-available` row never gates, and neither does a requirement listed under `unassigned` or
`unconfigured`. Without `gate`, or with `gate: false`, the command exits as before.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-161-AC-1 | With a policy giving `class-3` the obligations `design-trace` and `evidence` (method `unit-testing`), a `class-3` requirement with a `covered` join row and unit-testing-kind evidence on all its obligations reports both rows `met`. | Test |
| FR-161-AC-2 | The same requirement with a join row `uncovered` reports `design-trace` `unmet`; with one obligation lacking evidence of the method's kind it reports `evidence` `unmet` listing that obligation. | Test |
| FR-161-AC-3 | A `class-1` requirement with a policy giving `class-1` no obligations has no rows; a requirement of a class the policy does not name has no rows and is listed under `unconfigured`. | Test |
| FR-161-AC-4 | With an absent FR-086 join or `{"relations": []}`, every `design-trace` row is `not-available` naming the missing join; a requirement with no row in any design relation while the join is non-empty is `not-available`; a requirement whose row is `uncovered` is `unmet`; `evidence` rows are unaffected.  A requirement with one `covered` row and one `uncovered` row in two design relations is `unmet`. | Test |
| FR-161-AC-5 | A requirement whose `requirements[]` entry carries no class is listed as unassigned with no rows. | Test |
| FR-161-AC-6 | `--format json` emits one object whose facts equal the Markdown rows, with no class key for an unassigned requirement, and the two formats agree row for row. | Test |
| FR-161-AC-7 | A requirement's rows come from its own carried class only: the same requirement allocated to parts of other classes reports the same rows. | Test |
| FR-161-AC-8 | A policy `obligations` entry with a kind other than the five above, an `evidence` entry with no method or an undeclared method, or a duplicate `id` within a class is refused naming the cause. | Test |
| FR-161-AC-9 | The command writes no file, record or class value, and two runs over the same inputs print identical output. | Test |
| FR-161-AC-10 | With a `detailed-design` entry declaring `element_archetypes: [unit]`, a requirement whose `covered` row links an element of archetype `unit` reports `met`; one whose `covered` row links only an element of another archetype reports `unmet`; one whose row is `uncovered` reports `unmet`; one with no row reports `not-available`. | Test |
| FR-161-AC-11 | With a `soup-requirements` or `segregation` entry naming `relation: R` that a loaded module declares, a requirement with a `covered` row in `R` reports `met`, one with an `uncovered` row in `R` reports `unmet`, a row in another relation is not read, and one with no row in `R` reports `not-available`. | Test |
| FR-161-AC-12 | A `detailed-design` entry with no `element_archetypes`, a `soup-requirements` or `segregation` entry with no `relation`, a relation-backed entry naming a method, a blank relation or archetype, or a `design-trace` or `evidence` entry naming a relation or archetypes is refused naming the cause. | Test |
| FR-161-AC-13 | With `gate: true` the command exits `Partial` (exit 1) when a row is `unmet`, with the report unchanged, and 0 when every row is `met` or `not-available`; without `gate`, or with `gate: false`, an `unmet` row leaves the exit status unchanged. | Test |
| FR-161-AC-14 | A `relation` that no loaded module declares as a design relation is refused naming the class, the entry and the relation, with no report and the same exit status as an undeclared method; the same name declared by a loaded module is not refused. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-161-CON-1 | The report SHALL emit no key named or typed as a score, rank, verdict, approval or recommendation. | Design | Inspection |
| FR-161-CON-2 | Quoin SHALL NOT infer a requirement's class, an allocation or a design link. | Design | Test (FR-161-AC-7) |

## Dependencies

- **Upstream**: [FR-160](./FR-160-per-class-evidence-floors.md), [quire-rs FR-086](ix://agent-ix/quire-rs/FR-086) (merged) and [quire-rs FR-090](ix://agent-ix/quire-rs/FR-090)
- **Downstream**: [NFR-034](../non-functional/NFR-034-class-reports-state-and-never-assign.md)
