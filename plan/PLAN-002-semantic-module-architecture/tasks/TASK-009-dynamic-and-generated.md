---
id: TASK-009
title: "Record dynamic and generated module compatibility"
type: Task
track: "Architecture records"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-006"
    type: "depends_on"
---

# TASK-009: Record dynamic and generated module compatibility

## Status

**done** — `dynamic-and-generated.md` satisfies TC-1140..TC-1144 without changing current module or package behavior.

## Scope

Create `dynamic-and-generated.md`. Specify open generic module consumption, finite generated package
topology, unknown-extension policy, opt-in regeneration, and the separation of catalog manifests
from exports, targets, mappings, and profiles.

## Exit criteria

- TC-1140..TC-1144 pass.
- Installing a module remains independent of adopting its native generated package.
- Direct typed-Markdown authoring and current dynamic module behavior remain valid.
