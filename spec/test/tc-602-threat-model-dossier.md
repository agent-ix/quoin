---
id: TC-602
title: "FR-202 AC-2 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-602: FR-202 AC-2 threat-model dossier behavior

## Description

Verify FR-202-AC-2: Removing `THREAT-1`'s `threatens` edge yields a `GAP: crossing-without-threat CONN-1` row under `### Open findings` and one `crossing_without_threat` gap naming `CONN-1`; both appear in `--json` in the same order.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_only_engine_findings_with_severity` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
