---
id: TASK-007
title: "Record semantic planes and authority by concern"
type: Task
track: "Architecture records"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-006"
    type: "depends_on"
---

# TASK-007: Record semantic planes and authority by concern

## Status

**done** — `index.md` and `planes-and-authority.md` satisfy TC-1125..TC-1128 and TC-1134..TC-1139.

## Scope

Create the architecture index plus `planes-and-authority.md`. Define the four planes, definition
versus occurrence versus presentation, structural-kind/semantic-role independence, the authority
matrix, edit direction, provenance, conflict stopping rule, best-fit wire/analytical projections,
and the exact TypeSpec/JSON Schema/Avro status.

## Exit criteria

- TC-1125..TC-1128 and TC-1134..TC-1139 pass.
- The record does not present generated code, reports, or analytical exports as independent authority.
- TypeSpec remains unpromoted and current Avro compatibility remains intact.
