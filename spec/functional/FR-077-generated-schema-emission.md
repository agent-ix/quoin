---
id: FR-077
title: "Generated TypeSpec source and deterministic schema emission"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-073"
    type: "depends_on"
---

# FR-077: Generated TypeSpec source and deterministic schema emission

## Description

The rendered repository SHALL carry a TypeSpec source that imports
`@agent-ix/semantic-core` and an emit command that produces one real JSON Schema
per declared type from that source, so that the module's structural contract has
one authored origin and every schema in the tree is derived from it.

## Rationale

ADR-0005 fixes TypeSpec as the structural source. A template that shipped
hand-written JSON Schemas would give the rendered repository two origins for one
contract and no way to tell which was authoritative. Both completed migrations
converged on the same pipeline — compile with the official emitter into a scratch
directory, keep this module's namespace, absolutize the `$id` and `$ref` values
the emitter leaves relative, render deterministically, and record the toolchain
that produced the bytes.

## Inputs

- `typespec/main.tsp`, importing `@agent-ix/semantic-core` at the pinned version
- `typespec/tspconfig.yaml`, selecting the official `@typespec/json-schema` emitter
- The manifest `version`

## Outputs

- `<package>/schemas/<Model>.json`, one per declared type
- `<package>/schemas/toolchain.json`, recording compiler, emitter, semantic-core, base, normalization, files, and an overall digest
- A check-mode failure listing every file that differs from the committed output

## Behavior

- The rendered `typespec/main.tsp` SHALL import `@agent-ix/semantic-core`.
- The rendered `typespec/main.tsp` SHALL reference the grammar models it needs rather than redeclaring them.
- The rendered emit command SHALL invoke the official `@typespec/json-schema` emitter.
- The rendered emit command SHALL NOT carry a second implementation of that emitter.
- The rendered emit command SHALL write one JSON Schema file per type the rendered manifest exports.
- When the emitter leaves an `$id` or `$ref` relative, the emit command SHALL rewrite it to an absolute URL under this module's base or under the semantic-core base.
- The emit command SHALL discard the schemas the emitter re-emits for imported libraries, so that only this module's namespace is written.
- Where a `$ref` names a model of an imported semantic module, the emit command SHALL rewrite it to that module's schema base at the exact version `semantic.imports` records, rather than to this module's base or to the semantic-core base.
- If a `$ref` resolves to no declared base — this module's, the semantic-core base, or an imported module's — then the emit command SHALL fail naming the reference, rather than writing a schema whose reference points nowhere.
- When the `@jsonSchema` base declared in the source and the manifest `version` disagree, the emit command SHALL fail naming both values, leaving the committed output untouched.
- When `tsp compile` fails, the emit command SHALL fail carrying the compiler diagnostics, leaving the committed output untouched.
- Where the emit command is run in check mode, it SHALL exit non-zero listing every schema, toolchain, and manifest digest that differs from the committed output, writing no file.
- The rendered lint task SHALL run the emit command in check mode, so that schema drift fails the rendered repository's own gate.
- The emit command SHALL record the compiler, emitter, and semantic-core versions it used in `toolchain.json` alongside a digest over the emitted bytes.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-077-CON-1 | An emitted schema SHALL NOT be hand-edited; a wrong schema is corrected in `typespec/main.tsp` and re-emitted. | Integrity | Test (TC-1414) |
| FR-077-CON-2 | The rendered emitted schemas SHALL be `{type: object}` for no exported type. | Completeness | Test (TC-1409) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-077-AC-1 | In each rendered variant, running the emit command writes one schema per exported type plus `toolchain.json`, and every `$ref` resolves. | Test (TC-1408) |
| FR-077-AC-2 | No emitted schema of any rendered variant is the placeholder `{"type": "object"}` contract. | Test (TC-1409) |
| FR-077-AC-3 | Running the emit command twice over an unchanged tree produces byte-identical output. | Test (TC-1413) |
| FR-077-AC-4 | Check mode exits zero on the committed output and non-zero, naming the file, after one emitted byte is changed. | Test (TC-1414) |
| FR-077-AC-5 | Editing the manifest `version` without editing the `@jsonSchema` base fails the emit command naming both values, and no committed file changes. | Test (TC-1415) |
| FR-077-AC-6 | The rendered `main.tsp` imports `@agent-ix/semantic-core` and redeclares no model the grammar already declares. | Test (TC-1416) |
| FR-077-AC-7 | A rendered source with a deliberate TypeSpec error fails the emit command with the compiler diagnostics in the message, and no committed schema, toolchain, or manifest byte changes. | Test (TC-1449) |
| FR-077-AC-8 | In a mixed rendering with an imported module, a `$ref` to an imported model is written against that module's base at the imported version; a `$ref` matching no declared base fails the emit command naming the reference. | Test (TC-1457) |

## Dependencies

- **Upstream**: [FR-076](./FR-076-semantic-module-template-variants.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md)
- **Downstream**: [FR-078](./FR-078-generated-manifest-semantic-block.md), [NFR-019](../non-functional/NFR-019-deterministic-rendering.md)
