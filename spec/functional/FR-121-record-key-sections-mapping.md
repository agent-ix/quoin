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
change to Quoin.

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
- Diagnostics on the quire-rs FR-075 shape (`code`, `severity`, `line`, `section`, `reason`, `sourceSpan`, `message`), with the codes of FR-104.

## Behavior

### Token and key resolution

- Quire SHALL read record-key sections only when `semantic.mappings` names the token `record-sections`.
- A section heading SHALL name a record key when the heading text, lowercased with each run of spaces and hyphens replaced by `_`, equals a property name of the object type's record, other than `fields` (FR-071), `clauses` and `operations` (FR-072), `relations` (FR-104) and `featureOrder`.
- A section whose heading names no record key SHALL be prose to this mapping.
- When `semantic.mappings` does not name `record-sections`, Quire SHALL treat every section as prose to this mapping, with no diagnostic and no `availability.recordKeys`.
- If an artifact holds a second section naming the same key, then Quire SHALL emit the error `semantic.duplicate-section` at the second heading with `reason` `second-section`.
- Quire SHALL NOT read a record key from frontmatter.
- When the artifact has no section naming a key, Quire SHALL omit the key and report `availability.recordKeys` for it `not_applicable`.

### Section shape by key type

- A key whose property is an object SHALL be authored as one table with one data row; the header cells name the object's fields.
- A key whose property is an array of objects SHALL be authored as one table with a data row per element, in row order.
- A key whose property is a scalar SHALL be authored as a body of exactly one inline-code value.
- A key whose property is an array of scalars SHALL be authored as one bullet list with one value per item, written bare or in backticks.
- If a section's first block is not of its key's form, or it holds any further list, fence, table, or diagram, then Quire SHALL emit the error `semantic.feature-not-extractable` at that block's first line with `reason` the key name.
- If an object-valued key's table holds more than one data row, then Quire SHALL emit `semantic.invalid-model-cell` at the second data row with `reason` `second-row`.
- If a section holds no block, then Quire SHALL emit the warning `semantic.record-key-no-block` at the heading with `reason` `no-block` and report the key `not_applicable`.
- A table with a header and no rows SHALL yield an empty array for an array-valued key, report the key `available`, and for an object-valued key SHALL be refused as `semantic.invalid-model-cell` with `reason` `no-row`.

### Columns and cells

- A header cell SHALL match exactly one property of the key's object schema, comparing the cell and the property name lowercased with spaces, hyphens and underscores removed, so `Asserted By` matches `assertedBy` and `Retry After` matches `retry_after`.
- If a header cell matches no property or two header cells match one property, then Quire SHALL emit `semantic.invalid-model-cell` at the header line with `reason` `column-unknown` or `column-duplicate`.
- A cell value SHALL lose any surrounding backticks and be typed by the matched property: `integer` and `number` by their literal (a non-literal is `reason` `cell-type`), `boolean` by `true` or `false`, and a string, an enumeration, or an `anyOf` of enumerations and strings as the authored text.
- A property typed `anyOf` an integer and a string SHALL take the integer when the cell is a decimal integer literal, else the string.
- Quire SHALL NOT check a cell against an enumeration or a `format`; the referenced schema does that at record validation (FR-073).
- A cell authored `unknown`, `not_assessed`, or `not_applicable` SHALL extract as that exact string, never as an absent field and never as a value of the field's scale.
- An empty cell SHALL omit the field; Quire SHALL NOT substitute a default, an empty string, or a scale value.
- A property typed as an object or an array of objects SHALL NOT be authored in a cell; Quire SHALL emit `semantic.invalid-model-cell` at the cell's row with `reason` `nested-object-cell`. A key with such a field uses its own published form (FR-122).
- A property typed `SemanticId` SHALL take its value from the cell by the `Target` rules of FR-104, with the same refusals and the same `semantic.unresolved-target` advisory.
- Quire SHALL apply the cell checks left to right in row order and report only the first failing check per cell.

### Availability

- If any header, row, block, or section of a key carries an error, then Quire SHALL report `availability.recordKeys` for that key `unavailable` with reason `entry-errors: lines <lines>` as FR-104 formats it, with no value for the key and no `recordSources` entry.
- When every row of a key maps without error, Quire SHALL report the key `available` and not lossy.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-121-CON-1 | Quire SHALL NOT infer, default, or derive a score, a status, an acceptance, or an identity a document did not state. | Safety | Fixture inspection |
| FR-121-CON-2 | Quoin SHALL publish the golden fixtures (authored sections, expected `record` and `recordSources`, expected diagnostics) under `tests/fixtures/semantic-module/mapping/` for the extractor in `agent-ix/quire-rs` to consume unchanged. | Integrity | Fixture provenance |
| FR-121-CON-3 | The mapping SHALL be additive: an artifact authored without these sections stays valid under every module that names the token. | Compatibility | Fixture inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-121-AC-1 | A `hazard` fixture whose `## Assessment` table has the header `Severity \| Likelihood \| Exposure \| Controllability \| Rationale` and one row extracts to an `assessment` object that validates against `HazardAssessment.json`, with a `recordSources` entry spanning the section. | Test |
| FR-121-AC-2 | A `failure_mode` fixture whose `## Analysis` table has the header `Effect \| Cause \| Detection` extracts to an `analysis` object that validates against `FailureAnalysis.json`. | Test |
| FR-121-AC-3 | An axis authored `not_assessed` extracts as the string `not_assessed`; an axis whose cell is empty (`Exposure`) is absent from `assessment` and is not `E0`, `negligible`, or any other scale value; a hazard with no `## Assessment` section has no `assessment` key and `availability.recordKeys` `not_applicable`. | Test |
| FR-121-AC-4 | A `hazard` fixture with `## Status` holding `` `accepted` `` and `## Provenance` with `Asserted By \| Asserted At` extracts `status` and `provenance` and validates against `Hazard.json`; the same fixture without `## Provenance` extracts `status` alone, supplies no `provenance`, and fails record validation on the module's accepted-requires-provenance rule. | Test |
| FR-121-AC-5 | A `## Evidence` table `Target \| Kind` extracts to `evidence` in row order with each target resolved by the FR-104 `Target` rules; a header-only table extracts to an empty `evidence` and `available`; a frontmatter `status: accepted` yields no `status`. | Test |
| FR-121-AC-6 | `column-unknown`, `column-duplicate`, `second-row`, `no-row`, `nested-object-cell`, and `cell-type` fixtures each yield `semantic.invalid-model-cell` with that `reason` at the row or header, and a second `## Assessment` section yields `semantic.duplicate-section` `second-section`; each key is `unavailable` with `entry-errors: lines <lines>` and no value. | Test |
| FR-121-AC-7 | A bullet list under a table-form key, and a list after a valid table, each yield `semantic.feature-not-extractable` with `reason` the key name; a prose-only record-key section yields the warning `semantic.record-key-no-block` and `not_applicable`. | Test |
| FR-121-AC-8 | Without the token `record-sections`, the AC-1 fixture yields no `record`, no diagnostic, and no `availability.recordKeys`; a heading that names no record key (`## Notes`) is prose under the token. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md) (`semantic.mappings`), [FR-071](./FR-071-typed-properties-mapping.md), [FR-072](./FR-072-invariants-and-operations-mapping.md) (`SourceLocus`, availability), [FR-073](./FR-073-data-schema-by-path.md) (the referenced schema that directs the mapping and validates the record), [FR-104](./FR-104-relationships-mapping.md) (identity cells, diagnostics, `entry-errors` format), quire-rs FR-075 diagnostic shape
- **Downstream**: [FR-122](./FR-122-architecture-record-keys-mapping.md), the extractor in `agent-ix/quire-rs`, `agent-ix/spec-objects-safety#2`
