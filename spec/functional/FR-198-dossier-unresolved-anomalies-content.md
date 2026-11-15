---
id: FR-198
title: "The dossier unresolved-anomalies document renders carried anomaly records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-163"
    type: "references"
---

# FR-198: The dossier unresolved-anomalies document renders carried anomaly records

## Description

When `quoin dossier generate` selects `anomalies` (document id `DOS-ANOM`),
the unresolved-anomalies document SHALL render exactly the carried anomalies
selected by the [FR-162](./FR-162-anomaly-report.md) source report and their
authored [FR-163](./FR-163-anomaly-report-release-rationale.md) rationale.

## Inputs

- `quoin anomaly report [<scope>] --json`, built by
  `quoin_anomaly::build_report` in
  `rust/crates/quoin-anomaly/src/report.rs`. The generator SHALL call that
  library function and SHALL NOT reimplement FR-162's selection rules.
- The `Anomaly` records and their status, safety assessment, affected targets,
  release rationale, and `rationaleChange` as the source reports them.

## Outputs

- `dossier/DOS-ANOM.md`: `# Unresolved anomalies`, `## By hazard`,
  `## By requirement`, one `### <id>: <title>` per carried anomaly under
  `## Anomalies`, and the final `## Gaps` section.
- An anomaly entry shows source status, resolution, severity, safety impact,
  assessment rows, affected targets and release rationale with its attribution.
  Missing optional values read `not stated`.

## Behavior

- The document SHALL preserve FR-162's carried-anomaly selection and order,
  listing closed `wont-fix` anomalies and omitting closed `fixed` anomalies
  when the source does.
- The hazard and requirement sections SHALL reproduce the source `byHazard`
  and `byRequirement` groups, keeping a criterion target separate from its
  parent requirement target.
- Each carried anomaly with `safetyImpact: unassessed` SHALL produce an
  `anomaly_unassessed` gap naming it.
- When an impacting carried anomaly has empty source `releaseRationale`, the
  document SHALL produce an `anomaly_no_release_rationale` gap naming it.
- The document SHALL copy a supplied rationale's author, text and
  `rationaleChange` values exactly as the source reports them.
- When rationale is missing, the document SHALL display `none` without a
  suggested rationale, placeholder sentence or release decision.
- When a source document is unreadable, the command SHALL report a source
  failure under FR-191 rather than silently omitting it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-198-AC-1 | A fixture with open, resolved, closed `wont-fix` and closed `fixed` anomalies yields exactly the source's carried rows; the closed `fixed` row is absent. | Test |
| FR-198-AC-2 | An unassessed carried anomaly yields `anomaly_unassessed`; an assessed anomaly does not. | Test |
| FR-198-AC-3 | An impacting carried anomaly with empty `releaseRationale` yields `anomaly_no_release_rationale`, displays `none`, and contains no suggested wording. | Test |
| FR-198-AC-4 | An authored rationale's author, text and `rationaleChange` fields appear byte-equal to the source. | Test |
| FR-198-AC-5 | A hazard target and a criterion target appear in separate source-matching groups; an anomaly appears once in the detailed list. | Test |
| FR-198-AC-6 | An unreadable source document gives exit 3 with its source error, and no selected dossier file is written. | Test |
| FR-198-AC-7 | Two runs over the same records and source result produce byte-identical Markdown. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-162](./FR-162-anomaly-report.md), and
  [FR-163](./FR-163-anomaly-report-release-rationale.md).
- **Downstream**: PLAT-1619 implements this document.
