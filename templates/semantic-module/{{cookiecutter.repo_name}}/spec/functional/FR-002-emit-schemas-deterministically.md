---
id: FR-002
title: "Emit the declaration schemas deterministically from the TypeSpec source"
type: FR
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001"
    type: "implements"
---

# FR-002: Emit the declaration schemas deterministically from the TypeSpec source

## Description

The emit command SHALL produce one JSON Schema per exported type from
`typespec/main.tsp` using the official emitter, reproducibly, and SHALL fail on
any drift between the source and the committed bytes.

## Inputs

- `typespec/main.tsp` and `typespec/tspconfig.yaml`
- The manifest `version`

## Outputs

- `{{ cookiecutter.package_name }}/schemas/<Model>.json` and `toolchain.json`

## Behavior

- The emit command SHALL write one JSON Schema for every exported type.
- The emit command SHALL write every `$id` and `$ref` as an absolute URL under a declared base.
- The emit command SHALL record the compiler, emitter and semantic-core versions it used, together with a digest over the emitted bytes.
- Where the emit command runs in check mode, it SHALL exit non-zero listing every difference from the committed output, writing no file.
- If the schema toolchain is not installed, then the emit command SHALL fail naming the install command.
- The module SHALL declare no dependency on the Quire engine in its package metadata.
- The repository SHALL carry no `.npmrc`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-002-AC-1 | One schema exists for every exported type. | Test |
| FR-002-AC-2 | No emitted schema is an empty object contract. | Test |
| FR-002-AC-3 | Every `$id` and `$ref` in every emitted schema is absolute. | Test |
| FR-002-AC-4 | `toolchain.json` records the compiler, emitter, semantic-core, base, file list and overall digest. | Test |
| FR-002-AC-5 | Check mode exits zero against the committed output. | Test |
| FR-002-AC-6 | Check mode exits non-zero, naming the file, after one emitted byte changes. | Test |
| FR-002-AC-7 | The package metadata declares no dependency on the engine. | Test |
| FR-002-AC-8 | No `.npmrc` exists at any depth of the repository. | Test |

## Dependencies

- **Upstream**: [FR-001](./FR-001-declare-the-semantic-contract.md)
- **Downstream**: [FR-003](./FR-003-authoring-forms-and-fixtures.md)
