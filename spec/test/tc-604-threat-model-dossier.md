---
id: TC-604
title: "FR-202 AC-4 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-604: FR-202 AC-4 threat-model dossier behavior

## Description

Verify FR-202-AC-4: A covered sink reached by a classification and with no threat yields `GAP: classified-sink-without-threat` and one `classified_sink_without_threat` gap naming the sink, and the sink is listed under `### Covered sinks`.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_only_engine_findings_with_severity` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
