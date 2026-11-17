---
id: TC-615
title: "FR-202 AC-15 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-615: FR-202 AC-15 threat-model dossier behavior

## Description

Verify FR-202-AC-15: A sink carrying `CLASS-PHI` and one reached by a connection that `delivers_to` it and carries `CLASS-PII` each list that classification; a sink with a control that `protects` it and a control that `mitigates` its threat lists each once with safeguards, and a control doing both is listed once.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_authored_facts_without_inference` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
