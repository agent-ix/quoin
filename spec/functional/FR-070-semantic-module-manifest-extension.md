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
package identity it exports and the
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
- The `semantic` block SHALL admit exactly these keys: `contract_version` (semver, `1.0.0` for this specification), `package` (an IR `packageIdentity`, `<org>/<repo>`), `exports` (object-type names published as semantic types), `imports` (other modules' `package` identities with exact versions), `import_types` (optional per-package type lists, below), `targets` (values from the filament-core-data declared target registry: `common.schema.json` `target` together with its `representationFormat` values, as published), `mappings` (named representation mappings, FR-071..073), `compatibility_posture` (`strict`, `additive`, or `declared-lossy`, default `additive`), `legacy_forms` (`warning` or `error`, default `warning`, FR-074), and `sweep_report` (relative path, FR-074).
- The `semantic` block SHALL require `contract_version` and `package`.
- `semantic.imports` SHALL keep its shape `{ "<org>/<repo>": "<exact semver>" }`, so a manifest that declares imports and no `import_types` validates and behaves exactly as before.
- `semantic.import_types` SHALL be an optional key that maps `<org>/<repo>` to a non-empty array of unique, non-empty type-name strings, naming the types of that package the module uses.
- When `semantic.import_types` is not an object, when a key names a package absent from `semantic.imports`, or when a value is not an array, is empty, holds a non-string entry, an empty string, or a duplicate type name, Quoin SHALL reject the manifest as a shape error whose reason text names the package and the offending entry (`semantic.import_types.<package>`, or `semantic.import_types` for a non-object); `quire load` refuses the same shapes as `semantic.invalid-value` at that locus, so the two tools accept and refuse the same manifests.
- A provider is not installed for an import when no installed module, accepted without refusal, has a `semantic.package` equal to the import key at the exact version `semantic.imports` declares; otherwise the provider is installed.
- When the provider of an import is not installed, `quoin module install` SHALL fail with the existing unresolved-import error (FR-075), so `semantic.unknown-import-type` is never raised for a provider that is not installed.
- When the provider of an import is installed and a type listed in `semantic.import_types` for it is not named by the provider's `exports`, Quoin SHALL refuse `quoin module install` with the diagnostic `semantic.unknown-import-type` (severity error) at `semantic.import_types.<package>`, naming the package and the type.
- When the provider of an import is installed and a `$ref` in the module's own data schemas (the schemas its object and artifact types name through `data_schema`) names, under that package's schema base, a type the provider's `exports` does not name, Quoin SHALL refuse `quoin module install` with the same diagnostic `semantic.unknown-import-type` (severity error) at `<section>[<type>].data_schema.schema`, naming the package and the type, so `quoin module install` refuses what `quire load` refuses (`agent-ix/quire-rs` FR-069), whether or not the import carries an `import_types` list.
- When a declared import, with or without a type list, is referenced by no `$ref` in the module's own data schemas (the schemas its object and artifact types name through `data_schema`, whether emitted or written by hand), Quoin SHALL report the diagnostic `semantic.unused-import` (severity warning) naming the import.
- A reference, for `semantic.unused-import`, is a `$ref` whose URL is under the imported package's schema base `https://schemas.agent-ix.org/<org>/<repo>/`.
- `quoin module install` SHALL emit `semantic.unused-import` as a warning that does not fail the install.
- `quoin module inspect` SHALL list `semantic.unused-import` (FR-124).
- Quoin SHALL admit a module manifest that carries the eight registry keys `lexicon`, `observable_verbs`, `ambiguity_terms`, `vacuous_predicates`, `grammar_severity`, `property_idioms`, `traceability` and `verification_catalog`, and SHALL refuse a manifest whose root carries a key outside the declared set.
- The `semantic` block SHALL NOT admit `semantic_core`: the key is retired (PLAT-1221), a module declares no semantic-core version, and a manifest that still carries it is refused like any other unknown key.
- When `quoin module install` meets an unknown key inside `semantic`, Quoin SHALL reject the manifest with a diagnostic naming the key rather than degrading to an empty model.
- When `semantic.exports` names an object type the manifest does not declare, Quoin SHALL reject the manifest naming the export.
- When `semantic.contract_version` is outside the versions this specification defines, Quoin SHALL reject the manifest before reading any other semantic key.
- When `semantic.targets` names a value outside the target registry, Quoin SHALL reject the manifest naming the value.
- When `semantic.package` is not of the `<org>/<repo>` form, Quoin SHALL reject the manifest naming the value.
- When two installed modules declare the same `semantic.package`, Quoin SHALL reject the module being installed, naming both modules; catalog load order is the sorted module-root path order (FR-009).
- Quoin SHALL expose the loaded block through the authoring pack (`quoin write`) so an author sees the package identity, and schema paths in force.
- Quire SHALL apply the same block schema at artifact-validation time and report an invalid block as a document-level diagnostic (`agent-ix/quire-rs#388`).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-070-CON-1 | The `semantic` block (including `import_types`) SHALL add no required key to the manifest root, `ObjectTypeEntry`, `ArchetypeEntry`, or `ArtifactTypeEntry`. | Compatibility | Manifest schema diff |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-070-AC-1 | Every installed default module manifest loads unchanged with no `semantic` block. | Test |
| FR-070-AC-2 | A manifest with `semantic: { contract_version: "1.0.0", package: "agent-ix/spec-objects-business" }` loads and the pack reports the contract version and package. | Test |
| FR-070-AC-3 | A `semantic` block with an unknown key `foo`, or with the retired key `semantic_core`, is rejected naming that key; every key in the admitted list is accepted. | Test |
| FR-070-AC-4 | `exports: [endpoint]` on a module without an `endpoint` object type is rejected naming `endpoint`. | Test |
| FR-070-AC-5 | `contract_version: "2.0.0"` is rejected before any other semantic key is read. | Test |
| FR-070-AC-6 | Two modules declaring the same `semantic.package` fail to install together, naming both, with the module being installed rejected whatever its sorted position. | Test |
| FR-070-AC-7 | `targets: [go]` and `package: "ix://agent-ix/x"` are each rejected naming the value. | Test |
| FR-070-AC-8 | A manifest with `imports: { "agent-ix/spec-artifacts-iso": "0.2.0" }` and no `import_types` loads exactly as it did before this amendment, and one with `import_types: { "agent-ix/spec-artifacts-iso": ["StR", "FR"] }` alongside that import loads and the pack reports the lists. | Test |
| FR-070-AC-9 | An `import_types` key absent from `imports`, an empty list, a duplicate type name, an empty type name, a non-array value and a non-string entry are each rejected with reason text naming the package and the entry, and a non-object `import_types` is rejected naming `semantic.import_types`. | Test |
| FR-070-AC-10 | With the provider installed and its `exports` lacking `Nope`, an `import_types` entry listing `Nope` fails `quoin module install` with `semantic.unknown-import-type` naming the package and `Nope`. | Test |
| FR-070-AC-11 | With the provider not installed, an install of a module carrying an `import_types` entry for it fails with the existing unresolved-import error only, and raises no `semantic.unknown-import-type`. | Test |
| FR-070-AC-12 | An import, with and without a type list, referenced by no `$ref` under `https://schemas.agent-ix.org/<org>/<repo>/` in the module's own data schemas (emitted or hand-written) yields `semantic.unused-import` at warning severity naming it, `quoin module install` still succeeds with that warning, and an import that one such `$ref` references yields none. | Test |
| FR-070-AC-13 | With the provider installed and its `exports` lacking `Nope`, a `$ref` in the module's own data schema for type `T` naming `Nope` under the provider's schema base fails `quoin module install` with `semantic.unknown-import-type` at `<section>[T].data_schema.schema` naming the package and `Nope`, with and without an `import_types` list; a `$ref` naming an exported type raises none. | Test |
| FR-070-AC-14 | A module manifest carrying all eight registry keys `lexicon`, `observable_verbs`, `ambiguity_terms`, `vacuous_predicates`, `grammar_severity`, `property_idioms`, `traceability` and `verification_catalog` validates against the pinned `@agent-ix/semantic-schema` module-manifest schema, and the same manifest with an unknown root key is refused. | Test |

## Dependencies

- **Upstream**: [US-020](../usecase/US-020-declare-a-semantic-module-contract-once.md), [FR-009](./FR-009-read-module-manifest.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` FR-021 and `common.schema.json`, semantic-core (`agent-ix/filament-core-data#35`), `@agent-ix/semantic-schema` (the module-manifest schema, `agent-ix/filament-core-data`)
- **Downstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-073](./FR-073-data-schema-by-path.md), [FR-074](./FR-074-legacy-authoring-forms.md), [FR-075](./FR-075-semantic-package-exports-and-locks.md), `agent-ix/quire-rs#388`
