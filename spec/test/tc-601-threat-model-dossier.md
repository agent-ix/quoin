---
id: TC-601
title: "FR-202 AC-1 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-601: FR-202 AC-1 threat-model dossier behavior

## Description

Verify FR-202-AC-1: A fixture with a `threat_model` covering two connections, one carrying `CLASS-PHI` across `BOUND-NET` with `THREAT-1` threatening it, mitigated by `CONTROL-1` and with an accepted risk, lists both connections in `### Covered crossings` with the carried classification, the boundary and trust level, `THREAT-1 (<stride_category>)`, the control and its safeguards, and `RISK-1: accepted, A. Person`.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_authored_facts_without_inference` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
