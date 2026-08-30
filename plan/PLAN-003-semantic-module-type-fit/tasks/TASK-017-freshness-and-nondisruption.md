---
id: TASK-017
title: "Prove freshness, reproducibility, and non-disruption"
type: Task
track: "Assurance gate"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/NFR-015"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-016"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-016"
    type: "depends_on"
---

# TASK-017: Prove freshness, reproducibility, and non-disruption

## Status

**done** — the fixed-timestamp audit reruns byte-identically, input/source trees remain untouched,
and the path guard rejects production, module, schema, generated, migration, and consumer changes.

## Scope

Run the retained 10-module audit, repeat it for byte identity, validate every count and digest, run
the fresh-census comparison, prove read-only input trees, and enforce the changed-path allowlist.

## Exit criteria

- TC-1187..TC-1193 pass.
- The retained artifact set validates and an equal-input rerun is byte-identical.
- No module, registry, production, schema, skeleton, generated, migration, or consumer path changes.
