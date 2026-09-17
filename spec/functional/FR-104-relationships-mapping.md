---
id: FR-104
title: "Relationships typed table mapping to RelationDecl"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-072"
    type: "depends_on"
---

# FR-104: Relationships typed table mapping to RelationDecl

## Description

Quoin SHALL publish the mapping from a `## Relationships` section, authored as
one fixed-column table, to a semantic-core `RelationDecl[]` with a source locus
per row, as golden fixtures that Quire implements, so that a domain
declaration's relationships are declared in its spec artifact and reach the
semantic IR without a second reading of the Markdown.

## Rationale

The domain model is declared in spec artifacts typed by module object types:
Quire extracts it, the extraction frontend lifts it to semantic IR, and Quire
checks clauses against it. FR-071 and FR-072 cover fields, clauses, and
operations; relationships are the remaining declaration kind that semantic-core
`RelationDecl` and the IR `relationships[]` (filament-core-data FR-028) carry.
A declaration's identity is its artifact `id` and its class name is its declared
name, so a relationship target is an artifact `id`, never a title. Verbs,
categories, and composition come from the FR-040 edge registry the loaded
modules declare (`ix://agent-ix/quire-rs/FR-040`), so a relationship's category
is never inferred from its spelling.

## Inputs

- The `## Relationships` section of an object artifact.
- The module manifest `semantic.mappings` list (FR-070) and the artifact's object type.
- The merged `edge_types` registry of the loaded module set (verb → `{ category, inverse? }`) and the object type's `allowed_links` map (verb → target types).
- The bundle's package identity `<org>/<repo>`, its artifact `id`s, and the package identities named in `semantic.imports`.
- The source identity and path that FR-072 uses to build a `SourceLocus`.

## Outputs

- `relations`: `RelationDecl[]` in row order, each validating against the vendored semantic-core `RelationDecl.json`.
- `relationSources`: one `{ name, sourceSpan }` per `relations` element, at the same index, where `name` is the row's `Name` cell and `sourceSpan` is a semantic-core `SourceLocus`.
- `availability.relations`: `{ state, reason?, lossy }`, as FR-072 reports clauses and operations.
- Diagnostics carrying `code`, `severity`, `line`, `section` (`Relationships`), `reason`, and `sourceSpan` of the declaring line.

## Behavior

- Quire SHALL read a `## Relationships` section as a relationship declaration only when `semantic.mappings` names the token `relationships`.
- When `semantic.mappings` does not name `relationships` and a `## Relationships` section holds no table with the header `Name | Verb | Target | Multiplicity`, Quire SHALL treat the section as prose, with no diagnostic and no `availability.relations`.
- If a table with the header `Name | Verb | Target | Multiplicity` sits under `## Relationships` and `semantic.mappings` does not name `relationships`, then Quire SHALL emit the error `semantic.feature-not-extractable` at the header line with `reason` `relationships`.
- The table SHALL have exactly the header `Name | Verb | Target | Multiplicity`, in that order.
- If the table under `## Relationships` omits, reorders, or adds a column, then Quire SHALL emit the error `semantic.feature-not-extractable` at the header line with `reason` `relationships`.
- If the section holds a bullet list, a fence, or a diagram in place of the table, then Quire SHALL emit the error `semantic.feature-not-extractable` at the block's first line with `reason` `relationships`.
- If the section holds a second table, then Quire SHALL emit the error `semantic.duplicate-section` at the second header line.
- Each row SHALL map to one `RelationDecl` and one `relationSources` entry, with backticks around a cell value stripped.
- The `Name` cell SHALL be an `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`).
- If a second row repeats a `Name`, then Quire SHALL emit the error `semantic.duplicate-model-entry` at the second row.
- The `Verb` cell SHALL be a key of the merged `edge_types` registry and a key of the object type's `allowed_links`.
- `RelationDecl.verb` SHALL be the `Verb` cell as authored.
- `RelationDecl.category` SHALL be the `category` of the verb's registry entry.
- `RelationDecl.composite` SHALL be present, and `true` exactly when the verb's registry entry declares `inverse: part_of`.
- A verb that the registry declares only as another entry's `inverse` label SHALL be an unknown verb, because the relationship is declared on the side whose registry entry carries its category.
- The `Target` cell SHALL be either the `id` of an artifact in the bundle, mapped to `ix://<org>/<repo>/<id>` with the bundle's package identity, or an `ix://<org>/<repo>/<id>` identity whose `<org>/<repo>` is the bundle's package or a package in `semantic.imports`, carried verbatim.
- A row MAY target its own artifact.
- The `Multiplicity` cell SHALL use the FR-071 multiplicity grammar (`1`, `0..1`, `1..1`, `0..*`, `1..*`, `n..m` with `m >= n`, and the `ordered`/`unique` flags only on a collection) and map to `RelationDecl.multiplicity`, with an absent upper bound for `*`.
- If a row's `Name` is not an `Identifier`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `name-not-identifier`.
- If a row's `Verb` is not a key of the merged `edge_types` registry, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `unknown-verb`.
- If a row's `Verb` is registered but not listed under the object type's `allowed_links`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `verb-not-allowed`.
- If a row's `Target` is a title, a bare token that is no artifact `id` in the bundle, or an `ix://` identity of a package that is neither the bundle's nor imported, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `target-not-id`.
- If a row's `Multiplicity` cell is empty or does not match the FR-071 multiplicity grammar, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `multiplicity`.
- Every `sourceSpan` SHALL cover the declaring line: `startLine` and `endLine` that line, `startColumn` 1, and `endColumn` one past the line's byte length.
- When every row maps without error, Quire SHALL report `availability.relations` `available` and not lossy.
- If any row, header, or block under the section carries an error, then Quire SHALL report `availability.relations` `unavailable` with reason `entry-errors: lines <lines>`, with no `relations` and no `relationSources`.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-104-CON-1 | Quire SHALL NOT read relationship category or composition from a verb's spelling, a target's name, or the object type's roles; only the registry entry decides them. | Correctness | Fixture inspection |
| FR-104-CON-2 | Quoin SHALL publish the golden fixtures (`relationships.md`, `relationships.expected.json`, `relationships-cases.json`) under `tests/fixtures/semantic-module/mapping/`, each recording the semantic-core version (`0.2.0`) and the registry and bundle context it assumes, for `agent-ix/quire-rs#418` to consume unchanged. | Integrity | Fixture provenance |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-104-AC-1 | `relationships.md` rows `overlay \| references \| FR-005 \| 1..1`, `entries \| contains \| FR-007 \| 0..*`, and `predecessor \| references \| FR-006 \| 0..1` have an expected `RelationDecl[]` in which `references` is `traceability`, not composite, `contains` is `structural` and composite (`inverse: part_of`), targets are `ix://agent-ix/config-service/<id>`, `0..*` has no upper bound, each element validates against `RelationDecl.json`, and each `relationSources` span covers its row line. | Test |
| FR-104-AC-2 | The `verb-unknown` (`belongs_to`) and `verb-inverse-label` (`part_of`) cases have an expected `semantic.invalid-model-cell` with `reason` `unknown-verb` at the row; `verb-not-allowed` (`depends_on`) has `reason` `verb-not-allowed`. | Test |
| FR-104-AC-3 | The `target-title` (`ConfigOverlay`), `target-undeclared-id` (`FR-999`), and `target-qualified-not-imported` cases have an expected `semantic.invalid-model-cell` with `reason` `target-not-id` at the row; `target-qualified-import` carries `ix://agent-ix/spec-artifacts-iso/FR-001` verbatim with no diagnostic. | Test |
| FR-104-AC-4 | The `multiplicity-malformed` (`one`), `multiplicity-inverted` (`5..2`), and `multiplicity-empty` cases have an expected `semantic.invalid-model-cell` with `reason` `multiplicity` at the row. | Test |
| FR-104-AC-5 | The `name-not-identifier` case (`config-overlay`) has an expected `semantic.invalid-model-cell` with `reason` `name-not-identifier`, and the `duplicate-name` case has an expected `semantic.duplicate-model-entry` at the second row with `availability.relations` `unavailable` and reason `entry-errors: lines 21`. | Test |
| FR-104-AC-6 | The `column-missing` (`Name \| Verb \| Target`) and `column-extra` (`… \| Notes`) cases have an expected `semantic.feature-not-extractable` at the header line with `section` `Relationships` and `reason` `relationships`; `column-missing` has `availability.relations` `unavailable`. | Test |
| FR-104-AC-7 | The `bullet-list` case has an expected `semantic.feature-not-extractable` at the list's first line with `section` `Relationships`, and the `second-table` case has an expected `semantic.duplicate-section` at the second header. | Test |
| FR-104-AC-8 | Under `mappings: [typed-table]`, the `mapping-not-declared` case (a conforming table) has an expected `semantic.feature-not-extractable` at the header, and the `prose-without-mapping` case (a bullet list) has no diagnostic and no `availability.relations`. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-071](./FR-071-typed-properties-mapping.md) (multiplicity grammar), [FR-072](./FR-072-invariants-and-operations-mapping.md) (`SourceLocus` and availability), semantic-core `RelationDecl`/`Multiplicity`/`SourceLocus` `0.2.0` (`agent-ix/filament-core-data` FR-028, FR-031), the FR-040 edge registry (`ix://agent-ix/quire-rs/FR-040`), quire-rs FR-075 diagnostic shape
- **Downstream**: `agent-ix/quire-rs#418`, `agent-ix/filament-core-data` FR-094 (`RelationDecl` lowering)
