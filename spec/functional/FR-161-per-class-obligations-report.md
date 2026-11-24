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
  A `design-trace` entry MAY declare `relation`, the `name` of one design relation in the join. The entry then reads only the
  requirement's rows in that relation, by the rule below for a named relation: `unmet` if its row is `uncovered`, `not-available` if the
  requirement has no row in it, else `met`. A policy that wants the design-trace of a requirement kind to mean one relation (for example the
  realization relation of `agent-ix/spec-objects-architecture` FR-010) pins it this way; a policy that wants it for both `FR` and `NFR` lists two
  `design-trace` entries with distinct `id`s, each pinned, because one `relation` names one `from` archetype. A `design-trace` entry
  with no `relation` keeps the reading above and reads every design relation. A `design-trace` row that is `unmet` lists in `missing` the `name` of
  each design relation in which the requirement has an `uncovered` row, in join order, as an `evidence` row lists the obligations lacking
  evidence; a pinned entry lists only its own relation. This adds `missing` to an `unmet` `design-trace` row, which carried none before, and
  changes no row that is not `unmet`. A `design-trace` entry that names `element_archetypes` is
  refused: the entry reads `state` only, and an archetype filter is what `detailed-design` is for.
- `evidence` with a catalog `method`: every obligation derived from the requirement has bound **passing** evidence whose run
  `evidence_kind` equals the method's catalog `evidence_kind` (FR-160 join, including its passing-run rule; methods of one kind are
  interchangeable).
- `detailed-design`, `soup-requirements` and `segregation`: each reads the same design-element join, selected by what the entry
  declares. Quoin knows no relation name, no archetype and no meaning for any of the three beyond the reading below; the policy
  author says which relation of the loaded modules, and which linked-element archetypes, carry the activity:
  - `relation`: the `name` of a design relation in the join ([quire-rs FR-086](ix://agent-ix/quire-rs/FR-086)). Required for
    `soup-requirements` and `segregation`; optional for `detailed-design` and `design-trace`, which then read every design relation.
  - `element_archetypes`: archetype names a linked element must have. Required, non-empty, for `detailed-design`; optional for
    `soup-requirements` and `segregation`; not accepted on `design-trace`.

  The requirement's rows in the selected relation are read as `design-trace` reads them, with one addition: a row counts as satisfied
  only when it is `covered` and, if `element_archetypes` is declared, links at least one element whose `archetype` is in the list.
  The status is `unmet` if any selected row is unsatisfied, `not-available` if the requirement has no row in the selected relation
  or, for a `detailed-design` entry that names no `relation`, if the join is absent or empty (a named relation is refused below when the
  join does not declare it, and an empty join declares none), else `met`. A `relation` that no loaded module declares as a design relation (a typo, or a module that is not loaded) is a
  configuration error: the command **SHALL** refuse the policy naming the class, the entry and the relation, and produce no report,
  exactly as it refuses an `evidence` entry naming an undeclared method. A requirement that has no row in a declared relation is not
  a gap and reads `not-available`.

### Effect of design relations a module adds

Every relation a loaded module names in `design_relations` is read by an unpinned `design-trace` entry, so a module that adds a design
relation widens what that entry reads. The relations of `agent-ix/spec-objects-security` FR-030 (SOUP requirements) and
`agent-ix/spec-objects-architecture` FR-016 (segregation) are such additions. Quoin applies one rule to them and adds none of its own. It
reads the join rows as the engine emitted them and cannot see which requirements a relation's scope selected:

- An added relation puts a requirement in the `design-trace` reading only when the join has a row for it in that relation. The engine decides
  that from the relation's declared population, the `from`, `where`, `when_any` and `exclude` of the relation
  ([quire-rs FR-058](ix://agent-ix/quire-rs/FR-058)), not quoin. A requirement with no row in the relation reads, unpinned, exactly as it read over
  the same join without that relation's rows.
- A requirement with an `uncovered` row in the added relation (the engine found no stating edge) has an unpinned `design-trace` of `unmet`, where it
  was `met` over realization rows alone, and the row lists that relation in `missing`. This is the declared consequence of loading the module,
  not a change in quoin: a policy that does not want the activity folded into `design-trace` pins `relation` as above.
- `quire matrix --strict` ([quire-cli FR-043](ix://agent-ix/quire-cli/FR-043)) fails on a live requirement with an `uncovered` block, one block per
  relation. Quoin adds no scope, filter or exemption. quire-cli FR-043 names the `from`, `where` and `exclude` of a relation as what selects the
  requirements it covers and does not name `when_any`; that wording gap is upstream in quire-cli and is not corrected here.
- The `quoin matrix` design column (FR-165) carries each added relation's block verbatim with its `check` token, with no quoin code change. The dossier
  reads the same join, and [FR-193](./FR-193-dossier-architecture-content.md) states how a dossier design gap names the relation whose block is
  `uncovered`, and [FR-194](./FR-194-dossier-trace-matrix-content.md) states the same rule for the trace matrix; the code step changes both dossier generators. Output over a corpus whose modules add no design relation, or
  whose requirements have no row in an added relation, is unchanged.

Which activities a class demands is the policy author's data. The five kinds are the only closed vocabulary, and a new kind is a quoin
change. The report is per requirement: a part-level reading (a unit-level design element per part, a segregation rationale on a child
part) is expressed by the module's design relation and read here only as the requirement's own rows.

### Status

Each (requirement, obligation entry) row has a status: `met`, `unmet` (the row lists the obligations lacking evidence for `evidence`, and the uncovered relations for `design-trace`),
or `not-available` when the input the kind reads is absent. For `design-trace`: a join row it reads with `state: covered` is `met`; a join row it reads
with `state: uncovered` is `unmet`; an absent or empty join (`{"relations": []}`), or a requirement with no row in any design
relation (for an entry pinning `relation`, no row in that relation), is `not-available`, because a requirement no design relation obliges has not been asked for a design element, so its
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
| FR-161-AC-12 | A `detailed-design` entry with no `element_archetypes`, a `soup-requirements` or `segregation` entry with no `relation`, a relation-backed entry naming a method, a blank relation or archetype, a `design-trace` entry naming `element_archetypes` or a blank `relation`, or an `evidence` entry naming a relation or archetypes is refused naming the cause. A `design-trace` entry naming a non-blank `relation` that a loaded module declares is accepted. | Test |
| FR-161-AC-13 | With `gate: true` the command exits `Partial` (exit 1) when a row is `unmet`, with the report unchanged, and 0 when every row is `met` or `not-available`; without `gate`, or with `gate: false`, an `unmet` row leaves the exit status unchanged. | Test |
| FR-161-AC-14 | A `relation` that no loaded module declares as a design relation is refused naming the class, the entry and the relation, with no report and the same exit status as an undeclared method; the same name declared by a loaded module is not refused. | Test |
| FR-161-AC-15 | A `design-trace` entry naming `relation: R` reads only the requirement's rows in `R`: `covered` is `met`, `uncovered` is `unmet`, no row in `R` is `not-available`, and an `uncovered` row in another relation does not change the status; an `unmet` row lists `R` in `missing`. An `R` no loaded module declares as a design relation is refused as in AC-14. | Test |
| FR-161-AC-16 | Over one corpus, requirement X has a `covered` row in a realization relation only, and requirement Y likewise. Adding to the join an `uncovered` row of a further relation S for X and none for Y turns the unpinned `design-trace` of X from `met` to `unmet` with `missing` listing S, and leaves the status and `missing` of Y equal to those over the join without S. | Test |
| FR-161-AC-17 | A policy naming `design-trace` (unpinned), a `soup-requirements` entry with `relation: S` and a `segregation` entry with `relation: G`, S and G each declared by a loaded module, is not refused and reports one row per entry for a requirement; each row is `met` when the rows it reads are `covered`, `unmet` when one is `uncovered`, and `not-available` when the requirement has no row in the relation it reads. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-161-CON-1 | The report SHALL emit no key named or typed as a score, rank, verdict, approval or recommendation. | Design | Inspection |
| FR-161-CON-2 | Quoin SHALL NOT infer a requirement's class, an allocation or a design link. | Design | Test (FR-161-AC-7) |

## Dependencies

- **Upstream**: [FR-160](./FR-160-per-class-evidence-floors.md), [quire-rs FR-086](ix://agent-ix/quire-rs/FR-086) (merged) and [quire-rs FR-090](ix://agent-ix/quire-rs/FR-090)
- **Downstream**: [NFR-034](../non-functional/NFR-034-class-reports-state-and-never-assign.md)
