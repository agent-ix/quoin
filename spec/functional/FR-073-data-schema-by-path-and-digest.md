---
id: FR-073
title: "data_schema by emitted-schema path"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
---

# FR-073: data_schema by emitted-schema path

## Description

Where a module declares a `semantic` block, an object type's `data_schema` SHALL
reference the module's emitted JSON Schema by relative path instead of
carrying an inline schema, so that the archetype's structural contract is the
compiled one.

## Rationale

Ticket #293 mapping (d). Inline `{type: object}` types nothing. Emission itself is the module repository's
build (filament-core-data compiler, FR-047-AC-3); Quoin verifies what ships.

## Behavior

- The `data_schema` value SHALL accept the reference form `{ schema: <relative path> }` in addition to the current inline object form.
- If an object carries `schema` together with any other key, then Quoin SHALL reject it as ambiguous.
- If the referenced file is missing, unreadable, not JSON, or not a JSON Schema 2020-12 document with an `$id`, then Quoin SHALL reject the manifest naming the path and the reason.
- If the path escapes the module root by `..` or by symlink, then Quoin SHALL reject the manifest naming the path.
- Every `$ref` in the referenced schema SHALL resolve within the module's shipped bundle, whose base is `https://schemas.agent-ix.org/<org>/<repo>/` and carries no module version, or the semantic-core bundle `https://schemas.agent-ix.org/semantic-core/<semantic.semantic_core>/`.
- If a `$ref` names a semantic-core version other than `semantic.semantic_core`, an unshipped file, or forms a cycle that the resolver cannot close, then Quoin SHALL reject the manifest naming the `$ref`.
- Quoin SHALL ship the semantic-core JSON Schema bundle at each supported version so resolution needs no network read.
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
| FR-073-AC-1 | `data_schema: { schema: schemas/Entity.json }` installs, and the fixture artifact's declaration set validates against `FieldDecl.json` while its record validates against `Entity.json`. | Test |
| FR-073-AC-2 | A missing, non-JSON, or `$id`-less file fails naming the path and reason. | Test |
| FR-073-AC-3 | A schema `$ref`ing semantic-core `0.2.0` under a manifest recording `0.1.0`, an unshipped `$ref`, and a `$ref` cycle each fail naming the `$ref`. | Test |
| FR-073-AC-4 | Inline `data_schema` under a module with a `semantic` block yields `semantic.inline-data-schema`; without a `semantic` block it is silent. | Test |
| FR-073-AC-5 | A path escaping the module root by `..` or by symlink is rejected; `{ schema, type: object }` is rejected as ambiguous. | Test |

> **CR note (2026-09-30, agent-ix/quoin#658):** The `data_schema` digest and the versioned `$id`
> are no longer enforced. Quoin no longer compares a referenced schema's bytes
> to a recorded `digest` (the `semantic.data-schema-digest` and
> `semantic.data-schema-digest-mismatch` codes are removed) or requires its
> `$id` to equal a path built from the module version
> (`semantic.data-schema-id` is removed). The package manager already fixes
> which bytes ship. The `$id` is no longer required to be absolute. FR-073-AC-6 (bundle provenance equal to a recorded digest at a
> recorded revision) is withdrawn, and so are the recorded hashes and
> revisions in `SEMANTIC_CONTRACT`. The id is not reused. A module's own schema base
> carries no version either: it is `https://schemas.agent-ix.org/<org>/<repo>/`, so the
> manifest version is not copied into any `$id` or `$ref`. The semantic-core base keeps
> its `semantic-core/<version>/` shape.

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-029](./FR-029-consume-quire-json-contract.md) (vendoring pattern), semantic-core emitted schemas (`agent-ix/filament-core-data` FR-033)
- **Downstream**: `agent-ix/quire-rs#388`, module tickets, [FR-074](./FR-074-legacy-authoring-forms.md)
