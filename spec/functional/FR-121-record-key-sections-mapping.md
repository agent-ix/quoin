---
id: FR-121
title: "Record-key sections mapping to a module's own record keys"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-072"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-073"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-104"
    type: "depends_on"
---

# FR-121: Record-key sections mapping to a module's own record keys

## Description

Quoin SHALL publish the mapping from a named section of an object artifact,
authored as a typed table, a bullet list, or one inline value, to the record
key of the same name that the object type's referenced `data_schema` declares,
as golden fixtures that Quire implements, so that a module's own record keys,
including its lifecycle keys, are reachable from a document instead of only
from hand-built records.

## Rationale

FR-071, FR-072 and FR-104 map `fields`, `clauses`, `operations` and `relations`.
A module whose domain content is a table, such as `spec-objects-safety`'s
`## Assessment` and `## Analysis`, or whose record carries a lifecycle
(`status`, `provenance`, `evidence`), has no mapping for those keys, so its
emitted schemas validate records no document can produce (PLAT-201,
`agent-ix/quoin#342`). The mapping is schema-directed: the key set, each key's
shape and each field's scalar type come from the referenced emitted schema
(FR-073), so a module adds a key by emitting it and naming the token, with no
change to Quoin, unless another mapping already owns that key.

Lifecycle keys are authored in body sections, not in frontmatter. Frontmatter
is the artifact identity and graph surface (FR-104 rationale); a body section
has a source locus, which a safety judgement needs so that who asserted what,
and when, is attributable to a line. The mapping never completes a record: an
absent section yields an absent key, an empty cell yields an absent field, and
no score, status or acceptance a document did not state is ever supplied. The
rule "an accepted status requires a provenance" is a schema rule of the module
and is checked by record validation (FR-073), not by this extraction.

## Inputs

- The sections and preamble of an object artifact, with the source identity and path that FR-072 uses to build a `SourceLocus`.
- The module manifest `semantic.mappings` list (FR-070) and the artifact's object type.
- The object type's referenced `data_schema` (FR-073): the record's property names and, per property, its JSON type, `$ref` and `anyOf` branches.
- The bundle package and bundle index of FR-104, for identity cells.

## Outputs

- `record`: the extracted keys, each a JSON value the key's schema property accepts.
- `recordSources`: one `{ key, sourceSpan }` per extracted key, where `sourceSpan` is a semantic-core `SourceLocus` over the key's section. The record is sealed (`unevaluatedProperties: false`), so locus never rides inside it.
- `availability.recordKeys`: per key, `{ state, reason?, lossy }`, as FR-072 reports clauses.
- Both `recordSources` and `availability.recordKeys` are new output keys. quire-rs `schemas/output/semantic-v1.schema.json` closes `availability` to its per-kind entries and has no `recordSources`, so that schema, and the FR-076-AC-9 record-compatibility check over it, change in quire-rs before the first extractor run (Dependencies).
- Diagnostics on the quire-rs FR-075 shape (`code`, `severity`, `line`, `section`, `reason`, `sourceSpan`, `message`), with the codes of FR-104.

## Behavior

### Token and key resolution

- Quire SHALL read record-key sections only when `semantic.mappings` names the token `record-sections`.
- Quire SHALL compare a section heading with a record property name the way it compares a header cell with a field name: lowercase both and remove spaces, hyphens and underscores, so `## Declared Type` names `declaredType` and `## Associated Types` names `associated_types`. The rule is total over camelCase and snake_case keys.
- If two properties of one record normalize to the same name, then Quire SHALL emit `semantic.feature-not-extractable` at the heading with `reason` `key-ambiguous` and read neither.
- Quire SHALL read a section as a record key only when its heading names a property of the object type's record that no other mapping owns, so every key has exactly one authored source. The keys other mappings own are: `fields` (FR-071), `clauses` and `operations` (FR-072), `relations` (FR-104), the typed relation keys quire-rs FR-076 derives from `## Relationships` rows (FR-076-CON-4), `featureOrder` and the systems-kind members quire-rs FR-075 lowers, and, for the `queue`, `api_endpoint`, `interface`, `binary_format`, `rate_limit`, `external_contract` and `extension_point` object types of `agent-ix/spec-objects-architecture` only, the keys FR-122 maps by its own form (`associated_types`, `thresholds`, `records`, `endianness`). For any object type, the keys filled by a declared quire-rs FR-075 table locator (`values`, `states`, `transitions`, `steps`, `members`, `vocabulary`; FR-123) are owned too, and so are the keys FR-123 lists as unmapped (`owner`, `ubiquitous_language`, and the `members` of `domain` and `population`), which no section fills. A record of any other module with a property of one of those names is read under this requirement.
- A section whose heading names no readable record key, including a heading that names an owned key, SHALL be prose to this mapping.
- When `semantic.mappings` does not name `record-sections`, Quire SHALL treat every section as prose to this mapping, with no diagnostic and no `availability.recordKeys`.
- If an artifact holds a second section naming the same key, then Quire SHALL emit the error `semantic.duplicate-section` at the second heading with `reason` `second-section`.
- Quire SHALL NOT read a record key from frontmatter.
- When the artifact has no section naming a key, Quire SHALL omit the key and report `availability.recordKeys` for it `not_applicable`.

### Section shape by key type

- Quire SHALL read a key whose property is an object from one table with one data row; the header cells name the object's fields.
- Quire SHALL read a key whose property is an array of objects from one table, one element per data row in row order.
- Quire SHALL read a key whose property is a scalar from a body of exactly one inline-code value.
- Quire SHALL read a key whose property is an array of scalars from one bullet list with one value per item, written bare or in backticks.
- If a section's first block is not of its key's form, or it holds any further list, fence, table, or diagram, then Quire SHALL emit the error `semantic.feature-not-extractable` at that block's first line with `reason` the key name.
- If an object-valued key's table holds more than one data row, then Quire SHALL emit `semantic.invalid-model-cell` at the second data row with `reason` `second-row`.
- If a section holds no block, then Quire SHALL emit the warning `semantic.record-key-no-block` at the heading with `reason` `no-block` and report the key `not_applicable`.
- A table with a header and no rows SHALL yield an empty array for an array-valued key, report the key `available`, and for an object-valued key SHALL be refused as `semantic.invalid-model-cell` with `reason` `no-row`.

### Columns and cells

- Quire SHALL match each header cell to exactly one property of the key's object schema under the normalization above, so `Asserted By` matches `assertedBy` and `Retry After` matches `retry_after`.
- If a header cell matches no property or two header cells match one property, then Quire SHALL emit `semantic.invalid-model-cell` at the header line with `reason` `column-unknown` or `column-duplicate`.
- Quire SHALL strip surrounding backticks from a cell value and type it by the matched property: `integer` and `number` by their literal (a non-literal is `reason` `cell-type`), `boolean` by `true` or `false`, and a string, an enumeration, or an `anyOf` of enumerations and strings as the authored text.
- Quire SHALL type a property that is `anyOf` an integer and a string as the integer when the cell is a decimal integer literal, else the string.
- Quire SHALL NOT check a cell against an enumeration or a `format`; the referenced schema does that at record validation (FR-073).
- Quire SHALL extract a cell authored `unknown`, `not_assessed`, or `not_applicable` as that exact string, never as an absent field and never as a value of the field's scale.
- Quire SHALL omit the field of an empty cell and SHALL NOT substitute a default, an empty string, or a scale value.
- If a cell targets a property typed as an object or an array of objects, then Quire SHALL emit `semantic.invalid-model-cell` at the cell's row with `reason` `nested-object-cell`. A key with such a field uses its own published form (FR-122).
- Quire SHALL take the value of a `SemanticId` property from the cell by the `Target` rules of FR-104, with the same refusals and the same `semantic.unresolved-target` advisory.
- Quire SHALL apply the cell checks left to right in row order and report only the first failing check per cell.

### Availability

- If any header, row, block, or section of a key carries an error, then Quire SHALL report `availability.recordKeys` for that key `unavailable` with reason `entry-errors: lines <lines>` as FR-104 formats it, with no value for the key and no `recordSources` entry.
- When every row of a key maps without error, Quire SHALL report the key `available` and not lossy.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-121-CON-1 | Quire SHALL NOT infer, default, or derive a score, a status, an acceptance, or an identity a document did not state. | Safety | Fixture inspection |
| FR-121-CON-2 | Quoin SHALL read the golden fixtures (authored sections, expected `record` and `recordSources`, expected diagnostics) from the `quire-fixtures` crate in `agent-ix/quire-rs`, which owns and authors them; holding no copy follows the FR-071-CON-2 rule. | Integrity | Dependency inspection |
| FR-121-CON-3 | The mapping SHALL be additive: an artifact authored without these sections stays valid under every module that names the token. | Compatibility | Fixture inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-121-AC-1 | A `hazard` fixture whose `## Assessment` table has the header `Severity \| Likelihood \| Exposure \| Controllability \| Rationale` and one row extracts to an `assessment` object that validates against `HazardAssessment.json`, with a `recordSources` entry spanning the section. | Test |
| FR-121-AC-2 | A `failure_mode` fixture whose `## Analysis` table has the header `Effect \| Cause \| Detection` extracts to an `analysis` object that validates against `FailureAnalysis.json`. | Test |
| FR-121-AC-3 | An axis authored `not_assessed` extracts as the string `not_assessed`; an axis whose cell is empty (`Exposure`) is absent from `assessment` and is not `E0`, `negligible`, or any other scale value; a hazard with no `## Assessment` section has no `assessment` key and `availability.recordKeys` `not_applicable`. | Test |
| FR-121-AC-4 | A `hazard` fixture with a `## Properties` table (so `fields` is present), `## Assessment`, `## Status` holding `` `accepted` `` and `## Provenance` with `Asserted By \| Asserted At` extracts `status`, `assessment` and `provenance` and validates against `Hazard.json`; the same fixture without `## Provenance` extracts no `provenance` and fails record validation on the module's accepted-requires-provenance rule. | Test |
| FR-121-AC-5 | A `## Evidence` table `Target \| Kind` extracts to `evidence` in row order with each target resolved by the FR-104 `Target` rules; a header-only table extracts to an empty `evidence` and `available`; a frontmatter `status: accepted` yields no `status`. | Test |
| FR-121-AC-6 | `column-unknown`, `column-duplicate`, `second-row`, `no-row`, `nested-object-cell`, and `cell-type` fixtures each yield `semantic.invalid-model-cell` with that `reason` at the row or header, and a second `## Assessment` section yields `semantic.duplicate-section` `second-section`; each key is `unavailable` with `entry-errors: lines <lines>` and no value. | Test |
| FR-121-AC-7 | A bullet list under a table-form key, and a list after a valid table, each yield `semantic.feature-not-extractable` with `reason` the key name; a prose-only record-key section yields the warning `semantic.record-key-no-block` and `not_applicable`. | Test |
| FR-121-AC-8 | Without the token `record-sections`, the AC-1 fixture yields no `record`, no diagnostic, and no `availability.recordKeys`; under the token, a heading that names no record key (`## Notes`) and a heading that names an owned key (`## Thresholds`, `## Requires`, `## Endianness`) are prose and yield no diagnostic. | Test |
| FR-121-AC-9 | A record with a camelCase key (`retryPolicy`) is reached by `## Retry Policy`, and a record with two properties that normalize equal yields `semantic.feature-not-extractable` `key-ambiguous`. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md) (`semantic.mappings`), [FR-071](./FR-071-typed-properties-mapping.md), [FR-072](./FR-072-invariants-and-operations-mapping.md) (`SourceLocus`, availability), [FR-073](./FR-073-data-schema-by-path.md) (the referenced schema that directs the mapping and validates the record), [FR-104](./FR-104-relationships-mapping.md) (identity cells, diagnostics, `entry-errors` format), quire-rs FR-075 diagnostic shape
- **Upstream, quire-rs**: the `semantic-v1.schema.json` change that admits `recordSources` and `availability.recordKeys` (owner: `agent-ix/quire-rs`, lane P-PLAT-101); quire-rs FR-075 and FR-076 (the keys they own, excluded above)
- **Downstream**: [FR-122](./FR-122-architecture-record-keys-mapping.md), the extractor in `agent-ix/quire-rs`, `agent-ix/spec-objects-safety#2`
