---
id: FR-071
title: "Typed Properties table and SysML fence mapping to FieldDecl"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
---

# FR-071: Typed Properties table and SysML fence mapping to FieldDecl

## Description

The mapping contract SHALL define how a `## Properties` section authored as a
fixed-column table, or as one ```` ```sysml ```` fence, maps to a semantic-core
`FieldDecl[]`, so that both forms extract to identical declarations and an
artifact never carries both.

## Rationale

Ticket #293 mapping (a) and (b): the typed table is the skeleton default; the
SysML v2 textual subset is the alternate representation of the same
declarations, never a second authority.

## Behavior

- The typed table SHALL have exactly the header `Field | Type | Multiplicity | Constraints`, in that order, and a table under `## Properties` with any other header is a legacy form (FR-074).
- Each table row SHALL map to one `FieldDecl` with `name` = `Field` (an `Identifier`), `type.target` from `Type`, `type.multiplicity` from `Multiplicity`, and `constraints` from `Constraints`.
- The `Type` cell SHALL resolve, in order, to a `KernelScalar` name, an enumeration declared in the same bundle, or an object declared in the same bundle (by `id` or title).
- If a `Type` token resolves to none of those, then extraction SHALL record an advisory finding with locus and leave the field's target unresolved rather than lowering it as a string.
- A `Type` cell of the form `Decimal(p,s)` SHALL map to `type.decimal { precision: p, scale: s }`.
- A `Type` cell with a trailing ` [unit]` (for example `Duration [ms]`) SHALL map the bracketed symbol to `type.unit`.
- The `Multiplicity` cell SHALL accept `1`, `0..1`, `0..*`, `1..*`, `n..m` (integers, `m >= n`), each optionally followed by the flags `ordered`, `unique`, or both, with an empty cell meaning `1`.
- The `Constraints` cell SHALL be a comma-separated list of `keyword: value` items using the IR v1.1 closed keywords (`min`, `max`, `exclusiveMin`, `exclusiveMax`, `pattern`, `minLength`, `maxLength`, `enumValues`, `nonEmpty`, `unique`, `format`), plus the markers `identity` and `nullable` mapping to `FieldDecl.identity`/`nullable`.
- If a `Constraints` item uses a keyword outside that set, then extraction SHALL fail with locus.
- A ```` ```sysml ```` fence under `## Properties` SHALL map to the same `FieldDecl[]` through the SysML v2 textual subset: `attribute <name> : <Type>[<mult>];` → a field, `ref item <name> : <Type>[<mult>];` → a field whose target is an object, `item <name> : <Type>;` → a composite-owned field, `:> <Type>` → a `references` relation target.
- If a fence line uses a construct outside that subset, then extraction SHALL fail with locus.
- If an artifact carries both a typed table and a `sysml` fence under `## Properties`, then validation SHALL fail at the second form's locus.
- Table-authored and fence-authored artifacts with equal content SHALL extract to byte-identical normalized `FieldDecl[]`.
- The mapping SHALL record, per artifact, whether the authored authority was the table or the fence, so a renderer regenerates the other form only as a derived view.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-071-CON-1 | Quire SHALL extract the fence with its source span and recognise only the SysML subset lines; expression semantics stay with `agent-ix/quire-contract-ir#52`. | Boundary | Extraction-only inspection |
| FR-071-CON-2 | Quoin SHALL publish the mapping as golden fixtures (table form, fence form, expected `FieldDecl[]`) that `agent-ix/quire-rs#388` consumes unchanged. | Integrity | Fixture provenance |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-071-AC-1 | The config-service FR-006 rows, rewritten as a typed table, extract to the expected `FieldDecl[]` fixture, which validates against the semantic-core `FieldDecl.json`. | Test |
| FR-071-AC-2 | The equivalent `sysml` fence extracts to a byte-identical normalized `FieldDecl[]`. | Test |
| FR-071-AC-3 | An artifact with both forms fails at the second form's locus. | Test |
| FR-071-AC-4 | `Type` tokens `UUID`, `Decimal(10,2)`, `Duration [ms]`, `ConfigOverlay`, and `Status` resolve as kernel scalar, decimal policy, unit, object, and enumeration respectively; `Mystery` yields an advisory finding with locus. | Test |
| FR-071-AC-5 | Multiplicity cells `1`, `0..1`, `1..* ordered unique`, `2..5`, and empty map to the expected `Multiplicity` objects; `5..2` is an error. | Test |
| FR-071-AC-6 | Constraint cells `min: 1, maxLength: 64, identity` map to the expected constraints and flags; `mnimum: 1` is an error with locus. | Test |
| FR-071-AC-7 | A fence line outside the SysML subset (`part def`) is an error with locus. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), semantic-core `FieldDecl`/`TypeRef` (`agent-ix/filament-core-data` FR-031), IR v1.1 constraints (FR-029)
- **Downstream**: [FR-072](./FR-072-invariants-and-operations-mapping.md), [FR-074](./FR-074-legacy-authoring-forms.md), `agent-ix/quire-rs#388`, module tickets
