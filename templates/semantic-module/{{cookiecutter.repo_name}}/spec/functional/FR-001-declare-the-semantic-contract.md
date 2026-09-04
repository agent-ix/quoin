---
id: FR-001
title: "Declare the semantic-module contract in the manifest"
type: FR
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001"
    type: "implements"
---

# FR-001: Declare the semantic-module contract in the manifest

## Description

The module manifest SHALL carry a `semantic` block at contract version `1.0.0`
and SHALL reference every exported type's emitted schema by path and digest.

## Outputs

- `{{ cookiecutter.package_name }}/manifest.yaml`

## Behavior

- The manifest SHALL carry `contract_version`, `semantic_core`, `package`, `exports`, `imports`, `targets`, `mappings`, `compatibility_posture` and `legacy_forms`, and no other key inside `semantic`.
- The manifest SHALL set `compatibility_posture` to `additive` and `legacy_forms` to `warning`.
- Where `legacy_forms` is `warning`, the manifest SHALL omit `sweep_report`.
- The manifest SHALL name in `exports` exactly the types it declares.
- The manifest SHALL declare each type name exactly once.
- The manifest SHALL carry `data_schema` as a `{schema, digest}` mapping for every exported type.
- The manifest SHALL carry `data_schema: {type: object}` for no exported type.
- The emit command SHALL write every `digest` value from the emitted bytes.
- The manifest SHALL map each entry of `semantic.imports` to an exact version.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | The `semantic` block carries exactly the nine admitted keys. | Test |
| FR-001-AC-2 | `contract_version` is `1.0.0`, the posture is `additive`, `legacy_forms` is `warning`, and `sweep_report` is absent. | Test |
| FR-001-AC-3 | `semantic.exports` and the declared type names are the same set. | Test |
| FR-001-AC-4 | No type name is declared twice. | Test |
| FR-001-AC-5 | Every exported type references its schema as a `{schema, digest}` pair naming that type's emitted file. | Test |
| FR-001-AC-6 | No exported type carries the placeholder `{type: object}` contract. | Test |
| FR-001-AC-7 | Every digest equals the SHA-256 of the file it names. | Test |
| FR-001-AC-8 | Every `semantic.imports` entry is pinned to an exact version. | Test |
| FR-001-AC-9 | The manifest keeps its explanatory comments, proving the digests were rewritten textually rather than reserialized. | Test |

## Dependencies

- **Upstream**: [US-001](../usecase/US-001-declare-a-type-once.md)
- **Downstream**: [FR-002](./FR-002-emit-schemas-deterministically.md)
