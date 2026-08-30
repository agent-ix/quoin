---
id: TASK-010
title: "Close the intervention integration gate"
type: Task
status: done
track: Gate
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-007"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-008"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-009"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-047"
    type: references
  - target: "ix://agent-ix/quoin/FR-050"
    type: references
  - target: "ix://agent-ix/quoin/TC-1144"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1145"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1172"
    type: verifies
---

# TASK-010: Close the intervention integration gate

## Scope

Exercise the completed vertical slice, prove the no-execution boundary and
backward compatibility, and produce the final traceability evidence for #270.

## TDD Work

- Add static TC-1144 and compatibility TC-1145 to the normal test suite.
- Run a real baseline/treatment evaluation outside Quoin, retain the unmodified
  reports under the content-rights policy, and execute TC-1172 through intake and
  report rendering.
- Run validation, typecheck, lint, tests, coverage, and gap analysis.

## Exit Criteria

- TC-1144, TC-1145, and TC-1172 pass with tracking tags.
- Existing measurement and evidence fixtures remain readable without migration.
- The validated gap-analysis artifact reports no unowned requirement or task gap.
