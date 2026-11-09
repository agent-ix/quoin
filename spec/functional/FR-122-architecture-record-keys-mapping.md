---
id: FR-122
title: "Architecture object record keys mapping"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-121"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-104"
    type: "depends_on"
---

# FR-122: Architecture object record keys mapping

## Description

Quoin SHALL publish the authored form of each record key that
`agent-ix/spec-objects-architecture` declares beyond `fields`, `operations`,
`clauses` and `relations`, as golden fixtures that Quire implements, so that
every optional key of that module has an owner and a form an author can write.

## Rationale

PLAT-196 (`agent-ix/quoin#344`): `spec-objects-architecture` ships seventeen
optional record keys, plus `relations`, that no authored Markdown fills, and its
Out of Scope disclaims them to a ticket that did not cover them. Most keys are
flat and follow FR-121 unchanged; four need an explicit form because their
shape is nested or reuses an existing authored section. No key is declared
not-authored: every key below has a Markdown form. The mapping is additive; all
the keys are optional in the shipped schemas, so an artifact without these
sections stays valid.

## Behavior

### Keys that follow FR-121

- The keys below SHALL be authored as the FR-121 section form named in the table, under the heading shown, and a module that ships them SHALL name the token `record-sections`.

| Object type | Key | Heading | Form |
|---|---|---|---|
| `api_endpoint` | `requires` | `## Requires` | bullet list of ids |
| `queue` | `carries` | `## Carries` | bullet list of ids |
| `ui_component` | `renders` | `## Renders` | bullet list of ids |
| `action` | `triggers` | `## Triggers` | bullet list of ids |
| `binary_format` | `serializes` | `## Serializes` | bullet list of ids |
| `rate_limit` | `throttles` | `## Throttles` | bullet list of ids |
| `external_contract` | `provider` | `## Provider` | one inline id |
| `extension_point` | `exposes` | `## Exposes` | one inline id |
| `queue` | `delivery` | `## Delivery` | one-row table `Semantics \| Ordering \| Retention` |
| `external_contract` | `versioning` | `## Versioning` | one-row table `Scheme \| Breaking Change Signal` |
| `extension_point` | `registration` | `## Registration` | one-row table `Mechanism \| Conflict \| Discovery` |
| `extension_point` | `stability` | `## Stability` | one-row table `Compatibility \| Deprecation Window` |
| `api_endpoint` | `routes` | `## Routes` | table `Method \| Path \| Operation` |

- Each `Operation` cell of `routes` SHALL name an operation of the same artifact's `operations`; if it names none, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `route-operation-unknown`.

### Keys with their own form

- `associated_types` SHALL be authored as a `## Associated Types` table `Type | Multiplicity`, each row mapping to one `TypeRef` by the FR-071 `Type` and `Multiplicity` cell grammars, so that `## Operations` stays the one authority for an `interface`'s operations.
- The `interface` `## Contract` YAML fence SHALL stay a derived view: its `operations` lowering of `agent-ix/spec-objects-architecture` FR-006 is adopted unchanged, and its `associated_types` entry SHALL NOT be a source of the key, because two authored sources would be two authorities.
- A `binary_format` `## Layout` YAML fence SHALL map to `records` and `endianness`: top-level `endianness` to `endianness`; each `record_types[]` entry to one `RecordLayout` with `name`, `tag` (taken as authored text, so `0x21` stays `"0x21"`), `size`, and `fields`; each `fields[]` entry to one `LayoutField` of `name`, `offset`, `size`, `type`.
- A `LayoutField.size` that is a YAML integer SHALL map to an integer; a YAML string SHALL map to an identifier that names a sibling field of the same record, and if it names none then Quire SHALL emit `semantic.invalid-model-cell` at the fence line with `reason` `size-not-sibling`.
- A `## Layout` key with no `RecordLayout` or `LayoutField` counterpart (`format`, `magic`) SHALL remain documentation and SHALL NOT be added to the record.
- `thresholds` SHALL be authored as a `## Thresholds` table `Scope | Metric | Limit | Window | Status | Retry After | Behavior`, each row mapping to one `Threshold` whose `on_exceeded` is built from the `Status`, `Retry After`, and `Behavior` cells; an empty `Retry After` or `Behavior` omits that field of `on_exceeded`, and `Status` is required.
- `relations` is owned by [FR-104](./FR-104-relationships-mapping.md); a module that ships it SHALL name the token `relationships`.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-122-CON-1 | This requirement SHALL change no artifact of `agent-ix/spec-objects-architecture`; its skeletons, its Out of Scope, and the decision that routes are listed in `## Endpoint` are that module's to update against this mapping. | Boundary | Inspection |
| FR-122-CON-2 | Quoin SHALL publish the golden fixtures for every key above under `tests/fixtures/semantic-module/mapping/` for the extractor in `agent-ix/quire-rs`. | Integrity | Fixture provenance |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-122-AC-1 | An `api_endpoint`, a `queue`, a `ui_component`, an `action`, a `binary_format` and a `rate_limit` fixture each author their id-list key (`requires`, `carries`, `renders`, `triggers`, `serializes`, `throttles`) as a bullet list and extract the key as `SemanticId[]` in item order; an `external_contract` `## Provider` and an `extension_point` `## Exposes` extract a single `SemanticId`. | Test |
| FR-122-AC-2 | The `delivery`, `versioning`, `registration` and `stability` one-row tables extract to objects that validate against `DeliveryPolicy.json`, `VersioningPolicy.json`, `RegistrationPolicy.json` and `StabilityPolicy.json`, and an omitted optional cell omits its field. | Test |
| FR-122-AC-3 | A `## Routes` table extracts to `routes` that validates against `RouteDecl.json`; a route naming an operation the artifact does not declare yields `route-operation-unknown`. | Test |
| FR-122-AC-4 | A `## Associated Types` table `Type \| Multiplicity` extracts to `associated_types` as `TypeRef[]` that validates against `TypeRef.json`; an `interface` whose `## Contract` fence lists `associated_types` that its table omits extracts none from the fence. | Test |
| FR-122-AC-5 | The `binary_format` skeleton's `## Layout` fence extracts to `endianness` `little` and two `RecordLayout`s, `tag: 0x21` as `"0x21"`, `payload` with `size: payload_len` as that identifier, `magic` absent, and the record validates against `BinaryFormat.json`; a `size` naming no sibling field yields `size-not-sibling`. | Test |
| FR-122-AC-6 | A `## Thresholds` table row `per_token \| requests \| 60 \| 1m \| 429 \| true \| ` extracts to a `Threshold` with `on_exceeded` of `status` 429 and `retry_after` true and no `behavior`, validating against `Threshold.json`; an empty `Status` is refused as an empty required cell. | Test |
| FR-122-AC-7 | Every key of the table above and `relations` is either mapped by a row of this requirement or by FR-104, and none is declared not-authored. | Inspection |

## Dependencies

- **Upstream**: [FR-121](./FR-121-record-key-sections-mapping.md), [FR-071](./FR-071-typed-properties-mapping.md) (`Type` and `Multiplicity` cells), [FR-072](./FR-072-invariants-and-operations-mapping.md), [FR-104](./FR-104-relationships-mapping.md), `agent-ix/spec-objects-architecture` FR-004 and FR-006 (the record models and the interface lowering adopted here)
- **Downstream**: the extractor in `agent-ix/quire-rs`, `agent-ix/spec-objects-architecture` skeleton and Out-of-Scope updates
