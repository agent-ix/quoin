---
id: TASK-011
title: "Implement the operational record family"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/FR-048"
    type: references
  - target: "ix://agent-ix/quoin/TC-1147"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1155"
    type: verifies
---

# TASK-011: Implement the operational record family

## Scope

Add the v1 JSON Schema, TypeScript discriminated types, loader, and semantic
validator for standing-capability and exercise records.

## TDD Work

- Write TC-1147..TC-1155 first, including generated shape, clock, temporal, pin,
  link, adverse-outcome, and raw-evidence combinations.
- Implement deterministic path-addressed findings and derived clock-state checks.
- Export the engine-independent contract through the existing measurement seam.

## Exit Criteria

- Both shapes and the complete control vocabulary validate independently.
- Impossible clocks, pins, links, and mixed/missing shapes are refused.
- No GitHub-specific field enters the core operational record contract.
