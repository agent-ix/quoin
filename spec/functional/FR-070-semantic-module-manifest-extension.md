---
id: FR-070
title: "Semantic-module manifest extension"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-009"
    type: "depends_on"
---

# FR-070: Semantic-module manifest extension

## Description

Where a module declares semantic types, its manifest SHALL carry one optional
`semantic` block, versioned independently of `manifest_version`, that names the
semantic-core version it imports, the package identity it exports, and the
mappings it uses, so that Quire, Quoin, and the compiler read one boundary.

## Rationale

FR-049-AC-5 keeps catalog concerns and package concerns distinct. A single
optional block keeps every current manifest valid while giving the semantic
concerns one home with its own version. Two programs read the module store:
Quoin at `quoin module install` time and Quire at artifact-validation time;
this requirement allocates the install-time rejections to Quoin and leaves
artifact-time diagnostics to Quire (`agent-ix/quire-rs#388`).

## Behavior

- The `semantic` block SHALL be optional, so a manifest without it loads exactly as today.
- The `semantic` block SHALL admit exactly these keys: `contract_version` (semver, `1.0.0` for this specification), `semantic_core` (the exact `@agent-ix/semantic-core` version), `package` (an IR `packageIdentity`, `<org>/<repo>`), `exports` (object-type names published as semantic types), `imports` (other modules' `package` identities with exact versions), `targets` (values from the filament-core-data `common.schema.json` `target` registry), `mappings` (named representation mappings, FR-071..073), `compatibility_posture` (`strict`, `additive`, or `declared-lossy`, default `additive`), `legacy_forms` (`warning` or `error`, default `warning`, FR-074), and `sweep_report` (relative path, FR-074).
- The `semantic` block SHALL require `contract_version`, `semantic_core`, and `package`.
- The module-manifest JSON Schema, owned by filament-core-service (`agent-ix/filament-core-service#21`), SHALL publish the block with `additionalProperties: false`.
- Quoin SHALL vendor that schema with recorded provenance, as it vendors the Quire output schemas (FR-029).
- When `quoin module install` meets an unknown key inside `semantic`, Quoin SHALL reject the manifest with a diagnostic naming the key rather than degrading to an empty model.
- When `semantic.exports` names an object type the manifest does not declare, Quoin SHALL reject the manifest naming the export.
- When `semantic.contract_version` is outside the versions this specification defines, Quoin SHALL reject the manifest before reading any other semantic key.
- When `semantic.targets` names a value outside the target registry, Quoin SHALL reject the manifest naming the value.
- When `semantic.package` is not of the `<org>/<repo>` form, Quoin SHALL reject the manifest naming the value.
- When two installed modules declare the same `semantic.package`, Quoin SHALL reject the module being installed, naming both modules; catalog load order is the sorted module-root path order (FR-009).
- Quoin SHALL expose the loaded block through the authoring pack (`quoin write`) so an author sees the semantic-core version, package identity, and schema paths in force.
- Quire SHALL apply the same block schema at artifact-validation time and report an invalid block as a document-level diagnostic (`agent-ix/quire-rs#388`).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-070-CON-1 | The `semantic` block SHALL add no required key to the manifest root, `ObjectTypeEntry`, `ArchetypeEntry`, or `ArtifactTypeEntry`. | Compatibility | Manifest schema diff |
| FR-070-CON-2 | Quoin's vendored copy of the module-manifest schema SHALL carry the source repository, revision, path, and SHA-256 of the filament-core-service original. | Integrity | Provenance test |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-070-AC-1 | Every installed default module manifest loads unchanged with no `semantic` block. | Test |
| FR-070-AC-2 | A manifest with `semantic: { contract_version: "1.0.0", semantic_core: "0.1.0", package: "agent-ix/spec-objects-business" }` loads and the pack reports those values. | Test |
| FR-070-AC-3 | A `semantic` block with an unknown key `foo` is rejected naming `foo`; every key in the admitted list is accepted. | Test |
| FR-070-AC-4 | `exports: [endpoint]` on a module without an `endpoint` object type is rejected naming `endpoint`. | Test |
| FR-070-AC-5 | `contract_version: "2.0.0"` is rejected before any other semantic key is read. | Test |
| FR-070-AC-6 | Two modules declaring the same `semantic.package` fail to install together, naming both, with the later module root in sorted order rejected. | Test |
| FR-070-AC-7 | `targets: [go]` and `package: "ix://agent-ix/x"` are each rejected naming the value. | Test |

## Dependencies

- **Upstream**: [US-020](../usecase/US-020-declare-a-semantic-module-contract-once.md), [FR-009](./FR-009-read-module-manifest.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` FR-021 and `common.schema.json`, semantic-core (`agent-ix/filament-core-data#35`), `agent-ix/filament-core-service#21`
- **Downstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md), [FR-074](./FR-074-legacy-authoring-forms.md), [FR-075](./FR-075-semantic-package-exports-and-locks.md), `agent-ix/quire-rs#388`
