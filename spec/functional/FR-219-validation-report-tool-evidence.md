---
id: FR-219
title: "Include tool anomalies and validation evidence in the report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-162"
    type: "extends"
---

# FR-219: Include tool anomalies and validation evidence in the report

## Description

The validation report SHALL show each participating tool's retained run
evidence and open known anomalies, so the customer can see which results were
observed and which tool limitations remain open.

## Behavior

- The participating tool set SHALL include quoin, quire-cli, quire-rs,
  engineering-assurance, and filament-analyze when those tools appear in the
  applicable intended-use records.
- The report SHALL include retained run records for engineering-assurance and
  filament-analyze produced by their local real-tool test targets.
- Each tool's run result SHALL identify the tool version observed during the
  run. A missing version SHALL make that run incomplete.
- The report SHALL list open Anomaly records carried by each participating
  tool and identify any anomaly whose assessment names a high-process-risk FR.
- If a supplied anomaly set is empty, the report SHALL show `none known`. If
  the binary supplied no anomaly set, the report SHALL show `not provided`.
- A carried anomaly assessed against a high-process-risk FR SHALL be visibly
  highlighted with that FR id.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-219-AC-1 | Given a run record for engineering-assurance and one for filament-analyze, the report includes both tools and their observed versions. | Test |
| FR-219-AC-2 | Given a supplied empty anomaly set, the report says `none known`; given no supplied set, it says `not provided`. | Test |
| FR-219-AC-3 | Given an open anomaly assessed against a high-process-risk FR, the report highlights that anomaly and names the FR. | Test |

## Dependencies

- **Upstream**: [FR-136](./FR-136-anomaly-record-mapping.md) defines anomaly record mapping; [FR-162](./FR-162-anomaly-report.md) defines carried anomaly reporting; PLAT-1817 records real-tool runs.
- **Downstream**: [FR-217](./FR-217-validation-report.md) incorporates retained run evidence and anomaly reporting.
