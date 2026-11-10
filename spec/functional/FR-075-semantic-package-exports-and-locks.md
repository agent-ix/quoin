---
id: FR-075
title: "Semantic package manifest derivation"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
---

# FR-075: Semantic package manifest derivation

## Description

Where a module declares a `semantic` block, Quoin SHALL derive the module's
`filament-core-data` package-manifest document, so that dynamic use and static
generated use share one identity graph.

## Rationale

Ticket #293 deliverable three and FR-049: dynamic modules and finite generated
packages coexist over one identity graph.

## Behavior

- `quoin module install` SHALL write `<module root>/semantic/package-manifest.json` conforming to `filament-core-data` `package-manifest.schema.json`, with `contractVersion: "1.0.0"`, `package.identity` = `semantic.package`, `package.version` = the module `version`, `schemaDialect` = the v1 package-manifest constant `https://json-schema.org/draft/2020-12/schema` (the TypeSpec authority is recorded by the IR `source.dialect`, not the manifest; filament-core-data FR-021), `sourceRoots: ["schemas/"]`, `imports` = one entry per `semantic.imports` value, each with `versionConstraint` set to the declared version (the module-manifest schema requires `semantic.imports` values to be exact semantic versions), `exports` = the import's declared type list (`[]` when none), and `capabilities: []`, `exports` = one entry per `semantic.exports` object type with `name`, `typeIdentity: ix://<org>/<repo>/type/<Name>`, and `visibility: public`, `profiles` = one `default` profile (`version` = module version, `options: {}`) selecting all exports, `semantic.targets`, and `semantic.mappings` with `compatibilityPosture: semantic.compatibility_posture`, `targets` = `semantic.targets`, `mappings` = `[]`, `extensions` = `[]`.
- If `semantic.imports` names a package that no installed module provides a version satisfying it, then `quoin module install` SHALL fail naming the import and the installed versions.
- If the import graph over `semantic.imports` contains a cycle at the package level, then `quoin module install` SHALL fail naming the cycle, whatever the type-level graph holds; the type-level graph decides only how the cycle is named.
- The type-level import graph SHALL have nodes `<org>/<repo>#<Type>`, where a module's own `<Type>` is a type file of its schemas and the type name of a `$ref` is the file name stem (the final path segment of the URL with a trailing `.json` removed and then a trailing `.schema` removed if present; a `#fragment` is ignored).
- When a `$ref` in the module's own file `<X>` has a URL under imported package Q's schema base `https://schemas.agent-ix.org/<org>/<repo>/` and names Q's type file `<T>`, the type-level graph SHALL carry the edge `<P>#<X> -> <Q>#<T>`, where `<P>` is the module's `semantic.package`; edges inside a provider come the same way from the provider's own `$ref`s.
- When the `$ref`-derived type-level edges themselves form a cycle, the failure SHALL name the cycle by the `<org>/<repo>#<Type>` nodes that form it; otherwise (including when such edges exist but form no type-level cycle) the failure names the cycle over packages, as it does for a cycle on which a module has no `$ref`s into it.
- If a type listed in `semantic.import_types` is not named by the provider package's `exports`, or the provider is not installed, then `quoin module install` SHALL apply the rules stated once in FR-070 for `semantic.unknown-import-type` and the unresolved-import error.
- `quoin module install` SHALL fill each derived `imports` entry's `exports` with that import's declared `semantic.import_types` list as type names, and with `[]` when none is declared, so the type-level import graph is available to consumers of `package-manifest.json`.
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
| FR-075-AC-3 | An import at a version no installed module provides fails `quoin module install` naming the import and the installed versions; an import cycle fails naming the cycle (FR-075-AC-7 for how the cycle is named). | Test |
| FR-075-AC-4 | The derived manifest's export `typeIdentity` values equal the identities the dynamic load exposes for the same fixture module. | Test |
| FR-075-AC-5 | A `semantic.package` given as `ix://agent-ix/x` or as a URL is rejected. | Test |
| FR-075-AC-6 | For a fixture importing a package with `import_types: ["StR", "FR"]`, the derived `package-manifest.json` import entry carries `exports` naming `StR` and `FR`, and an import with no type list carries `exports: []`. | Test |
| FR-075-AC-7 | Two fixture modules whose `$ref`s give a type-level cycle fail install naming the cycle over the `<org>/<repo>#<Type>` nodes that form it; two modules importing each other with no `$ref`s fail naming the packages; a package cycle on which one module has no `$ref`s into it fails naming the packages; a package cycle fails install even where the `$ref`s form no type-level cycle. | Test |

> **CR note (2026-09-30, agent-ix/quoin#661):** FR-075-AC-2 is
> withdrawn. It required the `registry.json` entry to carry one digest per
> exported object type. The per-export digests are removed from the registry
> entry, and so is the `semantic.export-without-schema` refusal that existed
> only so every export had a digest. The package manager already fixes which
> schema bytes ship. TC-1373 is withdrawn with it. The id is not reused.

> **CR note (2026-09-30, agent-ix/quoin#662):** The Behavior clause recording
> the `package` in the module's `registry.json` entry under
> `semantic` is withdrawn. Both values are copies of the module manifest's
> `semantic` block and nothing reads them back from the registry. The registry
> entry no longer carries a `semantic` member.

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` FR-021 (package graphs and locks)
- **Downstream**: `agent-ix/quoin#290` (publication), `agent-ix/quoin#292`
