---
id: TASK-031
title: "Full assurance gate"
type: Task
status: not_started
track: Gate
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-030"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1261"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1292"
    type: "verifies"
---

# TASK-031: Full assurance gate

## Scope

Reconcile every plan task and matrix row, run repository gates, and complete
code review plus mechanical gap analysis before landing preparation.

## Subtasks

- [ ] Prove TC-1261..TC-1292 are backed by real trace tags.
- [ ] Run format, lint, test-with-quire, and compatibility checks.
- [ ] Resolve code-review and gap-analysis findings and normalize plan status.

## Deliverables

- Validated review artifacts, green gates, and a clean landing-ready branch.
