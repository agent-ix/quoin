---
id: FR-073
title: "data_schema by emitted-schema path and digest"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
---

# FR-073: data_schema by emitted-schema path and digest

## Description

Where a module declares a `semantic` block, an object type's `data_schema` SHALL
reference the module's emitted JSON Schema by relative path and SHA-256 digest
instead of carrying an inline schema, so that the archetype's structural
contract is the compiled one and drift is detected at install.

## Rationale

Ticket #293 mapping (d). Inline `{type: object}` types nothing; a path plus
digest binds the archetype to the exact emitted schema and to the semantic-core
version recorded in the manifest. Emission itself is the module repository's
build (filament-core-data compiler, FR-047-AC-3); Quoin verifies what ships.

## Behavior

- The `data_schema` value SHALL accept the reference form `{ schema: <relative path>, digest: sha256:<64 hex> }` in addition to the current inline object form.
- If an object carries `schema` or `digest` together with any other key, then Quoin SHALL reject it as ambiguous.
- When the reference form is used, Quoin SHALL, at `quoin module install`, read the file at the path relative to the module root and compare the SHA-256 of its bytes (no line-ending normalization) to `digest`.
- If the referenced file is missing, unreadable, not JSON, or not a JSON Schema 2020-12 document with an absolute `$id`, then Quoin SHALL reject the manifest naming the path and the reason.
- If the computed digest differs from `digest`, then Quoin SHALL reject the manifest naming the path and both digests.
- If the path escapes the module root by `..` or by symlink, then Quoin SHALL reject the manifest naming the path.
- The referenced schema's `$id` SHALL be `https://schemas.agent-ix.org/<org>/<repo>/<module version>/<Name>.json`, with every `$ref` resolving within the module's shipped bundle or the semantic-core bundle `https://schemas.agent-ix.org/semantic-core/<semantic.semantic_core>/`.
- If a `$ref` names a semantic-core version other than `semantic.semantic_core`, an unshipped file, or forms a cycle that the resolver cannot close, then Quoin SHALL reject the manifest naming the `$ref`.
- Quoin SHALL vendor the semantic-core JSON Schema bundle at each supported version with recorded provenance (repository, revision, path, digest) so resolution needs no network read.
- Where a module declares a `semantic` block and an object type still carries an inline `data_schema`, Quoin SHALL emit a `warning` `semantic.inline-data-schema` naming the object type and the migration (FR-074).
- Quire SHALL validate each extracted `FieldDecl` against the vendored `FieldDecl.json` and the archetype's extracted record against the referenced `<Name>.json` (`agent-ix/quire-rs#388`).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-073-CON-1 | Quoin SHALL perform no network read to resolve a schema reference; every referenced file ships inside the module or the vendored semantic-core bundle. | Offline | Integration test with network disabled |
| FR-073-CON-2 | A module without a `semantic` block SHALL keep the inline `data_schema` form valid with no warning. | Compatibility | Existing-fixture suite |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-073-AC-1 | `data_schema: { schema: schemas/Entity.json, digest: sha256:… }` installs, and the fixture artifact's declaration set validates against `FieldDecl.json` while its record validates against `Entity.json`. | Test |
| FR-073-AC-2 | A digest mismatch fails naming the path and both digests; a missing, non-JSON, or `$id`-less file fails naming the path and reason. | Test |
| FR-073-AC-3 | A schema `$ref`ing semantic-core `0.2.0` under a manifest recording `0.1.0`, an unshipped `$ref`, and a `$ref` cycle each fail naming the `$ref`. | Test |
| FR-073-AC-4 | Inline `data_schema` under a module with a `semantic` block yields `semantic.inline-data-schema`; without a `semantic` block it is silent. | Test |
| FR-073-AC-5 | A path escaping the module root by `..` or by symlink is rejected; `{ schema, digest, type: object }` is rejected as ambiguous. | Test |
| FR-073-AC-6 | The vendored semantic-core bundle carries provenance equal to the filament-core-data `toolchain.json` digest at the recorded revision. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-029](./FR-029-consume-quire-json-contract.md) (vendoring pattern), semantic-core emitted schemas (`agent-ix/filament-core-data` FR-033)
- **Downstream**: `agent-ix/quire-rs#388`, module tickets, [FR-074](./FR-074-legacy-authoring-forms.md)
