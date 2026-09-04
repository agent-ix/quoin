---
id: SR-120
title: "Integrity review of issue 293 semantic module contract requirements"
type: SpecReview
analysis: integrity
scope: "US-020, FR-070..FR-075, NFR-017, spec/matrix.md TC-1336..TC-1382, spec/spec.md, spec/log.md"
review_set: all
---

# Integrity review of issue 293 semantic module contract requirements

## Summary

Traceability is complete: US-020 traces to StR-002 and StR-003, every FR implements
US-020, every criterion and constraint has one test case, and NFR-017 is scoped with
measurable metrics. The set is not yet consistent with the contracts it imports.
Checked against the semantic-core grammar (`packages/semantic-core/main.tsp`,
`inventory.json`), the IR v1.1 schemas (`schema/semantic/v1/`), and the module-manifest
schema, two requirements contradict their own acceptance criteria (the closed
`semantic` key set versus `legacy_forms` and the posture key; `ix://` package identity
versus the IR `packageIdentity` pattern) and eleven mapping rules leave a criterion
with more than one valid reading. No requirement bundles two obligations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-096 | high | FR-070 closes the `semantic` block to `contract_version`, `semantic_core`, `package`, `exports`, `targets`, `mappings` and rejects any other key (AC-3, CON-2 `additionalProperties: false`), yet FR-074 reads `semantic.legacy_forms` and FR-075 requires an unnamed compatibility-posture key in the same block; as written FR-074-AC-3 and the FR-075 posture bullet are rejected by FR-070-AC-3. | FR-070-AC-3, FR-070-CON-2, FR-074, FR-075 |
| FND-097 | high | `semantic.package` is specified as `ix://<org>/<repo>` (FR-070, FR-075-CON-2 "the form used by the IR"), but `common.schema.json#/$defs/packageIdentity` is `^[a-z0-9][a-z0-9._-]*/[a-z0-9][a-z0-9._-]*$` with no scheme; `ix://` is the `semanticIdentity` pattern. FR-075-AC-1 (derived manifest validates against `package-manifest.schema.json`) cannot pass with FR-070-AC-2's value. | FR-070-AC-2, FR-075-AC-1, FR-075-CON-2 |
| FND-098 | medium | FR-072-AC-4's clause ids `not-archived` and `archived` come from `Pre:`/`Post:` lines, and clause ids are taken from `### <clauseId>` headings, but `ClauseRef.clauseId` is `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`): hyphens and heading spaces are rejected by the grammar, and the `id:` comment syntax per fence language is unstated. | FR-072, FR-072-AC-4 |
| FND-099 | medium | FR-072 sets `sourceSpan` to "the fence's byte span", but semantic-core `SourceLocus` requires `sourceIdentity`, `path`, `startLine`, `startColumn` (line and column, no byte offsets); FR-072-AC-1's expected `ClauseRef` has two valid shapes. | FR-072, FR-072-AC-1 |
| FND-100 | medium | The `sysml` fence admits `item` (composite-owned) and `:> <Type>` (a `references` relation) that `FieldDecl` cannot carry (no composite flag; relations are `RelationDecl` with required `verb` and `category`, neither stated), and the typed table has no row form for either; FR-071-AC-2 byte-identity is defined only over `attribute` and `ref item`. | FR-071, FR-071-AC-2 |
| FND-101 | medium | The `Constraints` cell grammar (`keyword: value`, comma-separated) has no form for valueless `nonEmpty`/`unique`, multi-valued `enumValues`, `pattern` (grammar needs `regex` plus `dialect: ecma-262`; regexes contain commas and colons), or namespaced `format` values; only `min`, `max`, `exclusiveMin`, `exclusiveMax`, `minLength`, `maxLength` are authorable as written. | FR-071, FR-071-AC-6 |
| FND-102 | medium | `Type` resolution to "an enumeration declared in the same bundle" and "an object declared in the same bundle (by `id` or title)" has no declaration side: no rule in FR-071..072 maps a Markdown form to `EnumValue[]`, "bundle" is undefined (module, repository, or corpus), and title collisions have no tie-break; FR-071-AC-4's `Status` and `ConfigOverlay` have no stated origin. | FR-071, FR-071-AC-4 |
| FND-103 | medium | "Byte-identical normalized `FieldDecl[]`" has no normalization rule (key order, whether `multiplicity` for `1` is emitted or absent, flag defaults, encoding of an unresolved target); the property test TC-1345 has no oracle. | FR-071-AC-2, TC-1345 |
| FND-104 | medium | FR-073 names two validation objects: the Behavior bullet validates the extracted declaration set against `FieldDecl.json` and a module wrapper, while AC-1 and the manifest schema's `data_schema` description validate the extracted record against the emitted `Entity.json`; which document the digest-bound schema validates is unstated. | FR-073, FR-073-AC-1 |
| FND-105 | medium | FR-073 puts the referenced schema's `$id` "under the module's semantic package base" and AC-3 reads a semantic-core version out of a `$ref`, but no rule maps a package identity to a URL base or names the semantic-core `$id` scheme (`https://schemas.agent-ix.org/semantic-core/<version>/`); the version comparison has no defined parse. | FR-073, FR-073-AC-3 |
| FND-106 | medium | `package-manifest.schema.json` also requires `schemaDialect`, `sourceRoots`, `profiles`, `mappings`, `extensions`; export items need `typeIdentity` and `visibility`; import items need `versionConstraint`, `exports`, `capabilities`; `compatibilityPosture` lives on `profiles[]`, not the package. FR-075 states none of these derivations, and FR-070 gives the manifest no key from which "any imported modules" are read. | FR-075, FR-075-AC-1, FR-075-AC-3 |
| FND-107 | medium | The schema FR-070-CON-2 amends is `filament-core-service/spec/schemas/module-manifest.schema.json` (top-level `additionalProperties: false`), and the loader that must reject is quire-rs's, not quoin's FR-009 loader, which ignores malformed entries; neither owner appears in FR-070's dependencies, so TC-1338, TC-1342 and TC-1343 have no stated home. | FR-070-CON-2, FR-009, NFR-008 |
| FND-108 | medium | FR-074's promotion guard requires a "recorded advisory sweep report (quoin#291)" but states neither where the record lives, its format, nor how a loader finds it; #291 is an open gate, so until it lands `legacy_forms: error` is unreachable by construction and the interim behavior is a hidden dependency. | FR-074, FR-074-AC-3, NFR-017 |
| FND-109 | low | FR-071 admits `ordered`/`unique` after any multiplicity (`1 ordered`) while the semantic-core reader allows flags only on collections; the mapping should reject or the criterion should state the outcome. | FR-071, FR-071-AC-5 |
| FND-110 | low | FR-072 admits namespaced `ClauseLanguage` values (`<ns>:<name>`) by pattern but assigns behavior only to `ocl`, `sysml`, `fretish`; the namespaced case is neither advisory nor error. | FR-072, FR-072-AC-2 |
| FND-111 | low | FR-071-AC-1 and FR-074-AC-1 test "the unmodified config-service FR-006", but quoin holds no pinned copy or commit provenance of it; NFR-017 forbids corpus writes, not reads, so the fixture's source commit should be declared. | FR-071-AC-1, FR-074-AC-1 |
| FND-112 | low | FR-074 emits "one warning per artifact" without saying which form is named when an artifact carries both a bullet list and a free-column table, and the `- name: type — note` recogniser's tolerance (hyphen versus em dash, missing note) is unstated. | FR-074, FR-074-AC-2 |
| FND-113 | low | FR-073's reference form `{ schema, digest }` is distinguished from an inline schema only by convention; an inline JSON Schema may legally carry unknown keys `schema` and `digest`, so the discriminator (presence of `digest`, absence of `type`) should be stated. | FR-073 |
| FND-114 | low | NFR-017's prohibitions (no generated-package publication, no catalog-pin change, no corpus write) bind FR-073, FR-074 and FR-075, but the NFR constrains only US-020 and FR-070 and those FRs do not reference it. | NFR-017 |
| FND-115 | low | FR-070 rejects "the second" of two modules sharing `semantic.package`; the ordering that makes one second (install order, name order) is unstated, so which module survives is implementation-defined. | FR-070-AC-6 |

## Traceability

| US | FR | StR (via US) | NFR | Verification |
| --- | --- | --- | --- | --- |
| US-020 | FR-070 | StR-002, StR-003 | NFR-017 (constrains) | TC-1336..TC-1343, Test + Static |
| US-020 | FR-071 | StR-002, StR-003 | — | TC-1344..TC-1352, Test + Property + Static |
| US-020 | FR-072 | StR-002, StR-003 | — | TC-1353..TC-1359, Test + Static |
| US-020 | FR-073 | StR-002, StR-003 | — | TC-1360..TC-1366, Test + Static + Integration |
| US-020 | FR-074 | StR-002, StR-003 | — | TC-1367..TC-1371, Test + Integration |
| US-020 | FR-075 | StR-002, StR-003 | — | TC-1372..TC-1378, Test + Static |
| US-020 | — | — | NFR-017 | TC-1379..TC-1382, Test + Analysis |

## Integrity checks

- Every FR carries `implements` to US-020; US-020 carries `traces_to` StR-002 and StR-003 and `depends_on` US-013; every upstream file named (FR-049, NFR-014, US-013, StR-002, StR-003) exists.
- 47 criteria and constraints map one-to-one to TC-1336..TC-1382; US-020-EX-1..4 are realised by TC-1344, TC-1345, TC-1360, TC-1367; spec.md, log.md and the matrix agree on ids and scope.
- Every criterion names a verification method; the five NFR metrics each name a method and TC.
- One `shall` per Behavior bullet; no requirement bundles two observable outcomes.
- Vocabularies checked against the source of truth: the eleven constraint keywords equal `ConstraintKeyword`; the nine kernel scalars are the resolution targets FR-071 names; FR-075's five generated coordinates equal `common.schema.json#/$defs/target`; `contract_version` `1.0.0` equals `package-manifest.contractVersion`; `semantic_core` `0.1.0` equals the semantic-core `$id` base.
- Hidden-assumption probes: external lookup tie-break (FND-102, FND-115); dependency on unimplemented work (FND-108, quoin#291; FND-107, filament-core-service schema); cross-repo ownership (FND-107); no external CLI, pagination, concurrency, or authenticated-API patterns apply.
- Failure-domain check: extension failure is fail-closed at the `semantic` block (FR-070-AC-3) but the block's own key set is inconsistent (FND-096); identity keys are `semantic.package` and `clauseId`, both mis-patterned against their targets (FND-097, FND-098); evaluation purity holds (FR-072-CON-1, FR-073-CON-1, FR-075-CON-1 forbid clause evaluation, network reads, and package builds); topological robustness holds for the duplicate-package and dangling-clause cases but not for the derived package graph, whose import source is undeclared (FND-106).
