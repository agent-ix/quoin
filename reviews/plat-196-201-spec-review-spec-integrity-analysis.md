---
id: SR-16394
title: integrity review of FR-121 and FR-122 record-key mappings
type: SpecReview
analysis: integrity
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# integrity review of FR-121 and FR-122 record-key mappings

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

Checked FR-121 and FR-122 for consistency with each other, with quoin FR-070/071/072/104, with quire-rs FR-075/FR-076 (origin/main), and with the spec-objects-architecture schemas, manifest, skeletons and FR-006 (origin/main). Six defects: three normative contradictions (high), three soundness gaps (medium).

## Scope

- `FR-121-AC-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-4` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-5` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-6` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-7` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-8` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122-AC-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-3` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-4` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-5` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-6` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-7` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-121#heading-rule` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122#id-list-keys` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122#contract-fence` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122#layout-size` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122#rationale` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-121` (context_only) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122` (context_only) — spec/functional/FR-122-architecture-record-keys-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-122 authors requires/carries/renders/triggers/serializes/throttles as bullet lists, but each is a quire-rs FR-076 typed relation key that the engine derives from ## Relationships rows only (FR-076-CON-4): two authorities for one key. | spec/functional/FR-122-architecture-record-keys-mapping.md:44-49 |
| FND-002 | high | FR-121's heading rule excludes only fields/clauses/operations/relations/featureOrder, so it also claims ## Associated Types and ## Thresholds and refuses the FR-122 forms (Type and Status columns are column-unknown), and a ## Endianness heading would be a second source beside ## Layout. | spec/functional/FR-121-record-key-sections-mapping.md:69 |
| FND-003 | high | FR-122-AC-5 expects the skeleton's payload size payload_len to extract as an identifier, but payload_len is a field of metadata_block, not a sibling in posting_tuple, so FR-122's own rule requires size-not-sibling there. | spec/functional/FR-122-architecture-record-keys-mapping.md:85 |
| FND-004 | medium | The heading rule lowercases the heading and compares it to case-sensitive property names, so any camelCase record key (declaredType, sourceEnd, featureOrder style) can never be named and silently stays prose, with no diagnostic. | spec/functional/FR-121-record-key-sections-mapping.md:69 |
| FND-005 | medium | FR-122 says FR-006's operations lowering is adopted unchanged but reverses the same FR-006 bullet's associated_types[] -> record key mapping; the contradiction with spec-objects-architecture FR-006 is not recorded as a required downstream change. | spec/functional/FR-122-architecture-record-keys-mapping.md:63 |
| FND-006 | medium | The additivity claim ignores that conforming architecture artifacts already carry ## Thresholds (required), ## Registration and ## Stability as bullet lists; once the module names record-sections each becomes a feature-not-extractable error. | spec/functional/FR-122-architecture-record-keys-mapping.md:32-34 |

## Details

- **FND-001** (FR-122): Each of these six keys is an array of SemanticId and its name is a verb in the type's allowed_links on spec-objects-architecture origin/main (api_endpoint.requires, queue.carries, ui_component.renders, action.triggers, binary_format.serializes, rate_limit.throttles). quire-rs FR-076 (origin/main) defines exactly that shape as a typed relation key, lowered from `## Relationships` rows only (CON-4, Behavior 'derive a typed key from the ## Relationships rows only'), and FR-069 refuses a locator that duplicates one. FR-122 gives them a second, competing authored form (`## Requires` etc.) and FR-121's heading rule excludes only `relations`, not typed keys. Fix: map these six through FR-104/quire-rs FR-076 typed keys (drop them from the FR-122 table) and exclude typed relation keys from the FR-121 heading rule.

- **FND-002** (FR-121): As written, FR-121 applies to every heading that normalizes to a record property. `## Associated Types` -> `associated_types` (TypeRef[]): header `Type` matches no TypeRef property, so FR-121 emits column-unknown, contradicting FR-122-AC-4. `## Thresholds` -> `thresholds` (Threshold[]): `Status`, `Retry After`, `Behavior` match no Threshold property (they are ExceedResponse fields), so FR-121 emits column-unknown, contradicting FR-122-AC-6. `endianness`/`records` come from the `## Layout` fence under FR-122, yet an `## Endianness` or `## Records` heading would still be read by FR-121, with no duplicate rule between the two sources. The same gap would let FR-121 claim the business module's `## Values` (AGE-987 scope) under the token. Fix: FR-121 excludes every key another mapping owns (FR-122 own-form keys, typed relation keys, quire-rs FR-075 keys), or FR-122 states its forms take precedence and a second source is refused.

- **FND-003** (FR-122-AC-5): spec-objects-architecture origin/main skeletons/binary_format.md: record `posting_tuple` has fields tag, heap_pointer, payload; `payload` has `size: payload_len`; `payload_len` is declared only in record `metadata_block`. FR-122 Behavior: a string size 'SHALL map to an identifier that names a sibling field of the same record, and if it names none then Quire SHALL emit semantic.invalid-model-cell ... size-not-sibling'. AC-5 asserts the opposite outcome on that same skeleton. Either the rule means 'any field of the declaration' (then say so, and 'sibling' is wrong) or AC-5 must expect size-not-sibling (and the skeleton is invalid under the mapping).

- **FND-004** (FR-121): Header cells use a case- and underscore-insensitive match (`Asserted By` -> `assertedBy`), but headings are only lowercased with spaces/hyphens -> `_`, so `## Declared Type` yields `declared_type`, never `declaredType`. All keys the PR names today are lowercase/snake_case, so the rule is unambiguous and collision-free for them, but the Rationale's claim that 'a module adds a key by emitting it and naming the token, with no change to Quoin' is false for camelCase keys, which the semantic-core convention and the architecture systems kinds use. Fix: compare headings with the same normalization as header cells, or state that only lowercase keys are reachable and refuse a camelCase key at module load.

- **FND-005** (FR-122): spec-objects-architecture FR-006 (origin/main) Behavior bullet 1 maps the contract_yaml fence's `associated_types[]` to the record's `associated_types` key. FR-122 makes a new `## Associated Types` table the only source and says the fence entry SHALL NOT be one. The operations part is accurately adopted (FR-006 already names ## Operations the authority, ## Contract the derived view), but FR-122's Downstream names only skeleton and Out-of-Scope updates, not the FR-006 change, so two published specs disagree on the authority for associated_types.

- **FND-006** (FR-122): rate_limit skeleton: `## Thresholds` is required and authored as bullets; extension_point skeleton: `## Registration` and `## Stability` are bullets. Under FR-121 an object/array-of-object key whose first block is a list is refused. 'An artifact without these sections stays valid' is literally true but every rate_limit artifact has the section, so adopting the token is a breaking change for that module. State it as such in FR-122 (the skeleton change is the module's, per CON-1) rather than claiming additivity.


## Verdict

Not mergeable until the high findings are fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | FR-122 assigns provider and exposes to quire-rs FR-076 as typed relation keys, but FR-076 defines a typed relation key as an array of SemanticId named by a verb in the type's allowed_links; provider and exposes are single SemanticIds and neither is a verb (external_contract allows conforms_to/implements/references, extension_point extends/references), so no owner produces them, FR-121's exclusion (defined by FR-076) does not exclude them, and AC-7 fails on FR-122's own table. | spec/functional/FR-122-architecture-record-keys-mapping.md:33-37 |
| FND-008 | low | FR-121 excludes the FR-122 own-form key names (associated_types, thresholds, records, endianness) for every module and object type, though FR-122 owns them only on the architecture types; and the heading-rule example and AC-9 use declaredType, which the next bullet excludes as a quire-rs FR-075 systems member. | spec/functional/FR-121-record-key-sections-mapping.md:70-72,128 |

## Dispositions

Round 1, against head 59833eb2.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |

Round 2, against head 497d1fe5.

| FND | outcome | reason |
| --- | --- | --- |
| FND-007 | fixed | - |
| FND-008 | fixed | - |

FND-008: the Behavior example `## Declared Type` names `declaredType` (line 70) remains; it illustrates normalization only, and the next bullet makes an owned key prose, so it is not a contradiction.
