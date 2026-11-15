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
  - target: "ix://agent-ix/quoin/FR-164"
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
  `rust/crates/quoin-anomaly/src/report.rs`. The common dossier `--scope`
  selects the same anomaly corpus. The generator calls that library function
  with FR-164's `unjustified-impacting` mode so the gap uses the source's
  existing rationale rule rather than a second placeholder check.
- The `Anomaly` records and their status, safety assessment, affected targets,
  and authored release rationale as the source reports them. The source's
  git-derived `rationaleChange` and `fixingChange` are not dossier inputs.

## Outputs

- `dossier/DOS-ANOM.md`: `# Unresolved anomalies`, `## By hazard`,
  `## By requirement`, `## Ungrouped`, one `### <id>: <title>` per carried anomaly under
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
- The `Ungrouped` section SHALL reproduce every source `ungrouped` row with
  its `unassessed` or `no-affected-id` reason.
- Each carried anomaly with `safetyImpact: unassessed` SHALL produce an
  `anomaly_unassessed` gap naming it.
- For each source `gate.failures` entry with mode `unjustified-impacting`,
  the document SHALL produce an `anomaly_no_release_rationale` gap naming
  that carried anomaly. The generator SHALL NOT restate FR-164's rationale
  validity or carried-anomaly rules.
- The document SHALL copy a supplied rationale's author, text and
  authored order exactly as the source reports them.
- The document SHALL NOT render `rationaleChange` or `fixingChange`; those
  git-derived fields can change when the default branch advances while the
  committed anomaly record remains the same.
- When rationale is missing, the document SHALL display `none` without a
  suggested rationale, placeholder sentence or release decision.
- When a source document is unreadable, the command SHALL report a source
  failure under FR-191 rather than silently omitting it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-198-AC-1 | A fixture with open, resolved, closed `wont-fix` and closed `fixed` anomalies yields exactly the source's carried rows; the closed `fixed` row is absent. | Test |
| FR-198-AC-2 | An unassessed carried anomaly yields `anomaly_unassessed`; an assessed anomaly does not. | Test |
| FR-198-AC-3 | A carried impacting anomaly that FR-164 reports under `unjustified-impacting` yields `anomaly_no_release_rationale`; an empty or placeholder rationale displays `none` or its authored text without suggested wording. | Test |
| FR-198-AC-4 | An authored rationale's author and text appear byte-equal to the source, in source order; changing only git-derived `rationaleChange` or `fixingChange` leaves the dossier bytes unchanged. | Test |
| FR-198-AC-5 | A hazard target and a criterion target appear in separate source-matching groups; an anomaly appears once in the detailed list. | Test |
| FR-198-AC-6 | An unreadable source document gives exit 3 with its source error, and no selected dossier file is written. | Test |
| FR-198-AC-7 | Two runs over the same records and source result produce byte-identical Markdown. | Test |
| FR-198-AC-8 | Every `ungrouped` source row appears under `Ungrouped` with its exact source reason; no carried anomaly is silently omitted. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-162](./FR-162-anomaly-report.md), and
  [FR-163](./FR-163-anomaly-report-release-rationale.md), and
  [FR-164](./FR-164-anomaly-report-gate.md).
- **Downstream**: PLAT-1619 implements this document.
