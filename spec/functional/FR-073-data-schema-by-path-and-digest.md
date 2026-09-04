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

When a module declares a `semantic` block, an object type's `data_schema` SHALL
reference the module's emitted JSON Schema by relative path and SHA-256 digest
instead of carrying an inline schema, so that the archetype's structural
contract is the compiled one and drift is detected at load.

## Rationale

Ticket #293 mapping (d). Inline `{type: object}` types nothing; a path plus
digest binds the archetype to the exact emitted schema and to the semantic-core
version recorded in the manifest.

## Behavior

- The `data_schema` value SHALL accept the reference form `{ schema: <relative path>, digest: sha256:<hex> }` in addition to the current inline object form.
- When the reference form is used, the loader SHALL read the file at the path relative to the module root and compare its SHA-256 to `digest`.
- If the computed digest differs from `digest`, then the loader SHALL fail naming the path and both digests.
- The referenced schema SHALL declare an absolute `$id` under the module's semantic package base, with every `$ref` resolving within the module's emitted bundle or the semantic-core bundle at the version in `semantic.semantic_core`.
- When the referenced schema `$ref`s a semantic-core schema at a version other than `semantic.semantic_core`, the loader SHALL fail naming both versions.
- When a module declares a `semantic` block and an object type still carries inline `{type: object}`, the loader SHALL emit a `warning` naming the object type and the migration (FR-074).
- Validation of an artifact's extracted declaration set SHALL use the referenced schema (`FieldDecl.json` and its module wrapper), not the inline placeholder.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-073-CON-1 | The loader SHALL perform no network read to resolve a schema reference; every referenced file ships inside the module. | Offline | Sandbox test |
| FR-073-CON-2 | A module without a `semantic` block SHALL keep the inline `data_schema` form as valid with no warning. | Compatibility | Existing-fixture suite |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-073-AC-1 | `data_schema: { schema: schemas/Entity.json, digest: sha256:… }` loads and the archetype validates a typed-table artifact against the emitted schema. | Test |
| FR-073-AC-2 | A digest mismatch fails naming the path and both digests. | Test |
| FR-073-AC-3 | A reference whose schema `$ref`s semantic-core `0.2.0` while the manifest records `0.1.0` fails naming both. | Test |
| FR-073-AC-4 | Inline `{type: object}` under a module with a `semantic` block yields a `warning` naming the type and migration; without a `semantic` block it is silent. | Test |
| FR-073-AC-5 | A path escaping the module root (`../x.json`) is rejected. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), semantic-core emitted schemas (`agent-ix/filament-core-data` FR-033)
- **Downstream**: `agent-ix/quire-rs#388`, module tickets, [FR-074](./FR-074-legacy-authoring-forms.md)
