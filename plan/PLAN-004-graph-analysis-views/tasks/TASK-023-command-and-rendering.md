---
id: TASK-023
title: "Expose graph commands and deterministic rendering"
type: Task
status: not_started
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

- [ ] Implement subcommand flags, seed validation, repeated relation selection, and export input.
- [ ] Render exact source/module premises, report state, gaps, relation selection, and view rows.
- [ ] Prove equivalent input permutations produce byte-identical canonical JSON.
- [ ] Add a real oclif command-path test without executing Quire or a producer.

## Deliverables

- Registered graph command surface and renderers.
- TC-1258 property/integration coverage.

## Notes

- Human output is a projection of the report object, not a second computation.
