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

Quoin SHALL specify the mapping from an enumeration's `## Values` table, and
from each other model-table section a module declares (`## States`,
`## Transitions`, `## Workflow`, `## Members`, `## Ubiquitous Language`), to the
record key of the same meaning that the object type's `data_schema` declares,
and SHALL name the owner of every other optional record key of
`agent-ix/spec-objects-business`, so that every such key has exactly one owner
and one authored form. The golden fixtures live in the `quire-fixtures` crate
of `agent-ix/quire-rs`, which implements the extractor.

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
  keys of the same name where the record schema declares that key.
- Relationship keys: `relations` and the typed relation keys (`emits`,
  `persists`) are FR-104 and quire-rs FR-076, from `## Relationships` rows only.
- Single values with no verb: `source` is FR-121, by its single inline id form.
- Keys with no authored source: `owner`, `ubiquitous_language`, and the `members`
  of `domain` and `population`. A single owner is a rule over the whole bundle,
  which a per-artifact extractor cannot see, and the other three have no form
  that fits their record shape.

## Behavior

### Enumeration Values

- Quire SHALL read an enumeration's `## Values` section as one table with the header `Value | Description`, in that order, when the object type declares the `values` table locator.
- Each data row SHALL map to one semantic-core `EnumValue` with `value` from the `Value` cell (an `Identifier`) and `doc` from the `Description` cell, carried verbatim; an empty `Description` omits `doc`.
- If a `## Values` section holds a bullet list, a fence, or a diagram, or a table with another header, then Quire SHALL emit `semantic.feature-not-extractable` with `reason` `values`, as quire-rs FR-075 refuses an undeclared form; a list is not a legacy form to be read (FR-074 does not apply) and no fallback reads it.
- If a second row repeats a `Value`, then Quire SHALL emit `semantic.duplicate-model-entry` at the second row; if a `Value` cell is not an `Identifier`, then `semantic.invalid-model-cell`.
- A table with a header and no rows SHALL yield an empty `values` and report the key `available`; the module's `min_rows` assertion refuses it.

### Other model tables

- Quire SHALL copy a model entry to the record key of the same name only where the record schema declares that key; otherwise the entry stays in `model` and the record gains no key.
- Quire SHALL map a `## States` table `State | Description` to `states` as `EnumValue[]` by the rule above.
- Quire SHALL map a `## Transitions` table `From | To | Trigger | Guard | Emits` to `transitions` as one `Transition` per row, with `from`, `to`, `trigger`, and `emits` (one `SemanticId`) taken from quire-rs FR-075. A `Guard` that is a clause id SHALL map to `guard` as the semantic-core `ClauseRef` `{ language: "quire", clauseId: <id> }` with no `sourceSpan`, `quire` being the only checked clause language; an inline `Guard` expression is carried as `guardExpression` in `model` and is not a `Transition` field.
- Quire SHALL map a `## Workflow` table `Step | Kind | Consumes | Emits | Description` to `steps` as one `ProcessStep` per row, with `name`, `kind`, `consumes` and `emits` (each `SemanticId[]`) and `doc`.
- Quire SHALL map an `aggregate_root` `## Members` table `Member | Multiplicity` to `members` as one `RelationDecl` per row, and `## Members` SHALL be the only source of `members`.
- Quire SHALL resolve a `Member` cell to a `SemanticId` by the quire-rs FR-075 "Declaration references" rule (an artifact id or a title), admitting only a declaration whose object type carries the role `aggregate-member` by the object type's construct `references.members`, so a member is named the way the module's skeletons name it and resolves to the same identity as an FR-104 `Target`.
- Quire SHALL set a member's `verb` to the one composite verb in the registry that the object type's `allowed_links` admit for the target's object type or role (`aggregates` for an `entity` or `nested_entity`, `contains` for a `value_object`), and `category` and `composite` from that verb's registry entry (FR-104-CON-1).
- If no composite verb or more than one admits a member's target, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `member-verb`.
- If an `aggregate_root` `## Relationships` row carries a verb that `## Members` also derives (`aggregates` or `contains`), then Quire SHALL emit `semantic.duplicate-model-entry` at the row with `reason` `declared-in-members`, as FR-104 refuses a relation declared in frontmatter.
- Quire SHALL map a `domain` `## Ubiquitous Language` table `Term | Description` to `vocabulary` as one `Term` per row. A `Term` requires `doc`, so an empty `Description` SHALL be extracted as an absent field and refused by record validation (FR-073), not by this extraction.
- The record SHALL carry no `sourceSpan`; each key's span is reported in `recordSources` (FR-121) and the per-entry spans in `model` (quire-rs FR-075).
- Quire SHALL NOT read under FR-121 a record key that a declared quire-rs FR-075 table locator of the object type fills (`values`, `states`, `transitions`, `steps`, `members`, `vocabulary`), whatever the heading of its section normalizes to, so the section is read once, by its locator.

### Owners of every other key

- Quire SHALL derive `emits` (`aggregate_root`, `process`) and `persists` (`repository`) from `## Relationships` rows whose verb is `emits` or `persists` (FR-076), and SHALL NOT read a `## Emits` or `## Persists` section. A module that ships them names the token `relationships`.
- Quire SHALL read an `event`'s `source` from a `## Source` section holding one inline id, by FR-121; a module that ships the key names the token `record-sections` and adds the section.
- `owner` of a `nested_entity` is not mapped. Quire SHALL NOT extract it from `## Parent` or any other section; the section stays human-facing prose. The owner is a bundle-level fact, the single inbound composite edge a bundle declares for the artifact, and its check (`single_owner`) is a bundle rule that no per-artifact extraction carries.
- `ubiquitous_language` of a `domain` is not mapped: it is the section's prose, and the structured key is `vocabulary`.
- `members` of `domain` and `members` of `population` are not mapped by this requirement (Dependencies).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-123-CON-1 | Quoin SHALL read the golden fixtures for every mapping above from the `quire-fixtures` crate in `agent-ix/quire-rs`, which owns and authors them; holding no copy follows the FR-071-CON-2 rule as amended by PR #777, which lands first. | Integrity | Dependency inspection |
| FR-123-CON-2 | This requirement SHALL change no artifact of `agent-ix/spec-objects-business`; its skeletons, manifest tokens and schemas are that module's to update against this mapping. | Boundary | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-123-AC-1 | An `enumeration` fixture whose `## Values` table is `Value \| Description` with rows `draft \| Not yet reviewed` and `final \|` extracts `values` as two `EnumValue`s, the second with no `doc`, each validating against `EnumValue.json`, and the record validates against `Enumeration.json`. | Test |
| FR-123-AC-2 | A `## Values` bullet list `- draft — Not yet reviewed` yields `semantic.feature-not-extractable` with `reason` `values` and no `values`; a repeated `Value` yields `semantic.duplicate-model-entry` at the second row; a `Value` of `not valid` yields `semantic.invalid-model-cell`. | Test |
| FR-123-AC-3 | A `state_machine` fixture extracts `states` as `EnumValue[]` and `transitions` as `Transition[]`, a clause-id `Guard` `NotCancelled` becoming `{ language: "quire", clauseId: "NotCancelled" }`, and a `process` fixture extracts `steps` as `ProcessStep[]` with `consumes` and `emits` as `SemanticId[]`, each validating against the module schema; a `process` `## States` table stays in `model` and adds no `states` key to a record whose schema declares none. | Test |
| FR-123-AC-4 | An `aggregate_root` `## Members` with `OrderLine \| 0..*` (a `nested_entity`) and `Money \| 3..3` (a `value_object`) extracts two `RelationDecl`s, `aggregates` and `contains`, both composite with the target `SemanticId`; a member whose target type no composite verb admits yields `semantic.invalid-model-cell` `member-verb`, and an `aggregates` row in the same artifact's `## Relationships` yields `semantic.duplicate-model-entry` `declared-in-members`; a `domain` `## Ubiquitous Language` row extracts to a `Term`, and a row with an empty `Description` extracts no `doc` and fails record validation. | Test |
| FR-123-AC-5 | Under `semantic.mappings` naming `record-sections`, an `enumeration` with a `## Values` table yields no `semantic.invalid-model-cell` `column-unknown` and no second reading of the section, and `## States`, `## Transitions`, `## Workflow` and `## Members` are likewise read once, by their locators. | Test |
| FR-123-AC-6 | A `repository` `## Relationships` row with verb `persists` and a `process` row with verb `emits` extract `persists` and `emits` through FR-076; under `record-sections` a `## Emits` section and a `## Persists` section are prose and yield no diagnostic; an `event` with a `## Source` inline id extracts `source` under `record-sections`. | Test |
| FR-123-AC-7 | Under `semantic.mappings` naming `record-sections`, a `nested_entity` fixture with a `## Parent` section and a `## Owner` section has no `owner` key and no diagnostic, and a `domain` fixture with a `## Ubiquitous Language` table has no `ubiquitous_language` key. | Test |
| FR-123-AC-8 | Every top-level property of every record schema that `agent-ix/spec-objects-business` ships is owned by exactly one of FR-071, FR-072, FR-104, FR-121, this requirement, or quire-rs FR-075 or FR-076, or is one of the four listed unmapped here (`owner`, `ubiquitous_language`, `domain` `members`, `population` `members`); any other property that no owner claims fails the test. | Test |
| FR-123-AC-9 | An `enumeration` fixture whose `## Values` table has a header and no rows extracts an empty `values` and reports `values` `available`. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md) (cell grammars; its CON-2 as amended by PR #777), [FR-104](./FR-104-relationships-mapping.md), [FR-121](./FR-121-record-key-sections-mapping.md), [FR-073](./FR-073-data-schema-by-path.md), quire-rs FR-075 (model tables, `model` output, Declaration references) and quire-rs FR-076 (typed relation keys)
- **Upstream, quire-rs** (lane P-PLAT-101, PLAT-1370): (1) copy the `model` entries to the record keys above, minus `sourceSpan`, only where the record schema declares the key, and report them in `recordSources` and `availability.recordKeys` (the semantic-v1 schema change FR-121 already needs); (2) the FR-121 reader's exclusion of the keys a declared FR-075 table locator fills; (3) resolve an `aggregate_root` `Member` cell by Declaration references with admission from `construct.references.members` (today `read_members` carries the raw string), choose the composite verb by target, and refuse `member-verb` and `declared-in-members`; (4) convert a clause-id `Guard` to a `ClauseRef`; (5) `read_vocabulary` SHALL omit an empty `Description` instead of emitting `""`, as FR-075 already says; (6) correct the FR-075 CR-192 note, which says `owner` is read from `## Parent`
- **Downstream, `agent-ix/spec-objects-business`**, changes that are that module's to make: drop `owner` from `NestedEntity.json` and make `single_owner` a bundle rule; drop `ubiquitous_language` from `Domain.json` or keep it as display-only section prose; add `states` to `Process.json` or drop the process `States` locator; add `record-sections` before it ships `source`, and add `## Source` to the `event` skeleton; stop authoring aggregate membership twice (the `aggregate_root` skeleton lists `OrderLine` in both `## Members` and an `aggregates` `## Relationships` row); `Population.json` `PopulationMember{type, extent}` follows quire-rs FR-075 (a `Type` table plus frontmatter `extent`), so `population` `members` becomes that table; `Domain.members` has no source, so the module drops `members` from `Domain.json` in favour of `relations` and a `## Relationships` locator
- **Downstream**: the extractor in `agent-ix/quire-rs`
