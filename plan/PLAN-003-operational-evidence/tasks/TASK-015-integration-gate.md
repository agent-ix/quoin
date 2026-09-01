---
id: TASK-015
title: "Close the operational integration gate"
type: Task
status: done
track: Gate
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-012"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-013"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-014"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-060"
    type: references
  - target: "ix://agent-ix/quoin/FR-061"
    type: references
  - target: "ix://agent-ix/quoin/TC-1242"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1243"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1248"
    type: verifies
---

# TASK-015: Close the operational integration gate

## Scope

Prove the no-control boundary, backward compatibility, real release-run ingestion,
and all-or-nothing pair persistence, then close traceability for #271.

## TDD Work

- Add static TC-1242 and compatibility TC-1243 to the normal suite.
- Capture a real release workflow-run/jobs response outside Quoin under the
  content-rights policy and execute TC-1248 entirely offline inside Quoin.
- Inject second-record validation/write failure and prove the pair leaves no entry.
- Run validation, typecheck, lint, tests, coverage, and gap analysis.

## Exit Criteria

- TC-1242, TC-1243, and TC-1248 pass with tracking tags.
- Existing measurement and pre-operational evidence remain readable.
- Gap analysis reports no incomplete task, unbacked matrix row, or unowned code.
