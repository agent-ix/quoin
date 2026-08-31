---
id: TASK-009
title: "Build the agent-eval intervention producer"
type: Task
status: done
track: D
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-006"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-007"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-058"
    type: references
  - target: "ix://agent-ix/quoin/TC-1217"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1220"
    type: verifies
---

# TASK-009: Build the agent-eval intervention producer

## Scope

Add a first-party adapter that consumes two retained, versioned real
`cli-agent-evals` reports, derives scenario-aligned observations/effects, computes
raw metadata, and submits the resulting record through TASK-007.

## TDD Work

- Write TC-1217..TC-1220, including generated scenario mismatch and qualifier
  combinations.
- Reuse the existing agent-eval evidence parser instead of introducing a parallel
  report interpretation.
- Force `cause_not_established` and `none` confidence whenever repetition,
  controls, or an attribution method are inadequate.

## Exit Criteria

- The adapter never accepts caller-supplied effects or raw metadata.
- It refuses empty, malformed, unversioned, duplicate, and mismatched reports.
- It invokes no agent, harness, command, subprocess, or network client.
