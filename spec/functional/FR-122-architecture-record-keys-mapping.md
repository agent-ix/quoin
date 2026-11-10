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

Quoin SHALL publish the authored form of each record key of
`agent-ix/spec-objects-architecture` that no other mapping owns (`provider`,
`exposes`, `delivery`, `versioning`, `registration`, `stability`, `routes`,
`associated_types`, `records`, `endianness`, `thresholds`), and SHALL name the owner of every other
key, as golden fixtures that the Quire extractor implements, so that every
optional key of that module has exactly one owner and one authored form.

## Rationale

PLAT-196 (`agent-ix/quoin#344`): the module ships optional record keys that no
authored Markdown fills. The keys fall into four groups, and each has one owner:

- `fields`, `operations`, `clauses`: FR-071 and FR-072.
- `relations` and the typed relation keys (`requires`, `carries`, `renders`,
  `triggers`, `serializes`, `throttles`): arrays of `SemanticId` named after an
  `allowed_links` verb, owned by FR-104 and quire-rs FR-076, which derive them from `## Relationships` rows only
  (FR-076-CON-4). This requirement adds no second authoring form for them; an
  author declares `requires` as a `## Relationships` row with the verb
  `requires`.
- `featureOrder` and the systems-kind members (`owner`, `declaredType`,
  `multiplicity`, `direction`, `interfaceType`, `sourceEnd`, `targetEnd`,
  `flowDirection`, `sourceElement`, `targetElement`): quire-rs FR-075.
- `provider` and `exposes`: single `SemanticId` values that name no verb, so FR-076 does not derive them; this requirement, by the FR-121 single-value form.
- The other nine keys above: this requirement, by FR-121 or by its own form.

Adopting `record-sections` is a breaking change for the module's artifacts, not
an additive one: a conforming `rate_limit` already carries `## Thresholds` as a
bullet list, and a conforming `extension_point` carries `## Registration` and
`## Stability` as bullet lists. Each is refused as `semantic.feature-not-extractable`
once read under the token, so the module converts its skeletons and corpus when
it names the token (Dependencies). No key is declared not-authored.

## Behavior

### Keys that follow FR-121

- Quire SHALL read these keys by the FR-121 form shown, under the heading shown, when the module names the token `record-sections`.

| Object type | Key | Heading | Form |
|---|---|---|---|
| `external_contract` | `provider` | `## Provider` | one inline id |
| `extension_point` | `exposes` | `## Exposes` | one inline id |
| `queue` | `delivery` | `## Delivery` | one-row table `Semantics \| Ordering \| Retention` |
| `external_contract` | `versioning` | `## Versioning` | one-row table `Scheme \| Breaking Change Signal` |
| `extension_point` | `registration` | `## Registration` | one-row table `Mechanism \| Conflict \| Discovery` |
| `extension_point` | `stability` | `## Stability` | one-row table `Compatibility \| Deprecation Window` |
| `api_endpoint` | `routes` | `## Routes` | table `Method \| Path \| Operation` |

- If a `routes` row's `Operation` cell names no operation of the same artifact's `operations`, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `route-operation-unknown`.

### Keys with their own form

- Quire SHALL read `associated_types` from a `## Associated Types` table `Type | Multiplicity`, mapping each row to one `TypeRef` by the FR-071 `Type` and `Multiplicity` cell grammars.
- Quire SHALL keep an `interface` `## Contract` YAML fence a derived view: the `operations` lowering of `agent-ix/spec-objects-architecture` FR-006 is adopted unchanged, and Quire SHALL NOT take `associated_types` from the fence, so the key has one authored source. This reverses the `associated_types[]` clause of that FR-006 bullet, which the module changes (Dependencies).
- Quire SHALL map a `binary_format` `## Layout` YAML fence to `records` and `endianness`: top-level `endianness` to `endianness`; each `record_types[]` entry to one `RecordLayout` of `name`, `tag` (taken as authored text, so `0x21` stays `"0x21"`), `size`, and `fields`; each `fields[]` entry to one `LayoutField` of `name`, `offset`, `size`, `type`.
- Quire SHALL NOT read a `## Records` or `## Endianness` section as a source of those keys.
- Quire SHALL map a `LayoutField.size` that is a YAML integer to an integer, and one that is a YAML string to an identifier naming a field of exactly one record in the same fence. The identifier is not limited to a sibling of the same record: a variable-length payload's length is normally carried by a header record, as the module's `binary_format` skeleton does (`posting_tuple.payload` is sized by `metadata_block.payload_len`). The module's `LayoutField` doc, which says "sibling field of the same record", is widened to match (Dependencies).
- If a string `size` names no field in the fence, then Quire SHALL emit `semantic.invalid-model-cell` at the fence line with `reason` `size-unknown-field`; if it names fields in two or more records, then `reason` `size-ambiguous`.
- Quire SHALL leave a `## Layout` key with no `RecordLayout` or `LayoutField` counterpart (`format`, `magic`) as documentation and out of the record.
- Quire SHALL read `thresholds` from a `## Thresholds` table `Scope | Metric | Limit | Window | Status | Retry After | Behavior`, mapping each row to one `Threshold` whose `on_exceeded` is built from the `Status`, `Retry After`, and `Behavior` cells; an empty `Retry After` or `Behavior` omits that field of `on_exceeded`.
- If a `Status` cell is empty, then Quire SHALL emit `semantic.invalid-model-cell` at the row with `reason` `status-required`.

### Relation keys

- Quire SHALL derive `relations` and the six typed relation keys of the Rationale from `## Relationships` rows (FR-104, quire-rs FR-076) and SHALL NOT read a `## Requires`, `## Carries`, or similar heading as a source of them (FR-121 excludes them).
- A module that ships them SHALL name the token `relationships`.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-122-CON-1 | This requirement SHALL change no artifact of `agent-ix/spec-objects-architecture`; its skeletons, FR-006, `LayoutField` doc, Out of Scope, and the decision that routes are listed in `## Endpoint` are that module's to update against this mapping. | Boundary | Inspection |
| FR-122-CON-2 | Quoin SHALL read the golden fixtures for every key above from the `quire-fixtures` crate in `agent-ix/quire-rs`, which owns and authors them; holding no copy follows the FR-071-CON-2 rule. | Integrity | Dependency inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-122-AC-1 | A `## Provider` and a `## Exposes` section each holding one inline id extract `provider` and `exposes` as one `SemanticId`; the `delivery`, `versioning`, `registration` and `stability` one-row tables extract to objects that validate against `DeliveryPolicy.json`, `VersioningPolicy.json`, `RegistrationPolicy.json` and `StabilityPolicy.json`, and an omitted optional cell omits its field; the module's current bullet-list `## Registration` is refused as `semantic.feature-not-extractable`. | Test |
| FR-122-AC-2 | A `## Routes` table extracts to `routes` that validates against `RouteDecl.json`; a route naming an operation the artifact does not declare yields `route-operation-unknown`. | Test |
| FR-122-AC-3 | A `## Associated Types` table `Type \| Multiplicity` extracts to `associated_types` as `TypeRef[]` that validates against `TypeRef.json`; an `interface` whose `## Contract` fence lists `associated_types` that its table omits extracts none from the fence. | Test |
| FR-122-AC-4 | The `binary_format` skeleton's `## Layout` fence extracts `endianness` `little` and two `RecordLayout`s, `tag: 0x21` as `"0x21"`, `posting_tuple.payload` with `size: payload_len` as the identifier resolving to `metadata_block.payload_len`, and `magic` absent, and the record validates against `BinaryFormat.json`; a `size` naming no field yields `size-unknown-field` and one naming a field in two records yields `size-ambiguous`; a `## Endianness` section is prose. | Test |
| FR-122-AC-5 | A `## Thresholds` row `per_token \| requests \| 60 \| 1m \| 429 \| true \| ` extracts to a `Threshold` with `on_exceeded` of `status` 429 and `retry_after` true and no `behavior`, validating against `Threshold.json`; the same row with an empty `Status` yields `semantic.invalid-model-cell` `status-required` at the row. | Test |
| FR-122-AC-6 | An `api_endpoint` whose `## Relationships` table holds a `requires` row extracts `requires` through FR-076 as `SemanticId[]`, and a `## Requires` bullet list in the same artifact is prose and yields no diagnostic. | Test |
| FR-122-AC-7 | Every top-level property of every object-type record schema that `agent-ix/spec-objects-architecture` ships is owned by exactly one of FR-071, FR-072, FR-104, FR-121, this requirement, quire-rs FR-075 or quire-rs FR-076, and none is declared not-authored; a property that no owner claims fails the test. | Test |

## Dependencies

- **Upstream**: [FR-121](./FR-121-record-key-sections-mapping.md), [FR-071](./FR-071-typed-properties-mapping.md) (`Type` and `Multiplicity` cells), [FR-072](./FR-072-invariants-and-operations-mapping.md), [FR-104](./FR-104-relationships-mapping.md), quire-rs FR-075 and quire-rs FR-076, `agent-ix/spec-objects-architecture` FR-004 and FR-006 (the record models and the interface lowering adopted here)
- **Upstream, quire-rs**: the `semantic-v1.schema.json` change of FR-121 (`recordSources`, `availability.recordKeys`), owned by `agent-ix/quire-rs`, lane P-PLAT-101
- **Downstream**: the extractor in `agent-ix/quire-rs`; `agent-ix/spec-objects-architecture` changes that are that module's to make: convert the `## Thresholds`, `## Registration` and `## Stability` skeleton sections from bullet lists to the tables above (breaking for its corpus), add the `relationships` token to its manifest `semantic.mappings` (today `typed-table`, `sysml-fence`, `ocl-clause`, `generalization`), since quire-rs FR-076 yields no typed relation keys until then, add `record-sections`, add `## Provider`, `## Exposes`, `## Delivery`, `## Versioning` and `## Routes`, drop `associated_types[]` as a source in FR-006, widen the `LayoutField.size` doc, revisit its decision #20, and name PLAT-196 as the owner in its Out of Scope
