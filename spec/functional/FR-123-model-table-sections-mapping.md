---
id: FR-123
title: "Enumeration Values and model-table sections mapping to record keys"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-104"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-121"
    type: "depends_on"
---

# FR-123: Enumeration Values and model-table sections mapping to record keys

## Description

Quoin SHALL publish the mapping from an enumeration's `## Values` table, and
from each other model-table section a module declares (`## States`,
`## Transitions`, `## Workflow`, `## Members`, `## Ubiquitous Language`), to the
record key of the same meaning that the object type's `data_schema` declares,
and SHALL name the owner of every other optional record key of
`agent-ix/spec-objects-business`, as golden fixtures that the Quire extractor
implements, so that every such key has exactly one owner and one authored form.

## Rationale

PLAT-240 (`agent-ix/quoin#335`). FR-071 described `## Values` as a bullet list
(`- value — doc`). The installed module declares it as a `Value | Description`
table with `min_rows: 1` (format-walkthrough decision #7), and quire-rs FR-075
already extracts the table as `model.values`. The table is the contract: a
table gives each value a row, a source locus and a column for the description,
the same shape as every other model table, and no module or corpus artifact
authors the list form. FR-071 is amended to say so.

The module's other optional keys fall into four groups, each with one owner:

- Model tables: quire-rs FR-075 extracts them under `model`, gated by the
  module's `table_row` locator. This requirement maps them onto the record
  keys of the same name, so that the record the schema validates is no longer
  empty of them.
- Relationship keys: `relations` and the typed relation keys (`emits`,
  `persists`) are FR-104 and quire-rs FR-076, from `## Relationships` rows only.
- Single values with no verb: `source` is FR-121, by its single inline id form.
- Derived views: `owner` and `ubiquitous_language` are not authored keys.

## Behavior

### Enumeration Values

- Quire SHALL read an enumeration's `## Values` section as one table with the header `Value | Description`, in that order, when the object type declares the `values` table locator.
- Each data row SHALL map to one semantic-core `EnumValue` with `value` from the `Value` cell (an `Identifier`) and `doc` from the `Description` cell, carried verbatim; an empty `Description` omits `doc`.
- If a `## Values` section holds a bullet list, a fence, or a diagram, or a table with another header, then Quire SHALL emit `semantic.feature-not-extractable` with `reason` `values`, as quire-rs FR-075 refuses an undeclared form; a list is not a legacy form to be read (FR-074 does not apply) and no fallback reads it.
- If a second row repeats a `Value`, then Quire SHALL emit `semantic.duplicate-model-entry` at the second row; if a `Value` cell is not an `Identifier`, then `semantic.invalid-model-cell`.
- A table with a header and no rows SHALL yield an empty `values`, report the key `available`, and leave the module's `min_rows` assertion to refuse it.

### Other model tables

- Quire SHALL map a `## States` table `State | Description` to `states` as `EnumValue[]` by the rule above.
- Quire SHALL map a `## Transitions` table `From | To | Trigger | Guard | Emits` to `transitions` as one `Transition` per row, with `from`, `to`, `trigger`, `guard` (a clause id) and `emits` (one `SemanticId`) taken from quire-rs FR-075; an inline `Guard` expression is carried as `guardExpression` there and is not a `Transition` field.
- Quire SHALL map a `## Workflow` table `Step | Kind | Consumes | Emits | Description` to `steps` as one `ProcessStep` per row, with `name`, `kind`, `consumes` and `emits` (each `SemanticId[]`) and `doc`.
- Quire SHALL map an `aggregate_root` `## Members` table `Member | Multiplicity` to `members` as one `RelationDecl` per row, with `verb` `aggregates`, `category` and `composite` from the verb's registry entry (FR-104), `target` the `SemanticId` of the declaration the `Member` cell names, and `multiplicity` from the cell.
- Quire SHALL map a `domain` `## Ubiquitous Language` table `Term | Description` to `vocabulary` as one `Term` per row. A `Term` requires `doc`, so an empty `Description` is extracted as an absent field and refused by record validation (FR-073), not by this extraction.
- The record SHALL carry no `sourceSpan`; each key's span is reported in `recordSources` (FR-121) and the per-entry spans in `model` (quire-rs FR-075).
- Model-table sections are read by their declared locator and never by FR-121: a `## Values`, `## States`, `## Transitions`, `## Workflow`, `## Members` or `## Ubiquitous Language` heading is an owned key for FR-121 whatever the module's `semantic.mappings` says.

### Owners of every other key

- Quire SHALL derive `emits` (`aggregate_root`, `process`) and `persists` (`repository`) from `## Relationships` rows whose verb is `emits` or `persists` (FR-076), and SHALL NOT read a `## Emits` or `## Persists` section. A module that ships them names the token `relationships`.
- Quire SHALL read an `event`'s `source` from a `## Source` section holding one inline id, by FR-121; a module that ships the key names the token `record-sections` and adds the section.
- Quire SHALL NOT extract `owner` of a `nested_entity` from `## Parent`. The section stays human-facing prose; `owner` is the single inbound `aggregates` or `contains` edge that the bundle declares for the artifact, and is absent when no bundle supplies one.
- Quire SHALL NOT extract `ubiquitous_language` from the `## Ubiquitous Language` table; the string key is a derived view of `vocabulary`.
- `members` of `domain` and `members` of `population` are not mapped by this requirement (Dependencies).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-123-CON-1 | Quoin SHALL read the golden fixtures for every mapping above from the `quire-fixtures` crate in `agent-ix/quire-rs`, which owns and authors them; holding no copy follows the FR-071-CON-2 rule. | Integrity | Dependency inspection |
| FR-123-CON-2 | This requirement SHALL change no artifact of `agent-ix/spec-objects-business`; its skeletons, manifest tokens and schemas are that module's to update against this mapping. | Boundary | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-123-AC-1 | An `enumeration` fixture whose `## Values` table is `Value \| Description` with rows `draft \| Not yet reviewed` and `final \|` extracts `values` as two `EnumValue`s, the second with no `doc`, each validating against `EnumValue.json`, and the record validates against `Enumeration.json`. | Test |
| FR-123-AC-2 | A `## Values` bullet list `- draft — Not yet reviewed` yields `semantic.feature-not-extractable` with `reason` `values` and no `values`; a repeated `Value` yields `semantic.duplicate-model-entry` at the second row; a `Value` of `not valid` yields `semantic.invalid-model-cell`. | Test |
| FR-123-AC-3 | A `state_machine` fixture extracts `states` as `EnumValue[]` and `transitions` as `Transition[]`, and a `process` fixture extracts `steps` as `ProcessStep[]` with `consumes` and `emits` as `SemanticId[]`, each validating against the module schema. | Test |
| FR-123-AC-4 | An `aggregate_root` `## Members` row `OrderLine \| 1..*` extracts to a `RelationDecl` with `verb` `aggregates`, `composite` true and the target `SemanticId`; a `domain` `## Ubiquitous Language` row extracts to a `Term`, and a row with an empty `Description` extracts no `doc` and fails record validation. | Test |
| FR-123-AC-5 | Under `semantic.mappings` naming `record-sections`, an `enumeration` with a `## Values` table yields no `semantic.invalid-model-cell` `column-unknown` and no second reading of the section, and `## States`, `## Transitions`, `## Workflow`, `## Members` and `## Ubiquitous Language` are likewise read once. | Test |
| FR-123-AC-6 | A `repository` `## Relationships` row with verb `persists` and a `process` row with verb `emits` extract `persists` and `emits` through FR-076, a `## Emits` section is prose, and an `event` with a `## Source` inline id extracts `source` under `record-sections`. | Test |
| FR-123-AC-7 | A `nested_entity` fixture with a `## Parent` section extracts no `owner` from it, and extracts `owner` only when the bundle declares one inbound `aggregates` edge; a `domain` fixture extracts no `ubiquitous_language` from its table. | Test |
| FR-123-AC-8 | Every top-level property of every record schema that `agent-ix/spec-objects-business` ships is owned by exactly one of FR-071, FR-072, FR-104, FR-121, this requirement, or quire-rs FR-075 or FR-076, or is listed as unmapped in this requirement; a property that no owner claims fails the test. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md) (cell grammars), [FR-104](./FR-104-relationships-mapping.md), [FR-121](./FR-121-record-key-sections-mapping.md), [FR-073](./FR-073-data-schema-by-path.md), quire-rs FR-075 (model tables, `model` output) and FR-076 (typed relation keys)
- **Upstream, quire-rs**: the extractor change that copies `model` entries to the record keys above, minus `sourceSpan`, and reports them in `recordSources` and `availability.recordKeys` (lane P-PLAT-101)
- **Downstream, `agent-ix/spec-objects-business`**, changes that are that module's to make: name `relationships` (it does) and add `record-sections` before it ships `source`; add `## Source` to the `event` skeleton; state in `Repository.json` that `persists` is one `SemanticId` while FR-076 yields `SemanticId[]` for a typed key, and reconcile; drop or redefine `owner` and `ubiquitous_language` as derived keys; reconcile `Population.json` `PopulationMember` (`type`, `extent` per member) with quire-rs FR-075, which retired the per-member `Type | Extent` table for a `Type` table plus frontmatter `extent`; decide whether `Domain.members` has a source, since no section or mapping fills it
- **Downstream**: the extractor in `agent-ix/quire-rs`
