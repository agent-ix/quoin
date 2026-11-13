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
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-091"
    type: "references"
---

# FR-161: Per-class obligations report: the activities and evidence a software item's class demands

Tracked by Linear PLAT-1526 (implemented by PLAT-1580).

## Description

[FR-160](./FR-160-per-class-evidence-floors.md) floors apply to each obligation. A higher software safety class also
demands **activities** of the software item itself, such as a trace from its requirements to the design elements that
realise them and unit-level verification evidence. `quoin class obligations <scope> --class-policy <file>
[--format markdown|json]` reports, per software item, which of the activities its class demands are met and which
are not.

### Declared as data

The `--class-policy` file of FR-160 gains an optional `obligations` list per class. Each entry has an `id` and one
of two kinds quoin knows:

- `design-trace`: every requirement allocated to the item has at least one design element linked to it in the design
  trace join (PLAT-1417).
- `item-evidence` with a catalog `method`: every obligation derived from a requirement allocated to the item has bound
  evidence of that method.

Which activities a class demands is the policy author's data; quoin holds no class, no list and no default. The kinds are
the only closed vocabulary, and a new kind is a quoin change.

### Which class, and what is read

An item's class is the `safetyClass.class` the item's own record carries
([spec-objects-architecture FR-012](ix://agent-ix/spec-objects-architecture/FR-012)), read from the Quire payload; it is not
derived from its requirements or its owner. An item with no class has no obligations from the policy and is listed as
unassigned, with the same absence shown by [quire-rs FR-091](ix://agent-ix/quire-rs/spec/functional/FR-091) as a finding.
The allocation of requirements to items comes from the payload's allocation edges.

### Status

Each (item, obligation) row has a status:

- `met`: the activity holds for every requirement or obligation it covers.
- `unmet`: it fails for at least one; the row lists them.
- `not-available`: the input the kind reads is absent, for example no design trace join in the payload. Quoin writes the word
  and the missing input; it does not count the row as met or unmet.

The command reports; it records nothing, and the absence of `unmet` rows is not an approval of the item.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-161-AC-1 | With a policy giving `class-3` the obligations `design-trace` and `item-evidence` (method `unit-test`), an item of `class-3` whose requirements all have a design link and unit-test evidence reports both rows `met`. | Test |
| FR-161-AC-2 | The same item with one requirement lacking a design link reports `design-trace` `unmet` listing that requirement; one obligation lacking unit-test evidence reports `item-evidence` `unmet` listing that obligation. | Test |
| FR-161-AC-3 | A `class-1` item with a policy giving `class-1` no obligations has no rows; an item of a class the policy does not name has no rows and is listed under `unconfigured`. | Test |
| FR-161-AC-4 | When the payload carries no design trace join, every `design-trace` row is `not-available` naming the missing input, and `item-evidence` rows are unaffected. | Test |
| FR-161-AC-5 | An item with no class is listed as unassigned with no rows. | Test |
| FR-161-AC-6 | `--format json` emits one object whose keys are the same facts the Markdown shows, and the two formats agree row for row. | Test |
| FR-161-AC-7 | An item's rows come from the item's own class: an item of `class-1` allocated a requirement stating `class-3` reports the `class-1` obligations. | Test |
| FR-161-AC-8 | A policy `obligations` entry with a kind other than `design-trace` or `item-evidence`, an `item-evidence` entry with no method or an undeclared method, or a duplicate `id` within a class is refused naming the cause. | Test |
| FR-161-AC-9 | The command writes no file, record or class value, and two runs over the same inputs print identical output. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-161-CON-1 | The report SHALL emit no key named or typed as a score, rank, verdict, approval or recommendation. | Design | Inspection |
| FR-161-CON-2 | Quoin SHALL NOT infer an item's class, an allocation or a design link. | Design | Test (FR-161-AC-7) |

## Dependencies

- **Upstream**: [FR-160](./FR-160-per-class-evidence-floors.md), quire-rs FR-090 and FR-091, spec-objects-architecture FR-012
- **Downstream**: none
