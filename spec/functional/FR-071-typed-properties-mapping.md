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

Quoin SHALL publish the mapping from a `## Properties` section, authored as a
fixed-column table or as one ```` ```sysml ```` fence, to a semantic-core
`FieldDecl[]`, as golden fixtures that Quire implements, so that both forms
extract to identical declarations and an artifact never carries both.

## Rationale

Ticket #293 mapping (a) and (b): the typed table is the skeleton default; the
SysML v2 textual subset is the alternate representation of the same field
declarations. The subset is deliberately limited to constructs that have a
table equivalent, so the two forms are interchangeable. Quire recognises the
subset at line level (a lexical grammar) and never parses expressions; clause
and expression semantics stay with `agent-ix/quire-contract-ir#52`.

## Behavior

- The typed table SHALL have exactly the header `Field | Type | Multiplicity | Constraints`, in that order, and a table under `## Properties` with any other header is a legacy form (FR-074).
- Each table row SHALL map to one `FieldDecl` with `name` = `Field` (an `Identifier`), `type.target` from `Type`, `type.multiplicity` from `Multiplicity`, and `constraints` and flags from `Constraints`, with backticks around a cell value stripped.
- The `Type` cell SHALL resolve, case-sensitively and in this order, to a `KernelScalar` name, an object or enumeration declared in the same bundle by frontmatter `id`, an object or enumeration declared in the same bundle by exact title, or an export of a module named in `semantic.imports`.
- If a title is shared by two declarations, then extraction SHALL fail naming both.
- An enumeration SHALL be an artifact of the `enumeration` object type whose `## Values` list maps to `EnumValue[]` (`- value — doc`).
- If a `Type` token resolves to none of those, then extraction SHALL record an advisory finding with locus and emit the field with `type.target` set to the placeholder identity `ix://<org>/<repo>/unresolved/<token>`, so the field survives to the IR reader, which reports it as unresolved, and is never lowered as a string.
- A `Type` cell of the form `Decimal(p,s)` SHALL map to `type.decimal { precision: p, scale: s }`.
- A `Type` cell with a trailing ` [unit]` (for example `Duration [ms]`) SHALL map the bracketed symbol to `type.unit`.
- The `Multiplicity` cell SHALL accept `1`, `0..1`, `0..*`, `1..*`, `n..m` (integers, `m >= n`), and, only when the upper bound is absent or greater than 1, the flags `ordered`, `unique`, or both, with an empty cell meaning `1`.
- The `Constraints` cell SHALL be a comma-separated list where each item is `min: <n>`, `max: <n>`, `exclusiveMin: <n>`, `exclusiveMax: <n>`, `minLength: <n>`, `maxLength: <n>`, `pattern: /<regex>/`, `enumValues: <a>|<b>|…`, `format: <ns>:<name>`, the bare words `nonEmpty` and `unique`, or the flags `identity` and `nullable`, mapping to the IR v1.1 constraint models and to `FieldDecl.identity`/`nullable`.
- If a `Constraints` item uses a keyword outside that set, then extraction SHALL fail with locus.
- A ```` ```sysml ```` fence under `## Properties` SHALL map to the same `FieldDecl[]` through two line forms only: `attribute <name> : <Type>[<mult>] { <constraints> }` and `ref item <name> : <Type>[<mult>] { <constraints> }`, where `<Type>`, `<mult>`, and `<constraints>` use the cell grammars above and `ref item` requires an object or import target.
- If a fence line uses any other construct (`item`, `part def`, `:>`, expressions), then extraction SHALL fail with locus.
- If an artifact carries both a typed table and a `sysml` fence under `## Properties`, then validation SHALL fail at the second form's locus.
- Table-authored and fence-authored artifacts with equal content SHALL extract to identical normalized `FieldDecl[]`, where normalized means canonical JSON (sorted keys, no whitespace) with optional properties omitted when absent and the authored-form flag kept outside the array.
- Extraction SHALL apply the semantic-core reader rules (bounds, flags on collections, `decimal` presence, unit applicability, uniqueness by name, identity on `1..1` non-`JsonObject`), reporting each violation at the row or fence-line locus.
- The mapping SHALL record, per artifact, whether the authored authority was the table or the fence, so a renderer regenerates the other form only as a derived view.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-071-CON-1 | Quire SHALL recognise the fence at line level with source spans, treating any expression inside braces as opaque constraint text mapped by the cell grammar only. | Boundary | Extraction-only inspection |
| FR-071-CON-2 | Quoin SHALL publish the golden fixtures (table form, fence form, expected normalized `FieldDecl[]`, expected diagnostics) under `tests/fixtures/semantic-module/` with the semantic-core version they target, for `agent-ix/quire-rs#388` to consume unchanged. | Integrity | Fixture provenance |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-071-AC-1 | Quoin's copy of the config-service FR-006 rows, rewritten as a typed table, has an expected `FieldDecl[]` fixture whose every element validates against the vendored semantic-core `FieldDecl.json`. | Test |
| FR-071-AC-2 | The `sysml` fence fixture with equal content has an expected normalized `FieldDecl[]` identical to the table fixture's. | Test |
| FR-071-AC-3 | The both-forms fixture's expected diagnostic is a failure at the second form's locus. | Test |
| FR-071-AC-4 | `Type` tokens `UUID`, `Decimal(10,2)`, `Duration [ms]`, `ConfigOverlay` (object by title), `Status` (enumeration by id), and an imported export resolve as specified; `Mystery` yields the advisory finding and the placeholder target. | Test |
| FR-071-AC-5 | Multiplicity cells `1`, `0..1`, `1..* ordered unique`, `2..5`, and empty map to the expected `Multiplicity` objects; `5..2` and `1 ordered` are errors. | Test |
| FR-071-AC-6 | Constraint cells `min: 1, maxLength: 64, identity`, `pattern: /^[a-z]+$/`, `enumValues: draft|final`, `nonEmpty`, `format: agent-ix:email` map as specified; `mnimum: 1` is an error with locus. | Test |
| FR-071-AC-7 | Fence lines `item x : Y;`, `part def X;`, and `:> Y` are errors with locus. | Test |
| FR-071-AC-8 | A fixture row violating a semantic-core reader rule (`Decimal` without `(p,s)`) yields the rule's diagnostic at the row locus. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), semantic-core `FieldDecl`/`TypeRef` and reader rules (`agent-ix/filament-core-data` FR-031), IR v1.1 constraints (FR-029)
- **Downstream**: [FR-072](./FR-072-invariants-and-operations-mapping.md), [FR-074](./FR-074-legacy-authoring-forms.md), `agent-ix/quire-rs#388`, module tickets
