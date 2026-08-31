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
  - target: "ix://agent-ix/quoin/FR-059"
    type: references
  - target: "ix://agent-ix/quoin/TC-1223"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1231"
    type: verifies
---

# TASK-011: Implement the operational record family

## Scope

Add one exact shipped v1 JSON Schema, TypeScript discriminated types, loader,
and semantic validator for standing-capability and exercise records.

## TDD Work

- Write TC-1223..TC-1231 first, including generated shape, clock, temporal, pin,
  link, adverse-outcome, and raw-evidence combinations.
- Implement deterministic path-addressed findings and derived clock-state checks.
- Import the shipped schema into the validator and export that exact
  engine-independent contract through the existing measurement seam.

## Exit Criteria

- Both shapes and the complete control vocabulary validate independently.
- Impossible clocks, pins, links, and mixed/missing shapes are refused.
- No GitHub-specific field enters the core operational record contract.
