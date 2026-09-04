---
id: SR-119
title: "Failure-domain review of issue 293 semantic module contract requirements"
type: SpecReview
analysis: failure-domain
scope: "US-020, FR-070..FR-075, NFR-017, spec/matrix.md TC-1336..TC-1382"
review_set: all
---

# Failure-domain review of issue 293 semantic module contract requirements

## Summary

The review walked the four checklist domains (trust boundaries, entity identity, evaluation
purity, topology) over the `semantic` manifest block, the typed-table and `sysml` fence mapping,
the Invariants/Operations mapping, `data_schema` by path and digest, legacy forms, and package
locks, cross-checking every named shape against the semantic-core grammar (`main.tsp`,
`kernel-scalars.json`, `lowering.json`), IR v1.1 (`semantic-ir.schema.json`), and the installed
module-manifest schema. The rejection paths the FRs name are all tested (TC-1336..1382); the
gaps are in what happens between them: two acceptance criteria produce values the grammar
cannot hold, the schema-reference loader has one failure named out of five, promotion is gated on
a report whose identity is never defined, and name resolution and module import graphs have no
cycle or shadowing rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-082 | high | FR-072-AC-4 uses `Pre: not-archived` as a clause id, but `ClauseRef.clauseId` is an `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`); the golden `OperationDecl` fixture cannot validate against the grammar, and FR-072 never says what a clause id may look like or which layer rejects an ill-formed one. | FR-072-AC-4; FR-072-AC-1; TC-1356; semantic-core `Identifier`, `ClauseRef` |
| FND-083 | high | FR-071 says an unresolvable `Type` token is advisory and "leaves the field's target unresolved", but `TypeRef.target` is required (`SemanticId \| KernelScalar`), so the `FieldDecl[]` for such an artifact cannot validate against `FieldDecl.json` (FR-071-AC-1) or be lowered (`lowering.json` `TypeRef.target` loss none); the representation of an unresolved target, and whether such an artifact still validates against the emitted schema (FR-073), is undefined. | FR-071 behavior 4; FR-071-AC-4; FR-073 behavior 7; TC-1347; TC-1360 |
| FND-084 | medium | FR-073 names one loader failure (digest mismatch) and one boundary (path escape); a missing file, an unreadable file, a file that is not JSON or not a JSON Schema, a `$ref` that resolves to neither bundle, and a `$ref` cycle are all undefined, so the loader could fall back to the inline placeholder exactly as US-020-EX-3 forbids. | FR-073 behavior 2..4; FR-073-AC-2; FR-073-AC-5; US-020-EX-3 |
| FND-085 | medium | The promotion guard hinges on a "recorded advisory sweep report" (FR-074, NFR-017) with no definition of where it is recorded, what identifies it, which module version and corpus snapshot it binds to, or when it goes stale; any file satisfies the guard, so a stale or unrelated report promotes `warning` to `error`. | FR-074 behavior 7; FR-074-AC-3; NFR-017 metric 5; TC-1369 |
| FND-086 | medium | `Type` resolves kernel scalar first, then enumeration, then object "by `id` or title": an object titled `Duration` or `Status` is silently shadowed, two objects sharing a title are ambiguous, and case sensitivity and backtick-wrapped cells (the FR-006 fixture writes `` `id` ``) are unstated; none of these has a defined outcome or locus. | FR-071 behavior 3; FR-071-AC-1; FR-071-AC-4; TC-1344; TC-1347 |
| FND-087 | medium | "Byte-identical normalized `FieldDecl[]`" is required but normalization is not specified: whether an empty `Multiplicity` cell emits `{lower:1,upper:1}` or omits `multiplicity` (`lowering.json` says absent → 1..1), whether `ordered: false` is written or dropped, key order, and the representation of the "authored authority" flag all decide the byte string the property test compares. | FR-071 behavior 13..14; FR-071-AC-2; TC-1345; TC-1352 |
| FND-088 | medium | The semantic-core reader rules are not surfaced by FR-071: bare `Decimal` without `(p,s)` (policy required), a unit on a non-unit scalar (`String [ms]`), `identity` on a `0..*` or `JsonObject` field, `ordered`/`unique` on a `1` multiplicity, and two rows with the same `Field` are all rejected by the reader with no Quoin locus, so the author sees a fixture-level failure instead of a diagnostic at the row. | FR-071 behavior 2, 5..7; kernel-scalars.json `unitAllowed`; semantic-core `FieldDecl`, `Multiplicity`, `TypeRef` reader rules |
| FND-089 | medium | FR-071 resolves objects "in the same bundle" while FR-075 lets a module import another module's semantic package; whether an imported object is a valid `Type` target, how it is named, and whether a cyclic import graph (A imports B imports A) terminates or is rejected at `quoin install` are undefined. | FR-071 behavior 3; FR-075 behavior 1, 3; FR-075-AC-3; TC-1374 |
| FND-090 | medium | Duplicate `semantic.package` rejects "the second" module, but load order is not defined, so which module survives is installation-order dependent; the same module installed at two versions (a legitimate upgrade window under the `(name, version)` manifest identity) also collides with itself. | FR-070 behavior 7; FR-070-AC-6; TC-1341 |
| FND-091 | low | `data_schema` accepts an inline object or `{schema, digest}`, both objects; a value carrying both `type` and `schema` keys, a `digest` with the wrong hex length or upper-case hex, and a `schema` path that is a symlink out of the module root (not `../`) have no stated outcome. | FR-073 behavior 1..2; FR-073-AC-5; TC-1364 |
| FND-092 | low | Clause-id scope is per artifact for duplicates but `Pre:`/`Post:` may name a clause "in a fence inside the subsection"; whether one operation may reference a clause fenced inside another operation, and whether two `### <name>` operation subsections with the same name are rejected, is unstated. | FR-072 behavior 3..5; FR-072-AC-3; FR-072-AC-5 |
| FND-093 | low | A `## Properties` section holding both a bullet list and a typed table, a typed table with an extra trailing column, and multiplicities exceeding `int32` (`1..99999999999`) fall between the typed form and the two named legacy forms with no defined classification. | FR-071 behavior 1, 7; FR-074 behavior 1..3; TC-1367; TC-1368 |
| FND-094 | low | `semantic.semantic_core` pins a version and `semantic.targets` cites the declared target registry, but a pinned semantic-core version absent from the installed catalog and a `targets` value outside the registry have no loader outcome; only the `$ref` version-drift case (FR-073-AC-3) is specified. | FR-070 behavior 2..3; FR-073-AC-3; FR-075 behavior 1 |

## Failure inventory

| Failure shape | Required response |
| --- | --- |
| A clause id or field name does not match `Identifier` | Fail at the heading, `id:` comment, or row with locus; never emit a `ClauseRef`/`FieldDecl` the grammar rejects. |
| A `Type` token resolves to nothing | Record the advisory finding and emit a declaration set the emitted schema still accepts, or state that the artifact's declaration set is withheld; FR-071 must pick one. |
| The referenced schema file is missing, unreadable, not JSON, or `$ref`s outside both bundles | Fail naming the path and reason; never fall back to `{type: object}`. |
| The `$ref` graph or module import graph contains a cycle | Detect and reject naming the cycle; resolution must terminate. |
| A sweep report is absent, stale, or bound to another module version | Reject `legacy_forms: error`; the report must carry the module version and corpus snapshot it measured. |
| An object title shadows a kernel scalar or enumeration, or two objects share a title | Fail with locus naming both candidates; never resolve silently by precedence. |
| Two installed modules, or two versions of one module, declare one `semantic.package` | Reject deterministically with a defined load order, and exempt the same module identity at a newer version or say why not. |
| Table and fence differ only in defaulted fields | Normalization rules (absent vs explicit defaults, key order) make them byte-identical; the golden fixture is the normative form. |
| A reader rule (decimal policy, unit allowance, identity placement, duplicate name) fires | Surface it at the row or fence line with a Quoin diagnostic, not as a downstream fixture failure. |

## Proposed additions

- **FR-071** (behavior): duplicate `Field` names, bare `Decimal`, unit on a non-unit scalar, `identity` on a collection or `JsonObject`, and flags on `1` multiplicity each fail with locus; define the identifier form for `Field`; define the normalized `FieldDecl[]` form the fixture publishes.
- **FR-072** (behavior): clause ids are `Identifier`s; a `Pre:`/`Post:` line may name only clauses under `## Invariants` or fenced in the same subsection; duplicate operation names fail at the second.
- **FR-073** (behavior): missing, unreadable, non-JSON, non-schema, and unresolvable-`$ref` files fail naming the path and reason; `$ref` resolution is cycle-safe; symlinks are resolved before the root check; `digest` is exactly 64 lower-case hex characters.
- **FR-074** (behavior): the sweep report is identified by path and digest in the manifest and must record the module version and corpus snapshot it measured.
- **FR-075** (behavior): the module import graph is acyclic, checked at `quoin install`; an imported object is addressable as a `Type` target by its `ix://` identity.
- **FR-070** (behavior): load order is the catalog order; a `semantic.package` collision between two versions of one module name is not a collision.
- **NFR** (integrity): a loader failure in any semantic path leaves the module unloaded with a diagnostic; no partial semantic model is ever exposed to `quoin write` or validation.
