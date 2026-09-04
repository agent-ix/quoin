---
id: SR-124
title: "Scope-boundary review of issue 293 semantic module contract requirements"
type: SpecReview
analysis: scope-boundary
scope: "spec/spec.md, US-020, FR-070..FR-075, NFR-017, FR-047"
review_set: all
---

# Scope-boundary review of issue 293 semantic module contract requirements

## Summary

Quoin owns the `semantic` manifest block, the mapping contract and its golden
fixtures, manifest loading at install, lock derivation, and the authoring pack.
Quire (`agent-ix/quire-rs#388`) owns extraction and validation of authored
artifacts against that contract. `filament-core-data` owns the grammar, IR,
emitted schemas, compiler, and target registry; module repositories own
vocabulary, archetypes, and the schemas they ship; clause semantics stay with
`agent-ix/quire-contract-ir#52`; publication (`#290`), the sweep (`#291`), and
catalog locks (`#287`) are sibling tickets. The allocation is consistent with
FR-047 except for one unowned capability: the SysML fence to `FieldDecl[]`
mapping requires line-level parsing of fence content, which the boundary facts
and US-020 deny to Quire and no other component is named to perform. Three
further boundaries (`loader`/`validator` naming, the sweep-report guard, and
the lock shape) are consumed without a named owner or contract, and
`spec/spec.md` does not yet claim the contract in its scope list.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-143 | high | The `sysml` fence to `FieldDecl[]` mapping has no allocated implementer: FR-071 requires recognising `attribute`, `ref item`, `item`, and `:>` lines and failing on `part def` with locus, but US-020 constrains Quire to never parse fence content and FR-071-CON-1 limits Quire to extraction with span; either the constraint admits a lexical SysML subset in Quire or a named frontend owns it. | FR-071; FR-071-CON-1; FR-071-AC-7; US-020; FR-047-AC-1 |
| FND-144 | medium | `spec/spec.md` In Scope has no bullet for the semantic-module contract (manifest block, mapping contract, golden fixtures, lock derivation); the nearest bullet is the architecture record, which is qualified "without activating compiler, publication, or migration work". | spec/spec.md Scope; FR-070..FR-075 |
| FND-145 | medium | FR-070, FR-073, and FR-074 address "the loader" and "the validator" without naming the component; Quire and Quoin both read the shared module store, so install-time rejection (FR-070-AC-6, FR-073-AC-2, FR-075-AC-3) is Quoin's and artifact-time diagnostics (FR-073 last bullet, FR-074 warnings) are Quire's, but the text lets either be read as one program. | FR-070; FR-073; FR-074; FR-047-AC-1; FR-047-AC-2 |
| FND-146 | medium | The `legacy_forms: error` promotion guard consumes a "recorded advisory sweep report" from `agent-ix/quoin#291` with no named location, format, or contract, while FR-074 lists `#291` as downstream; the guard cannot be implemented or tested without that artifact's shape. | FR-074; FR-074-AC-3; NFR-017 |
| FND-147 | medium | FR-075 writes per-export schema digests into the catalog lock and fails `quoin install` on absent import versions, yet lists `agent-ix/quoin#287` (catalog locks) as downstream; the lock's shape is an assumed dependency with no contract in this bundle. | FR-075; FR-075-AC-2; FR-075-AC-3 |
| FND-148 | low | `semantic.targets` values come from the `filament-core-data` declared target registry, an assumed dependency; no acceptance criterion rejects an unknown target, so the boundary is unverified. | FR-070 |
| FND-149 | low | FR-071-AC-1 and FR-074-AC-1 use config-service FR-006 as the fixture; config-service is read-only, so the rewritten typed table and the unmodified copy must live as Quoin-held fixtures, which the text does not state. | FR-071-AC-1; FR-074-AC-1; FR-071-CON-2 |
| FND-150 | low | Emitted schemas referenced by path and digest ship inside the module (FR-073-CON-1) but are produced by the `filament-core-data` compiler; the emission step is correctly outside this specification and belongs to module repositories under FR-047-AC-3, which the text leaves implicit. | FR-073; FR-073-CON-1; FR-047-AC-3 |

## Boundary allocation

| Boundary | Contract responsibility | Excluded responsibility |
| --- | --- | --- |
| Quoin (catalog, install, locks) | Load and reject the `semantic` block, verify `data_schema` digests, derive package manifest and lock entries, expose the authoring pack | Parser semantics, compiling or publishing packages, editing corpus files, changing pins |
| Quire (`agent-ix/quire-rs#388`) | Extract typed tables, fences with span, clause text verbatim; emit legacy-form and resolution diagnostics; validate against referenced schemas | Parsing fence content beyond the FR-071 subset (see FND-143), rendering, clause evaluation |
| `filament-core-data` | Grammar, IR v1.1, `FieldDecl.json`, `package-manifest.schema.json`, target registry, compiler and emitters | Module vocabulary, Quoin lock format, Quire diagnostics |
| Module repositories | Vocabulary, archetypes, `semantic` block values, shipped emitted schemas, semantic versions | The mapping contract, the lock, the sweep |
| `agent-ix/quire-contract-ir#52` | Clause typechecking and evaluation per language | Extraction, `ClauseRef` mapping |
| `agent-ix/quoin#290` / `#291` / `#287` | Publication; advisory sweep and its report; catalog lock format | Anything this bundle declares |
| config-service | Fixture source only | Any write |

## External dependencies

| Dependency | Assumed or guaranteed | Contract |
| --- | --- | --- |
| semantic-core `FieldDecl.json` / `TypeRef` (`filament-core-data` FR-031) | Guaranteed | FR-071-AC-1 validates fixtures against the schema |
| `package-manifest.schema.json` (`filament-core-data` FR-021) | Guaranteed | FR-075-AC-1 |
| IR v1.1 constraint keywords (FR-029) | Guaranteed | FR-071-AC-6 closed keyword set |
| Declared target registry | Assumed | None (FND-148) |
| `agent-ix/quire-rs#388` consumes golden fixtures unchanged | Guaranteed | FR-071-CON-2 fixture provenance |
| `agent-ix/quoin#291` sweep report | Assumed | None (FND-146) |
| `agent-ix/quoin#287` lock shape | Assumed | None (FND-147) |
| Emitted schemas shipped in modules | Guaranteed | FR-073-AC-1..AC-3 digest and version checks |

## Responsibility allocation

| Requirement | Owning component | Class |
| --- | --- | --- |
| US-020 | Quoin (contract) with Quire (extraction) | core |
| FR-070 | Quoin manifest loader and authoring pack | core |
| FR-071 | Quoin (mapping contract, fixtures); Quire (extraction); fence parser unallocated (FND-143) | core |
| FR-072 | Quoin (mapping contract); Quire (extraction) | core |
| FR-073 | Quoin (digest verification at load); Quire (validation against referenced schema) | infrastructure |
| FR-074 | Quire (diagnostics); Quoin (promotion guard, authoring pack) | cross-cutting |
| FR-075 | Quoin (package manifest and lock derivation) | infrastructure |
| NFR-017 | Quoin (schema diff, promotion guard, change-set gate) | cross-cutting |
