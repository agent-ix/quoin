---
id: TASK-014
title: "Build the GitHub Actions release producer"
type: Task
status: todo
track: D
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-011"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-012"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-051"
    type: references
  - target: "ix://agent-ix/quoin/TC-1173"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1176"
    type: verifies
---

# TASK-014: Build the GitHub Actions release producer

## Scope

Add a first-party adapter for retained workflow YAML plus GitHub workflow-run and
jobs API JSON. It derives and submits one linked `release` capability/exercise pair.

## TDD Work

- Write TC-1173..TC-1176 with retained public fixtures and generated mismatch and
  adverse-conclusion cases.
- Parse workflow structure and uniquely select the configured release job.
- Derive every decision-bearing record field and raw digest from retained bytes;
  treat every non-success conclusion as visible non-discharging evidence.

## Exit Criteria

- Workflow, path, event, revision, run, and job mismatches refuse the full pair.
- Caller-supplied timestamps, conclusions, clock labels, sizes, and digests cannot
  replace source-derived values.
- The adapter contains no GitHub client, dispatch, publication, or process path.
