---
id: FR-078
title: "Generated module manifest carries the semantic block and reference-form data_schema"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-073"
    type: "depends_on"
---

# FR-078: Generated module manifest carries the semantic block and reference-form data_schema

## Description

The rendered module manifest SHALL carry a complete `semantic` block that
references every exported type's emitted schema by path and digest, so that a
rendered repository declares the contract from its first commit rather than
acquiring it in a later migration.

## Rationale

FR-070 makes the `semantic` block the one boundary Quire, Quoin, and the compiler
read, and FR-073 makes `data_schema` a `{schema, digest}` reference rather than an
inline placeholder. A rendered repository that omitted either would be a
repository that has to be migrated, which is the outcome this template exists to
prevent. Digests are machine-written from the emitted bytes: a hand-written digest
is a claim nobody checked.

## Inputs

- The rendered template inputs of [FR-076](./FR-076-semantic-module-template-variants.md)
- The emitted schemas of [FR-077](./FR-077-generated-schema-emission.md)

## Outputs

- `<package>/manifest.yaml`, carrying `semantic` and one `data_schema` reference per exported type

## Behavior

- The rendered `semantic` block SHALL carry `contract_version`, `semantic_core`, `package`, `exports`, `imports`, `targets`, `mappings`, `compatibility_posture`, and `legacy_forms`.
- The rendered `semantic.contract_version` SHALL be `1.0.0`, the version FR-070 defines.
- The rendered `semantic.semantic_core` SHALL be the exact version the rendered toolchain installs.
- The rendered `semantic.package` SHALL be `<org>/<repo>` built from the template inputs.
- The rendered `semantic.exports` SHALL name exactly the types the rendered manifest declares.
- The rendered `semantic.legacy_forms` SHALL be `warning`.
- The rendered `semantic.compatibility_posture` SHALL be `additive`.
- Where `semantic.legacy_forms` is `warning`, the rendered manifest SHALL omit `sweep_report`, which FR-074 requires only under `error`.
- The rendered manifest SHALL carry, for every exported type, `data_schema` as a `{schema, digest}` mapping whose `schema` is the repository-relative path of an emitted file and whose `digest` is `sha256:` followed by that file's digest.
- The rendered manifest SHALL carry `data_schema: {type: object}` for no exported type.
- The rendered emit command SHALL write the `digest` values by rewriting the manifest lines textually, so that the manifest's comments and YAML anchors survive regeneration.
- When `imported_modules` is non-empty, the rendered `semantic.imports` SHALL map each imported package identity to its exact version.
- When `imported_modules` is empty, the rendered `semantic.imports` SHALL be an empty mapping rather than an absent key.
- The rendered manifest SHALL declare each exported type name exactly once across `artifact_types` and `object_types`, so that a type name identifies one declaration.
- If two entries of `imported_modules` name the same `<org>/<repo>`, then the template SHALL abort rendering naming the package identity, rather than collapsing them to whichever came last.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-078-CON-1 | The rendered `semantic` block SHALL carry no key outside the set FR-070 admits. | Contract | Test (TC-1417) |
| FR-078-CON-2 | A `digest` value SHALL be produced only by the emit command, never authored by hand. | Integrity | Test (TC-1419) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-078-AC-1 | Each rendered variant's `semantic` block carries the nine keys above and no other, and validates against Quoin's vendored module-manifest schema. | Test (TC-1417) |
| FR-078-AC-2 | Every exported type of every rendered variant has `data_schema: {schema, digest}` whose digest equals the SHA-256 of the file named. | Test (TC-1410) |
| FR-078-AC-3 | No exported type of any rendered variant carries `data_schema: {type: object}`. | Test (TC-1409) |
| FR-078-AC-4 | Regenerating the manifest digests preserves every comment and YAML anchor in the rendered manifest. | Test (TC-1419) |
| FR-078-AC-5 | A mixed-variant rendering with two imported modules yields two `semantic.imports` entries carrying exact versions; an object-variant rendering with none yields `imports: {}`. | Test (TC-1402) |
| FR-078-AC-6 | The rendered `semantic.exports` and the rendered manifest's declared type names are the same set. | Test (TC-1417) |
| FR-078-AC-7 | No exported type name appears in more than one declaration of a rendered manifest, in any variant. | Test (TC-1458) |
| FR-078-AC-8 | Two `imported_modules` entries naming the same package identity abort rendering naming that identity. | Test (TC-1459) |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md), [FR-077](./FR-077-generated-schema-emission.md)
- **Downstream**: [FR-080](./FR-080-generated-verification-suite.md)
