---
id: TC-611
title: "FR-202 AC-11 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-611: FR-202 AC-11 threat-model dossier behavior

## Description

Verify FR-202-AC-11: A `threat_model` covering a `part`, a `trust_boundary` and a `connection` lists the part and the boundary by id under `### Scope`, in that order, and adds a crossing row only for the connection.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_scope_lists_parts_then_boundaries_and_only_connection_rows` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
