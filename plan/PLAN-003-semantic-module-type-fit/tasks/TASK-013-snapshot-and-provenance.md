---
id: TASK-013
title: "Implement audit snapshot and provenance"
type: Task
track: "Audit engine"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-051"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-012"
    type: "depends_on"
---

# TASK-013: Implement audit snapshot and provenance

## Status

**done** — exact Git-tree, installed-byte, registry, Quoin, Quire, corpus, and core-data identities
are retained; equal installed/canonical bytes reconcile and all manifest-version disagreements remain findings.

## Scope

Implement canonical paths/digests, Quoin/tool/external evidence identities, ordered module-source
resolution, inspected-content identity, and typed provenance conflicts without mutating checkouts or registries.

## Exit criteria

- TC-1156..TC-1161 pass.
- Equal identities serialize deterministically and conflicts retain both sides.
- Source resolution performs no install, checkout, reset, pull, registry write, or source-tree write.
