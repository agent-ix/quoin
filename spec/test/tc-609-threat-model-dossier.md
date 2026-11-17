---
id: TC-609
title: "FR-202 AC-9 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-609: FR-202 AC-9 threat-model dossier behavior

## Description

Verify FR-202-AC-9: An unreadable `threat` record makes `generate` exit 3 naming the source and leaves every existing file unchanged.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_unreadable_threat_record_is_a_source_failure` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
