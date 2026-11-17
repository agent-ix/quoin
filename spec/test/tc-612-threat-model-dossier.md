---
id: TC-612
title: "FR-202 AC-12 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-612: FR-202 AC-12 threat-model dossier behavior

## Description

Verify FR-202-AC-12: With `trace:crossing-without-threat` set to `off`, a connection lacking a threat yields no `GAP:` row and no gap; with it set to `warning`, the row reads `(warning)`, the gap is present and status is `incomplete`.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_only_engine_findings_with_severity` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
