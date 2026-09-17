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
one fixed-column table, to a semantic-core `RelationDecl[]` with a name and a
source locus per row, as golden fixtures that Quire implements, so that a domain
declaration's relationships are declared in its spec artifact and reach the
semantic IR without a second reading of the Markdown.

## Rationale

The domain model is declared in spec artifacts typed by module object types:
Quire extracts it, the extraction frontend lifts it to semantic IR, and Quire
checks clauses against it. FR-071 and FR-072 cover fields, clauses, and
operations; relationships are the remaining declaration kind that semantic-core
`RelationDecl` and the IR `relationships[]` (filament-core-data FR-028) carry.
A relationship target names another declaration by its artifact `id`, never by
its title. Verbs, categories, composition, and permitted targets come from the
FR-040 edge registry and object `allowed_links` the loaded modules declare
(`ix://agent-ix/quire-rs/FR-040`), so a relationship's category is never inferred
from its spelling. Every relationship is authored in its forward direction only;
the inverse label is a derived view of that edge (`ix://agent-ix/quire-rs/FR-041`,
quire-rs ADR-0008), so one edge has one declaration site.

The `## Relationships` table is the one authority for domain relationships
between declarations: verb, target, and multiplicity. Frontmatter
`relationships:` stays the artifact-graph and traceability edge surface that
quire-rs FR-040 validates. `RelationDecl` comes from this extraction, not from
lowering frontmatter edges (`agent-ix/filament-core-data#156`).

## Inputs

- The `## Relationships` section of an object artifact, and every other section and the preamble of that artifact.
- The artifact's frontmatter `relationships:` edges.
- The module manifest `semantic.mappings` list (FR-070) and the artifact's object type.
- The relation vocabulary, when the extracting surface supplies one: the merged `edge_types` registry of the loaded module set (verb → `{ category, inverse? }`), the object types' `roles`, and the object type's `allowed_links` map (verb → object type, role, or `*`).
- The bundle's package identity `<org>/<repo>`, and, when the extracting surface supplies a bundle index, its artifact `id`s with their object types; the package identities named in `semantic.imports`.
- The source identity and path that FR-072 uses to build a `SourceLocus`.

## Outputs

- `relations`: `RelationDecl[]` in row order, each validating against the vendored semantic-core `RelationDecl.json`.
- `relationSources`: one `{ name, sourceSpan }` per `relations` element, at the same index, where `name` is the row's `Name` cell and `sourceSpan` is a semantic-core `SourceLocus`.
- `availability.relations`: `{ state, reason?, lossy }`, as FR-072 reports clauses and operations.
- Diagnostics on the quire-rs FR-075 shape: `code`, `severity`, `line`, `section`, `reason`, `sourceSpan` of the declaring line, and a `message`.

## Behavior

### Target design and the 0.2.0 carrier

- Each row SHALL lower to one `RelationDecl` that carries the row's `Name` as `name` and the row's `SourceLocus` as `sourceSpan`, the shape `agent-ix/filament-core-data#155` adds to semantic-core.
- Each row SHALL lower to one `RelationDecl` whose `multiplicity` follows the one default-multiplicity rule that `agent-ix/filament-core-data#155` settles.
- While the published semantic-core is `0.2.0`, whose `RelationDecl` has no `name` and no `sourceSpan`, Quire SHALL carry them in `relationSources` at the index of their `relations` element.
- When `agent-ix/filament-core-data#155` ships, `relationSources` SHALL be removed and the goldens SHALL carry `name` and `sourceSpan` on each `RelationDecl`.
- The `Multiplicity` cell SHALL be required, because semantic-core `0.2.0` lowers an absent multiplicity to `0..1` and `agent-ix/filament-core-data#155` has not yet settled the default; an omitted cell would otherwise encode a bound nobody authored.

### Mapping token and table shape

- Quire SHALL read a `## Relationships` section as a relationship declaration only when `semantic.mappings` names the token `relationships`.
- A relationship-shaped table is a table whose header starts with `Name` and uses only the columns `Name`, `Verb`, `Target`, `Multiplicity`.
- When `semantic.mappings` does not name `relationships` and no table in the artifact is relationship-shaped, Quire SHALL treat a `## Relationships` section as prose, with no diagnostic and no `availability.relations`.
- If `semantic.mappings` does not name `relationships` and a relationship-shaped table sits under any section or in the preamble, then Quire SHALL emit the error `semantic.feature-not-extractable` at the header line with `reason` `relationships` and `section` naming where the table was found (quire-rs FR-075 refusal rule).
- If `semantic.mappings` names `relationships` and a relationship-shaped table sits in the preamble or under a section other than `## Relationships`, then Quire SHALL emit the error `semantic.feature-not-extractable` at the header line with `reason` `relationships` and `section` naming where the table was found; the FR-075 refusal rule holds regardless of the token.
- If `semantic.mappings` names `relationships` and the `## Relationships` section holds no table, list, fence, or diagram, then Quire SHALL emit the warning `semantic.relationships-no-block` at the heading line with `reason` `no-block`, as quire-rs FR-070 emits `semantic.properties-no-block`.
- The table SHALL have exactly the header `Name | Verb | Target | Multiplicity`, in that order.
- If the first block of the `## Relationships` section is a table that omits, reorders, or adds a column, then Quire SHALL emit the error `semantic.feature-not-extractable` at the header line with `reason` `relationships`.
- If the first block of the section is a bullet list, a fence, or a diagram, or a list, fence, or diagram follows the table, then Quire SHALL emit the error `semantic.feature-not-extractable` at that block's first line with `reason` `relationships`.
- Bullet-form and diagram-form `## Relationships` sections in the corpus are out of scope for this requirement. Converting them is owned by the corpus tickets, whose bodies list the artifacts: `agent-ix/config-service#5` (FR-005, FR-006, bullet lists), `agent-ix/catalog-service#6` (FR-017..FR-025, mermaid diagrams), and `agent-ix/config-overlay#4` (no `## Relationships` section).
- If the section holds a second table, then Quire SHALL emit the error `semantic.duplicate-section` at the second header line with `reason` `second-table`.
- If the artifact holds a second `## Relationships` section, then Quire SHALL emit the error `semantic.duplicate-section` at the second heading line with `reason` `second-section`.

### Relation vocabulary and bundle index

- If `semantic.mappings` names `relationships`, the `## Relationships` section holds a table with the exact header, and the extracting surface supplies no relation vocabulary, then Quire SHALL report `availability.relations` `unavailable` with reason `no-relation-vocabulary`, emit no row diagnostic and no `relations`, and emit one advisory `semantic.relationships-no-vocabulary` at the section heading line with `reason` `no-relation-vocabulary`. A missing vocabulary is an explicit state, never a refusal per row.
- The artifact's own `id`, with its frontmatter object type, SHALL count as a bundle artifact whether or not the surface supplies a bundle index.
- A target identity segment SHALL be in the semantic-core `SemanticId` id alphabet: an ASCII letter or digit, then ASCII letters, digits, `.`, `_`, `~`, `:`, or `-`. A `Target` whose id is outside that alphabet, such as a title with spaces, is `target-not-id` whether or not the surface supplies a bundle index, because a title is never a target.
- If the extracting surface supplies no bundle index and a row's `Target` is a bare id in that alphabet other than the artifact's own `id`, or an `ix://<org>/<repo>/<id>` identity of the artifact's own package whose `<id>` is not the artifact's own `id`, then Quire SHALL emit the advisory `semantic.unresolved-target` at the row with `reason` `no-bundle-index` and a message naming the target, skip only the bundle-existence and `allowed_links` checks for that row, and lower the row with the target `ix://<org>/<repo>/<id>` under the artifact's own package, as quire-rs FR-070 maps an unresolved type with a `no-bundle-index` advisory.
- An advisory SHALL NOT make `availability.relations` `unavailable` or lossy.

### Rows

- Each row SHALL map to one `RelationDecl` and one `relationSources` entry, with backticks around a cell value stripped.
- The `Name` cell SHALL be an `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`).
- The `Verb` cell SHALL be a forward key of the merged `edge_types` registry and a key of the object type's `allowed_links`.
- `RelationDecl.verb` SHALL be the `Verb` cell as authored.
- `RelationDecl.category` SHALL be the `category` of the verb's registry entry.
- `RelationDecl.composite` SHALL be present, and `true` exactly when the verb's registry entry declares `inverse: part_of`.
- The `Target` cell SHALL be the `id` of an artifact in the bundle, mapped to `ix://<org>/<repo>/<id>` with the bundle's package identity, or an `ix://<org>/<repo>/<id>` identity carried verbatim.
- An `ix://` target whose `<org>/<repo>` is the bundle's package SHALL name an artifact `id` in the bundle.
- An `ix://` target whose `<org>/<repo>` is not the bundle's package SHALL name a package in `semantic.imports`.
- A bundle target SHALL satisfy the verb's `allowed_links` entry under quire-rs FR-040 `target_satisfies`: the token is `*`, the target's object type, or a role of that object type.
- Quire SHALL skip the `allowed_links` target check for a target in an imported package, as quire-rs FR-040 Tier 2 does.
- A bundle target whose artifact declares no object type SHALL satisfy only the `allowed_links` token `*`. This departs from quire-rs FR-040 Tier 2, which skips the check for an untyped target: a domain relationship names a declaration, so an untyped target (a behavioural requirement, for example) is refused rather than passed.
- A row MAY target its own artifact.
- The `Multiplicity` cell SHALL use the FR-071 multiplicity grammar (`1`, `0..1`, `1..1`, `0..*`, `1..*`, `n..m` with `m >= n`, and the `ordered`/`unique` flags only on a collection) and map to `RelationDecl.multiplicity`, with an absent upper bound for `*`.

### Refusals

- Quire SHALL apply the row checks in this order: `Name` (`name-not-identifier`, then `duplicate-name`), `inverse-verb`, `generalization`, `unknown-verb`, `verb-not-allowed`, the target checks (`target-not-id`, then `target-not-allowed`), `multiplicity`, and `declared-in-frontmatter`.
- Quire SHALL report only the first failing check for a row, so each failing row yields exactly one diagnostic.
- If a row's `Name` is not an `Identifier`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `name-not-identifier`.
- If a second row repeats a `Name`, then Quire SHALL emit `semantic.duplicate-model-entry` at the second row with `reason` `duplicate-name`.
- If a row's `Verb` is registered only as another entry's `inverse` label, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `inverse-verb`.
- The `inverse-verb` message SHALL name the forward verb and the target artifact, which is the artifact that declares the edge.
- If a row's `Verb` is `specializes`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `generalization`, and a message that names the quire-rs FR-075 `generalization` mapping as the place a specialization is declared.
- If a row's `Verb` is neither a forward key nor an inverse label of the merged `edge_types` registry, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `unknown-verb`.
- If a row's `Verb` is registered but not listed under the object type's `allowed_links`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `verb-not-allowed`.
- If a row's `Target` is a title, a bare token that is no artifact `id` in the bundle, an `ix://` identity of the bundle's package that names no bundle artifact, or an `ix://` identity of a package that is neither the bundle's nor imported, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `target-not-id`.
- If a bundle target does not satisfy the verb's `allowed_links` entry, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `target-not-allowed`.
- If a row's `Multiplicity` cell is empty or does not match the FR-071 multiplicity grammar, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `multiplicity`.
- If the artifact's frontmatter `relationships:` declares the same verb and target as a table row, then Quire SHALL emit `semantic.duplicate-model-entry` at the table row with `reason` `declared-in-frontmatter`, as FR-072 refuses a clause declared both inline and externally.

### Diagnostics and availability

- Every diagnostic this requirement emits (`semantic.feature-not-extractable`, `semantic.invalid-model-cell`, `semantic.duplicate-model-entry`, `semantic.duplicate-section`, `semantic.relationships-no-block`, `semantic.relationships-no-vocabulary`, `semantic.unresolved-target`) SHALL carry `section` and `reason`. quire-rs FR-075 defines that shape for `semantic.feature-not-extractable`; this requirement extends it to the other codes.
- `section` SHALL be `Relationships` for a diagnostic inside a `## Relationships` section, `preamble` for the preamble, and the heading text of any other section (quire-rs FR-075).
- Every diagnostic message SHALL name the artifact path, the section, and the feature `relationships` (quire-rs FR-075).
- Every `sourceSpan` SHALL cover the declaring line: `startLine` and `endLine` that line, `startColumn` 1, and `endColumn` one past the line's byte length.
- When `semantic.mappings` names `relationships` and the artifact has no `## Relationships` section, Quire SHALL report `availability.relations` `not_applicable` (quire-rs FR-071 absent section).
- When `semantic.mappings` names `relationships` and the table has a header and no rows, Quire SHALL report `availability.relations` `available`, not lossy, with empty `relations` and `relationSources` (quire-rs FR-070 empty typed table).
- When every row maps without error, Quire SHALL report `availability.relations` `available` and not lossy.
- When `semantic.mappings` names `relationships` and the `## Relationships` section holds no block, Quire SHALL report `availability.relations` `not_applicable`.
- If any row, header, block, or section of the relationships feature carries an error, then Quire SHALL report `availability.relations` `unavailable` with reason `entry-errors: lines <lines>`, with no `relations` and no `relationSources`.
- `<lines>` SHALL be the error lines, sorted ascending, deduplicated, and joined with `, ` (a comma and one space), for example `entry-errors: lines 20, 21`.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-104-CON-1 | Quire SHALL NOT read relationship category or composition from a verb's spelling, a target's name, or the object type's roles; only the registry entry decides them. | Correctness | Fixture inspection |
| FR-104-CON-2 | Quoin SHALL publish the golden fixtures (`relationships.md`, `relationships.expected.json`, `relationships-cases.json`) under `tests/fixtures/semantic-module/mapping/`, each recording the semantic-core version (`0.2.0`), the registry and bundle context it assumes, and the `agent-ix/spec-artifacts-iso` and `agent-ix/spec-objects-business` revisions that context comes from, for `agent-ix/quire-rs#418` to consume unchanged. | Integrity | Fixture provenance |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-104-AC-1 | `relationships.md` rows `overlay \| references \| FR-005 \| 1..1`, `entries \| contains \| FR-007 \| 0..*`, and `predecessor \| references \| FR-006 \| 0..1` have an expected `RelationDecl[]` in which `references` is `traceability`, not composite, `contains` is `structural` and composite (`inverse: part_of`), targets are `ix://agent-ix/config-service/<id>`, `0..*` has no upper bound, each element validates against `RelationDecl.json`, and each `relationSources` entry names its row and spans its row line. | Test |
| FR-104-AC-2 | `verb-unknown` (`belongs_to`) has `reason` `unknown-verb`; `verb-inverse-label` (`composed_by`) has `reason` `inverse-verb` with a message containing `composes` and `FR-005`; `verb-not-allowed` (`depends_on`) has `reason` `verb-not-allowed`; `verb-specializes` has `reason` `generalization`; each is `semantic.invalid-model-cell` at the row. | Test |
| FR-104-AC-3 | `target-title`, `target-undeclared-id`, `target-own-package-undeclared-id` (`ix://agent-ix/config-service/FR-999`), and `target-qualified-not-imported` have `reason` `target-not-id`; `target-not-allowed-object-type` (`references` → nested_entity `FR-007`) and `target-not-allowed-behavioral` (`references` → behavioural `FR-010`) have `reason` `target-not-allowed`; `target-qualified-import` carries `ix://agent-ix/spec-artifacts-iso/FR-001` verbatim with no diagnostic. | Test |
| FR-104-AC-4 | `multiplicity-malformed` (`one`), `multiplicity-inverted` (`5..2`), and `multiplicity-empty` have `semantic.invalid-model-cell` with `reason` `multiplicity` at the row. | Test |
| FR-104-AC-5 | `name-not-identifier` has `reason` `name-not-identifier`; `duplicate-name` has `semantic.duplicate-model-entry` `reason` `duplicate-name` at line 21; `declared-in-frontmatter` has `semantic.duplicate-model-entry` `reason` `declared-in-frontmatter` at the table row. | Test |
| FR-104-AC-6 | `column-missing`, `column-extra`, and `bullet-list` have `semantic.feature-not-extractable` with `section` `Relationships` and `reason` `relationships`; `second-table` has `semantic.duplicate-section` `reason` `second-table` and `second-section` has `reason` `second-section`, each with `availability.relations` `unavailable`. | Test |
| FR-104-AC-7 | Under `mappings: [typed-table]`, `mapping-not-declared` has `semantic.feature-not-extractable` at the header with `section` `Relationships`, `subset-header-unowned-section` (`Name \| Target` under `## Associations`) has it with `section` `Associations`, `preamble-table` has it with `section` `preamble`, and `prose-without-mapping` has no diagnostic and no `availability.relations`. | Test |
| FR-104-AC-8 | `good-and-bad-rows` has no `relations` and `availability.relations` `unavailable` with reason `entry-errors: lines 21`; `two-error-lines` has reason `entry-errors: lines 20, 21`; `header-only` is `available` with empty `relations`; `section-absent` is `not_applicable`; `first-failing-check-only` (a row failing the verb, target, and multiplicity checks) has exactly one diagnostic, `inverse-verb`. | Test |
| FR-104-AC-9 | Under the token, `token-table-other-section` (the exact header under `## Associations`) has `semantic.feature-not-extractable` with `section` `Associations` and `token-preamble-table` has it with `section` `preamble`, each with `availability.relations` `unavailable`; `prose-only-section` has the warning `semantic.relationships-no-block` at the heading with `reason` `no-block` and `availability.relations` `not_applicable`. | Test |
| FR-104-AC-10 | `no-relation-vocabulary` (rows that would fail the verb and multiplicity checks) has exactly one diagnostic, the advisory `semantic.relationships-no-vocabulary` at the heading, no `relations`, and `availability.relations` `unavailable` with reason `no-relation-vocabulary`; `no-bundle-index` lowers both rows under `ix://agent-ix/config-service/`, with the advisory `semantic.unresolved-target` `reason` `no-bundle-index` naming `FR-005` at its row, no diagnostic for the artifact's own `FR-006`, and `availability.relations` `available`, not lossy; `no-bundle-index-title-target` (`Config Overlay`) is still `target-not-id`; `no-bundle-index-own-package-identity` carries `ix://agent-ix/config-service/FR-005` verbatim with the `no-bundle-index` advisory. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-071](./FR-071-typed-properties-mapping.md) (multiplicity grammar), [FR-072](./FR-072-invariants-and-operations-mapping.md) (`SourceLocus`, availability, inline-plus-external refusal), semantic-core `RelationDecl`/`Multiplicity`/`SourceLocus` `0.2.0` (`agent-ix/filament-core-data` FR-028, FR-031), `agent-ix/filament-core-data#155` (`RelationDecl` `name`, `sourceSpan`, default multiplicity), the FR-040 edge registry and `target_satisfies` (`ix://agent-ix/quire-rs/FR-040`), authorable inverse edges (`ix://agent-ix/quire-rs/FR-041`, quire-rs ADR-0008), quire-rs FR-075 diagnostic shape, refusal rule, and `generalization` mapping
- **Downstream**: `agent-ix/quire-rs#418` (PR agent-ix/quire-rs#436), `agent-ix/filament-core-data` FR-094 (`RelationDecl` from extraction, `agent-ix/filament-core-data#156`)
