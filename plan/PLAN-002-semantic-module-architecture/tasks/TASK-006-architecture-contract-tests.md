---
id: TASK-006
title: "Add red semantic-module architecture contract tests"
type: Task
track: "Test-first foundation"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "references"
---

# TASK-006: Add red semantic-module architecture contract tests

## Status

**done** — the focused red run failed 29 content cases against the intentionally absent architecture directory; TC-1154's scope guard already passed.

## Scope

Create `tests/semantic-module-architecture.test.ts` with exact trace tags for TC-1125..TC-1154.
The suite first fails because the architecture directory does not exist. It checks index/link
integrity, required content and exclusions, external-decision fields, provisional status, and the
allowed changed-path set.

## Exit criteria

- The focused test command fails for missing architecture artifacts rather than a broken harness.
- Every automated matrix case TC-1125..TC-1154 appears exactly once as a trace tag.
- The path guard permits only requirements, plans, architecture docs, tests, and review artifacts.
