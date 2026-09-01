---
id: TASK-023
title: "Expose graph commands and deterministic rendering"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-020"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-021"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-022"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
  - target: "ix://agent-ix/quoin/TC-1258"
    type: verifies
---

# TASK-023: Expose graph commands and deterministic rendering

## Scope

Expose `quoin graph fan-out`, `quoin graph change-impact`, and `quoin graph churn`, with `--json`
and human output consuming the same sorted `GraphAnalysisReport` model.

## Subtasks

- [x] Implement subcommand flags, seed validation, repeated relation selection, and required
      `--export <json>`/`--premises <json>`/`--audit <json>` inputs; keep `--repo` limited to
      retained Quoin state.
- [x] Render exact source/module premises, report state, gaps, relation selection, and view rows.
- [x] Prove equivalent input permutations produce byte-identical canonical JSON.
- [x] Add real oclif command-path tests without executing Quire or a producer.

## Deliverables

- Registered graph command surface and renderers.
- TC-1258 property/integration coverage.

## Notes

- Human output is a projection of the report object, not a second computation.
