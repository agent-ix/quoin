---
id: FR-161
title: "Per-class obligations report: the activities and evidence a software item's class demands"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-160"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-architecture/FR-012"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-090"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-084"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-086"
    type: "depends_on"
---

# FR-161: Per-class obligations report: the activities and evidence a software item's class demands

Tracked by Linear PLAT-1526 (implemented by PLAT-1580).

## Description

`quoin class obligations <scope> --class-policy <file> [--format markdown|json]` **SHALL** report, for each software item the Quire
payload provides, which of the activities its class demands are met, unmet or not available. A higher software safety class demands
activities of the item itself, such as a trace from its requirements to the design elements that realise them and unit-level
verification evidence ([FR-160](./FR-160-per-class-evidence-floors.md) floors apply to each obligation).

### Inputs, all existing payload keys

- **Items and their requirements.** The class policy names, as `item_relation`, a relation of the Quire relation report
  ([quire-rs FR-084](ix://agent-ix/quire-rs/FR-084)) whose rows are the items and whose `linked` entries are the requirements
  allocated to them. The module declares the relation; quoin holds no verb and no archetype. An item is each row `document` of that
  relation, whatever its archetype, so a system part gets rows exactly when the relation provides it; otherwise it is absent from the
  report.
- **Item class.** The `carried` object on the row `document` ([quire-rs FR-090](ix://agent-ix/quire-rs/FR-090)) under the policy's
  `class_field`, validated against the run's `carried_vocabulary` as FR-160 does. An item without it is listed as unassigned with no
  rows. The class is not derived from the item's requirements or owner.
- **Requirement class and obligations.** The `requirements[]` entries and obligation records of the coverage matrix, joined as FR-160
  joins them; the requirement class does not select an item's rows.
- **Design trace.** The design-element join of [quire-rs FR-086](ix://agent-ix/quire-rs/FR-086): one row per requirement with
  `state: covered` or `uncovered`, `linked` design elements and, per element, `realized_criteria`.

### Declared as data

The policy's per-class `obligations` list holds entries with an `id` and one of two kinds quoin knows:

- `design-trace`: every requirement allocated to the item has an FR-086 join row with `state: covered`.
- `item-evidence` with a catalog `method`: every obligation derived from a requirement allocated to the item has bound evidence
  whose run `evidence_kind` equals the method's catalog `evidence_kind` (FR-160 join; methods of one kind are interchangeable).

Which activities a class demands is the policy author's data. The two kinds are the only closed vocabulary, and a new kind is a
quoin change.

### Status

Each (item, obligation) row has a status: `met` when the activity holds for every requirement or obligation it covers; `unmet` when it
fails for at least one, the row listing them; `not-available` when the input the kind reads is absent, for example no FR-086 join
in the payload or no `item_relation` row for the item. `not-available` names the missing input and is neither met nor unmet. The
command reports; it records nothing, and the absence of `unmet` rows is not an approval of the item.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-161-AC-1 | With a policy giving `class-3` the obligations `design-trace` and `item-evidence` (method `unit-testing`), an item of `class-3` whose requirements all have a `covered` join row and unit-testing-kind evidence reports both rows `met`. | Test |
| FR-161-AC-2 | The same item with one requirement whose join row is `uncovered` reports `design-trace` `unmet` listing that requirement; one obligation lacking evidence of the method's kind reports `item-evidence` `unmet` listing that obligation. | Test |
| FR-161-AC-3 | A `class-1` item with a policy giving `class-1` no obligations has no rows; an item of a class the policy does not name has no rows and is listed under `unconfigured`. | Test |
| FR-161-AC-4 | When the payload carries no FR-086 join, every `design-trace` row is `not-available` naming the missing join, and `item-evidence` rows are unaffected; an item with no `item_relation` row is absent from the report. | Test |
| FR-161-AC-5 | An item whose row `document` carries no class is listed as unassigned with no rows. | Test |
| FR-161-AC-6 | `--format json` emits one object whose facts equal the Markdown rows, with no class key for an item without one, and the two formats agree row for row. | Test |
| FR-161-AC-7 | An item's rows come from the item's own class: an item of `class-1` allocated a requirement stating `class-3` reports the `class-1` obligations. | Test |
| FR-161-AC-8 | A policy `obligations` entry with a kind other than `design-trace` or `item-evidence`, an `item-evidence` entry with no method or an undeclared method, or a duplicate `id` within a class is refused naming the cause. | Test |
| FR-161-AC-9 | The command writes no file, record or class value, and two runs over the same inputs print identical output. | Test |
| FR-161-AC-10 | A system part and an item part both returned by the `item_relation` each get rows from their own class. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-161-CON-1 | The report SHALL emit no key named or typed as a score, rank, verdict, approval or recommendation. | Design | Inspection |
| FR-161-CON-2 | Quoin SHALL NOT infer an item's class, an allocation or a design link. | Design | Test (FR-161-AC-7) |

## Dependencies

- **Upstream**: [FR-160](./FR-160-per-class-evidence-floors.md), quire-rs FR-084, FR-086 (quire-rs#668, merging first) and FR-090, spec-objects-architecture FR-012
- **Downstream**: [NFR-034](../non-functional/NFR-034-class-reports-state-and-never-assign.md)
