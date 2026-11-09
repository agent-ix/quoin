---
id: SR-16399
title: integrity review of FR-123 model-table mapping
type: SpecReview
analysis: integrity
scope: agent-ix/quoin; spec/functional/FR-123-model-table-sections-mapping.md; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-071-typed-properties-mapping.md; spec/matrix.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# integrity review of FR-123 model-table mapping

## Summary

Ticket: PLAT-240 (agent-ix/quoin PR #780, head 45b5004a).

Checked FR-123 and the FR-071/FR-121 edits for single-source-per-key, consistency with merged FR-121/FR-122, and truth of every claim about quire-rs FR-075/FR-076 (origin/main 1bf1079) and agent-ix/spec-objects-business (origin/main 15f9138: manifest, schemas, skeletons). Values, states, transitions, steps and vocabulary claims are true; the FR-071 amendment is coherent; emits/persists via FR-076 and source via FR-121 are coherent. Eight defects, two high.

## Scope

- `FR-123` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-read` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-row` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-refusal` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#values-dup` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#states` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#transitions` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#steps` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#vocabulary` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#owned-headings` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#emits-persists` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#source` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#owner` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#ubiquitous-language` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#unmapped-members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#deps-downstream-business` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-121#owned-keys` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-071#enumeration` (examined) — spec/functional/FR-071-typed-properties-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-123 maps every aggregate_root `## Members` row to a RelationDecl with verb `aggregates`, but the module's own aggregate_root skeleton lists `Money \| 3..3` (a value_object) as a member, and the manifest's `allowed_links.aggregates` admits only entity and nested_entity; the mapping emits an `aggregates` edge the module forbids. | spec/functional/FR-123-model-table-sections-mapping.md:64 |
| FND-002 | medium | The same aggregate membership is authored twice: the aggregate_root skeleton has `OrderLine` in `## Members` and also the `## Relationships` row `lines \| aggregates \| nested_entity_001`, so `members` (FR-123) and `relations` (FR-104) carry the same edge from two authored forms, with no rule choosing one or checking they agree. | spec/functional/FR-123-model-table-sections-mapping.md:64 |
| FND-003 | high | `owner` is specified as 'the single inbound aggregates or contains edge that the bundle declares' with no SHALL, yet AC-7 tests a positive extraction of it (aggregates only); the per-artifact extractor's BundleIndex carries ids, object types, names and operations but no edges, and quire-rs FR-075's CR-192 note says owner is to be read from the required `## Parent` section, the opposite of this bullet. | spec/functional/FR-123-model-table-sections-mapping.md:73 |
| FND-004 | medium | The FR-121 edit appends FR-123's keys inside the clause scoped 'for the queue ... extension_point object types of spec-objects-architecture only', so whether the business keys are owned is ambiguous; it labels `ubiquitous_language` and every `members` as keys 'FR-123 maps' though FR-123 says it maps neither; and it omits `owner`, so once the business module adds `record-sections` (as FR-123 directs for `source`) a `## Owner` section becomes a second authored source. | spec/functional/FR-121-record-key-sections-mapping.md:72 |
| FND-005 | medium | The Dependencies ask spec-objects-business to 'state in Repository.json that persists is one SemanticId while FR-076 yields SemanticId[] ... and reconcile', but Repository.json on origin/main already types `persists` as an array of SemanticId, exactly what FR-076 yields (FR-076-AC-16 tests it); the stated mismatch does not exist. | spec/functional/FR-123-model-table-sections-mapping.md:101 |
| FND-006 | medium | `## States` maps to `states` unconditionally, but the `process` object type declares an optional `States` table locator while Process.json declares no `states` property and is sealed (`unevaluatedProperties`), so a process with a `## States` table yields a record its schema rejects; FR-123 says nothing about it and the Dependencies omit it. | spec/functional/FR-123-model-table-sections-mapping.md:61 |
| FND-007 | medium | `guard` is copied from quire-rs FR-075 as a clause id (a string), but Transition.json types `guard` as a semantic-core ClauseRef object (`language`, `clauseId` required), so 'copy model entries' produces a record that fails Transition.json; the id-to-ClauseRef conversion (and its `language`) is unspecified. | spec/functional/FR-123-model-table-sections-mapping.md:62 |
| FND-008 | low | FR-123 calls `ubiquitous_language` 'a derived view of vocabulary', but the module defines it as the section prose (manifest `ubiquitous_language: {from: section_body, after_heading: Ubiquitous Language}`; domain skeleton: 'shared vocabulary prose is preserved as ubiquitous_language'); no requirement derives it, so the claim describes a behaviour nobody owns. | spec/functional/FR-123-model-table-sections-mapping.md:74 |

## Details

- **FND-001** (FR-123#members): spec-objects-business origin/main: `aggregate_root.allowed_links: {aggregates: [entity, nested_entity], contains: [value_object], ...}`; construct `references.members: [aggregate-member]` and value_object carries the `aggregate-member` role; skeletons/aggregate_root.md `## Members` rows `OrderLine | 0..*` and `Money | 3..3`. A fixed verb cannot fit both. Fix: either derive the verb per member from the target's type (aggregates for entity/nested_entity, contains for value_object/enumeration, both registry-driven per FR-104-CON-1), or restrict `## Members` to aggregates targets and state that value members are `contains` Relationships rows; AC-4 must add a value-object member case.

- **FND-002** (FR-123#members): Same defect class as round-1 'competing authoring forms for the same key'. AggregateRoot.json declares both `relations` and `members` as RelationDecl[]. Fix: name one source. Either `## Members` is the only source of aggregates edges and an `aggregates` Relationships row on aggregate_root is refused (or prose), or `members` is derived from `aggregates`/`contains` Relationships rows and `## Members` stops being a mapping source. Add the downstream skeleton change to Dependencies.

- **FND-003** (FR-123#owner): quire-rs origin/main FR-075 Inputs/Declaration references: BundleIndex.artifacts (id, object, operations), objects (id, names); 'No other index field is read'. FR-075 CR-192 note: '`owner` is a RelationDecl read from the required `## Parent` section under the composite-owner role ... wait for a module-side locator'. The Rationale also calls owner a derived view, not an authored key, which contradicts AC-7 expecting extraction. Behaviour says 'aggregates or contains', AC-7 says 'aggregates'. Right resolution: pick one owner. Recommended: owner is a bundle-level derivation, not an extracted record key, so spec-objects-business drops `owner` from NestedEntity.json (single_owner becomes a bundle rule over inbound composite edges), FR-123 lists `owner` as unmapped, AC-7 keeps only the negative half under record-sections, and the quire-rs FR-075 CR-192 note is corrected. If owner stays a record key, specify where the edge set comes from (a new bundle input) with a SHALL.

- **FND-004** (FR-121#owned-keys): Read literally, the architecture-only qualifier covers the FR-123 list, and the next sentence ('A record of any other module with a property of one of those names is read under this requirement') would then hand business `values`/`states`/... to FR-121, contradicting FR-123 line 67. Unscoped, `members` is excluded for every module, including Domain.members and Population.members which nothing then reads. Same class as round-1 'exclusion-list overlap'. Fix: give FR-123's keys their own sentence scoped to object types that declare the corresponding FR-075 table locator; list `ubiquitous_language`, `owner`, and domain/population `members` as excluded-because-unmapped (named as such), not as FR-123 mappings.

- **FND-005** (FR-123#deps-downstream-business): spec-objects-business origin/main schemas/Repository.json: `persists: {type: array, items: {$ref: .../SemanticId.json}}`. The local checkout at ~/dev/spec-objects-business is on branch bump-semantic-core-0.3.0 (older); verify against origin/main. Fix: delete that clause; persists is fully owned by FR-076 today.

- **FND-006** (FR-123#states): spec-objects-business manifest: process `states: {from: table_row, under_section: States, columns: [State, Description]}`; Process.json properties fields, steps, clauses, operations, emits only. The Description scopes to 'the record key ... that the object type's data_schema declares' but the behaviour bullet does not. Fix: state that a model entry is copied to the record only where the record schema declares the key (it stays in `model` otherwise), and add 'add `states` to Process.json or drop the locator' to the module's downstream list.

- **FND-007** (FR-123#transitions): quire-rs FR-075 Outputs: transitions `guard` 'a clause id'; semantic-core ClauseRef `{language, clauseId, sourceSpan?}` required [language, clauseId]. AC-3's schema validation would fail without a stated rule. Fix: state `guard` maps to `{ language: quire, clauseId: <id> }` (FR-072 ClauseRef shape) without sourceSpan.

- **FND-008** (FR-123#ubiquitous-language): Right resolution: the structured key is `vocabulary`; the prose is not a record key. Say that `ubiquitous_language` is unmapped and the module should drop it from Domain.json (or keep it as display-only section_body yield), instead of asserting a derivation.

## Verdict

Not mergeable: FND-001 and FND-003 are high. Clean: FR-071 amendment (table-only, list refused, no FR-074 legacy path), values/states/steps/vocabulary mapping, emits/persists via FR-076, source via FR-121, no-sourceSpan rule, FR-070 untouched.
