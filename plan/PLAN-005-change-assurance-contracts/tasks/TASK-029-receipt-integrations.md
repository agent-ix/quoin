---
id: TASK-029
title: "Receipt integrations"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-026"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/TASK-028"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1287"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1288"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1289"
    type: "verifies"
---

# TASK-029: Receipt integrations

## Scope

Join retained ix-flow history, impact/unknown state, and FR-032 findings into
the pure evaluator without changing or suppressing their source verdicts.

## Subtasks

- [ ] Exercise absent, duplicate, rejected, revise, mismatched, and broken decisions.
- [ ] Adapt exact FR-032 kinds/ids and all impact/unknown states.
- [ ] Verify inputs are read-only and external commands are never invoked.

## Deliverables

- Retained-input adapters and TC-1287..TC-1289 integration tests.
