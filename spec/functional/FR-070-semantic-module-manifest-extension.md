---
id: FR-070
title: "Semantic-module manifest extension"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
---

# FR-070: Semantic-module manifest extension

## Description

When a module declares semantic types, its manifest SHALL carry one optional
`semantic` block, versioned independently of `manifest_version`, that names the
semantic-core version it imports, the package identity it exports, and the
representation mappings it uses, so that Quire, Quoin, and the compiler read one
boundary.

## Rationale

FR-049-AC-5 keeps catalog concerns and package concerns distinct. A single
optional block keeps every current manifest valid while giving the semantic
concerns one home with its own version.

## Behavior

- The `semantic` block SHALL be optional, so a manifest without it loads exactly as today.
- The `semantic` block SHALL carry `contract_version` (semver, `1.0.0` for this specification), `semantic_core` (the exact `@agent-ix/semantic-core` version the module compiles against), and `package` (an `ix://<org>/<repo>` package identity).
- The `semantic` block MAY carry `exports` (object-type names published as semantic types), `targets` (values from the filament-core-data declared target registry), and `mappings` (named representation mappings, FR-071..073).
- When the loader meets an unknown key inside `semantic`, it SHALL reject the manifest with a diagnostic naming the key rather than degrading to an empty model.
- When `semantic.exports` names an object type the manifest does not declare, the loader SHALL reject the manifest naming the export.
- When `semantic.contract_version` is outside the versions this specification defines, the loader SHALL reject the manifest before reading any other semantic key.
- When two installed modules declare the same `semantic.package`, the loader SHALL reject the second with a diagnostic naming both modules.
- Quoin SHALL expose the loaded block through the authoring pack (`quoin write`) so an author sees the semantic-core version and schema paths in force.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-070-CON-1 | The `semantic` block SHALL add no required key to `ObjectTypeEntry`, `ArchetypeEntry`, or `ArtifactTypeEntry`. | Compatibility | Manifest schema diff |
| FR-070-CON-2 | The module-manifest JSON Schema SHALL publish the `semantic` block with `additionalProperties: false`. | Integrity | Schema inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-070-AC-1 | Every installed default module manifest loads unchanged with no `semantic` block. | Test |
| FR-070-AC-2 | A manifest with `semantic: { contract_version: "1.0.0", semantic_core: "0.1.0", package: "ix://agent-ix/spec-objects-business" }` loads and the pack reports those values. | Test |
| FR-070-AC-3 | A `semantic` block with an unknown key `foo` is rejected naming `foo`. | Test |
| FR-070-AC-4 | `exports: [endpoint]` on a module without an `endpoint` object type is rejected naming `endpoint`. | Test |
| FR-070-AC-5 | `contract_version: "2.0.0"` is rejected before any other semantic key is read. | Test |
| FR-070-AC-6 | Two modules declaring the same `semantic.package` fail to load together, naming both. | Test |

## Dependencies

- **Upstream**: [US-020](../usecase/US-020-declare-a-semantic-module-contract-once.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` package-manifest contract (FR-021), semantic-core (`agent-ix/filament-core-data#35`)
- **Downstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md), [FR-075](./FR-075-semantic-package-exports-and-locks.md), `agent-ix/quire-rs#388`
