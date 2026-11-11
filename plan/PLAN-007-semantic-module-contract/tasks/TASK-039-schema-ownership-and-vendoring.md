---
id: TASK-039
title: "Schema ownership and published packages"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-038"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-070"
    type: references
  - target: "ix://agent-ix/quoin/FR-073"
    type: references
  - target: "ix://agent-ix/quoin/TC-1342"
    type: verifies
---

# TASK-039: Schema ownership and published packages

## Scope

Read the module-manifest schema (with its `semantic` block) and the semantic-core 0.1.0 JSON Schema bundle from their published npm packages at the lockfile versions.

## Subtasks

- [x] Consume `@agent-ix/semantic-schema` and `@agent-ix/semantic-core` from the published packages at the versions the lockfile records; no copy and no recorded hash (TC-1343 and TC-1385 withdrawn).
- [x] Schema-diff test asserting `required` arrays unchanged versus the schema before the `semantic` block (TC-1342).

## Deliverables

- Both schemas consumed from the published packages at lockfile versions.

## Notes

- The module-manifest schema is owned and published upstream as `@agent-ix/semantic-schema`.
