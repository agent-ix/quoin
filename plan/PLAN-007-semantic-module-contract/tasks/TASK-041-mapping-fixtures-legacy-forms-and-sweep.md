---
id: TASK-041
title: "Mapping fixtures, legacy forms, and sweep"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-039"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-071"
    type: references
  - target: "ix://agent-ix/quoin/FR-072"
    type: references
  - target: "ix://agent-ix/quoin/FR-074"
    type: references
---

# TASK-041: Mapping fixtures, legacy forms, and sweep

## Scope

Publish the golden mapping fixtures quire-rs#388 implements, the legacy-form fixtures, and `quoin semantic sweep` with its report schema.

## Subtasks

- [x] `tests/fixtures/semantic-module/`: FR-006 typed-table and `sysml` fence artifacts with expected normalized `FieldDecl[]` (identical), both-forms, type/multiplicity/constraint cell cases, subset violations, reader-rule row (TC-1344..1352, TC-1384); Invariants/Operations cases (TC-1353..1359); pinned unmodified FR-006 copy with provenance and bullet-list/mixed cases with expected `semantic.legacy-properties-form` diagnostics (TC-1367, TC-1368, TC-1371).
- [x] Fixture tests: every expected `FieldDecl`/`ClauseRef`/`OperationDecl` validates against the published semantic-core schemas; table and fence expectations are byte-identical; expected diagnostics carry loci.
- [x] `src/semantic/sweep.ts` + `quoin semantic sweep`: walk a corpus root, classify Properties forms, emit the FR-074 report; sweep-report schema; promotion guard test (TC-1369, TC-1386).
- [x] Authoring-pack migration example (TC-1370).

## Deliverables

- Golden fixtures with expected outputs and diagnostics.
- Sweep command and report schema.

## Notes

- Extraction execution against these fixtures is quire-rs#388's exit criterion; record the hand-off in the fixture README.
