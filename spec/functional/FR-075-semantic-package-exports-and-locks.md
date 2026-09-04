---
id: FR-075
title: "Semantic package manifest derivation and registry pins"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-019"
    type: "depends_on"
---

# FR-075: Semantic package manifest derivation and registry pins

## Description

Where a module declares a `semantic` block, Quoin SHALL derive the module's
`filament-core-data` package-manifest document and record per-export schema
digests in the installed-module registry, so that dynamic use and static
generated use share one identity graph.

## Rationale

Ticket #293 deliverable three and FR-049: dynamic modules and finite generated
packages coexist over one identity graph. The catalog lock (`agent-ix/quoin#287`)
does not exist yet, so this requirement pins into the surface that does — the
ts-plugin-kit `registry.json` (FR-019) — and #287 may later move the pins.

## Behavior

- `quoin module install` SHALL write `<module root>/semantic/package-manifest.json` conforming to `filament-core-data` `package-manifest.schema.json`, with `contractVersion: "1.0.0"`, `package.identity` = `semantic.package`, `package.version` = the module `version`, `schemaDialect: "typespec"`, `sourceRoots: ["schemas/"]`, `imports` = one entry per `semantic.imports` value plus `agent-ix/semantic-core` at `semantic.semantic_core`, each with `versionConstraint: "=<version>"`, `exports` = one entry per `semantic.exports` object type with `typeIdentity: ix://<org>/<repo>/type/<Name>` and `visibility: public`, `profiles` = one `default` profile selecting all exports, `semantic.targets`, and `semantic.mappings` with `compatibilityPosture: semantic.compatibility_posture`, `targets` = `semantic.targets`, `mappings` = `[]`, `extensions` = `[]`.
- Quoin SHALL record, in the module's `registry.json` entry under `semantic`, the `package`, `semantic_core`, and one `sha256` per exported object type's referenced schema (FR-073), forming the module's schema fingerprint.
- If `semantic.imports` names a package that no installed module provides at exactly that version, then `quoin module install` SHALL fail naming the import and the installed versions.
- If the import graph over `semantic.imports` contains a cycle, then `quoin module install` SHALL fail naming the cycle.
- Quoin SHALL treat generated package coordinates (`rust`, `typescript`, `python-pydantic-v2`, `python-dataclass`, `json-schema`) as declarations in the derived manifest's `targets`, leaving publication to `agent-ix/quoin#290`.
- The dynamic load path (FR-049-AC-1) SHALL validate artifacts from the shipped schemas with no generated package present.
- The derived manifest's export `typeIdentity` values SHALL equal the identities the dynamic load exposes.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-075-CON-1 | Quoin SHALL NOT compile, publish, or fetch generated packages in this specification; it derives declarations. | Boundary | Static scan for publish/fetch calls |
| FR-075-CON-2 | `semantic.package` and `package.identity` SHALL be the IR `packageIdentity` form `<org>/<repo>`, with type identities of the form `ix://<org>/<repo>/type/<Name>`. | Integrity | Schema validation |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-075-AC-1 | The derived package manifest for a fixture module validates against `filament-core-data` `package-manifest.schema.json` with every required field derived as specified. | Test |
| FR-075-AC-2 | The `registry.json` entry carries one digest per exported object type and changes when an emitted schema changes. | Test |
| FR-075-AC-3 | An import at a version no installed module provides fails `quoin module install` naming the import and the installed versions; an import cycle fails naming the cycle. | Test |
| FR-075-AC-4 | The derived manifest's export `typeIdentity` values equal the identities the dynamic load exposes for the same fixture module. | Test |
| FR-075-AC-5 | A `semantic.package` given as `ix://agent-ix/x` or as a URL is rejected. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-019](./FR-019-manage-plugin-registry.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` FR-021 (package graphs and locks)
- **Downstream**: `agent-ix/quoin#287` (catalog locks may relocate the pins), `agent-ix/quoin#290` (publication), `agent-ix/quoin#292`
