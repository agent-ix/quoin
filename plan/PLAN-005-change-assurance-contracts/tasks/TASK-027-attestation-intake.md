---
id: TASK-027
title: "Atomic attestation intake"
type: Task
status: not_started
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-025"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1272"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1273"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1274"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1275"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1276"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1277"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1278"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1279"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1280"
    type: "verifies"
---

# TASK-027: Atomic attestation intake

## Scope

Validate raw attestations and exact output, then retain both as one
content-addressed, crash-atomic directory without changing FR-030 storage.

## Subtasks

- [ ] Write result-state, missing-field, digest, idempotence, and collision tests.
- [ ] Inject failures before the atomic rename and verify cleanup/recovery.
- [ ] Implement the isolated versioned attestation store family.

## Deliverables

- Attestation intake/store API and TC-1272..TC-1280 tests.
