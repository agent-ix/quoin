---
id: TASK-039
title: "Schema ownership and vendoring"
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
  - target: "ix://agent-ix/quoin/TC-1343"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1385"
    type: verifies
---

# TASK-039: Schema ownership and vendoring

## Scope

Land the `semantic` block in filament-core-service's module-manifest schema (agent-ix/filament-core-service#21) and read that schema plus the semantic-core 0.1.0 JSON Schema bundle from their published npm packages at the lockfile versions.

## Subtasks

- [x] Open the filament-core-service PR for #21: optional `semantic` object with the ten FR-070 keys, `data_schema` reference form, no new required key; merge it.
- [x] Consume `@agent-ix/semantic-schema` and `@agent-ix/semantic-core` from the published packages at the versions the lockfile records; no copy and no recorded hash (TC-1343 and TC-1385 withdrawn).
- [x] Schema-diff test asserting `required` arrays unchanged versus the pre-#21 copy (TC-1342).

## Deliverables

- Merged filament-core-service#21.
- Both schemas consumed from the published packages at lockfile versions.

## Notes

- Cross-repo delivery: the filament-core-service change is part of landing this ticket.
