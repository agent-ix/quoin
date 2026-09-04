---
id: TASK-046
title: "Rendered TypeSpec source and emit pipeline"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-045"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-077"
    type: references
  - target: "ix://agent-ix/quoin/NFR-019"
    type: references
  - target: "ix://agent-ix/quoin/TC-1408"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1409"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1413"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1414"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1415"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1416"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1449"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1456"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1457"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1467"
    type: verifies
---

# TASK-046: Rendered TypeSpec source and emit pipeline

## Scope

Render a TypeSpec source that imports `@agent-ix/semantic-core` and declares one
model per exported type in neutral vocabulary, together with the emit driver that
compiles it with the official emitter, keeps this module's namespace, absolutizes
every `$id` and `$ref`, and records the toolchain that produced the bytes.

## Subtasks

- [x] `typespec/main.tsp`: `@jsonSchema` base carrying the manifest version, one model per exported type, every grammar item a reference to semantic-core rather than a redeclaration (TC-1416).
- [x] `typespec/tspconfig.yaml`: the official `@typespec/json-schema` emitter, sealed object schemas.
- [x] `scripts/generate-schemas.mjs`: compile into a scratch directory, filter to this module's namespace, absolutize relative references against this module's base, the semantic-core base, or an imported module's base at its exact version, and fail naming a reference that matches none (TC-1457).
- [x] Digest each rendered file and the whole set into `schemas/toolchain.json` alongside the compiler, emitter and semantic-core versions.
- [x] `--check` mode: write nothing, exit non-zero listing every difference (TC-1414).
- [x] Fail naming both values when the `@jsonSchema` base and the manifest version disagree, leaving the committed output untouched (TC-1415).
- [x] Fail carrying the compiler diagnostics when `tsp compile` fails (TC-1449).
- [x] Assert no emitted schema is the placeholder `{type: object}` contract (TC-1409) and that two emissions are byte-identical (TC-1413).

## Deliverables

- A rendered repository whose `make schemas` produces real JSON Schemas from its own source.

## Notes

- The emitter is reached as a pinned dependency and never copied (FR-076-CON-1). A wrong schema is corrected in `main.tsp` and re-emitted, never hand-edited.
