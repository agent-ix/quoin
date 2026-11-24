---
id: FR-224
title: "State the tool-confidence evidence in an assurance case"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-040"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-217"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-216"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-219"
    type: "requires"
  - target: "ix://agent-ix/spec-artifacts-process/FR-024"
    type: "requires"
---

# FR-224: State the tool-confidence evidence in an assurance case

## Description

When an assurance-case leaf relies on output from a tool represented in the
Toolchain Validation Kit, `quoin assurance` SHALL include a tool-confidence
strategy that identifies the tool's stated intended use, retained self-test
run, and known anomaly records. The strategy SHALL report these source facts
without calculating or asserting a confidence score, qualification, or
compliance verdict.

## Inputs

- The evidence leaf's declared producing tool identity.
- The tool's `IntendedUse` record, retained validation report and referenced
  self-test run records, and typed per-tool `Anomaly` records. The initial
  tool set includes quoin, quire-rs, quire-cli, engineering-assurance, and
  filament-analyze.

## Outputs

- A tool-confidence strategy and references to the source records used to
  populate it.
- An open sub-goal with a specific missing-source reason when a required
  source is unavailable or cannot be linked to the producing tool.

## Behavior

- The strategy SHALL identify each producing tool by the package identity in
  the Toolchain Validation Kit record and SHALL retain the source record ids
  and reported tool versions.
- The strategy SHALL show the tool's declared purpose and limits from its
  `IntendedUse`, the retained outcome and identity of the relevant self-test
  run, and every applicable per-tool `Anomaly` record in the known-anomaly
  list. It SHALL use the existing Anomaly type defined by
  [FR-014](ix://agent-ix/spec-artifacts-process/FR-014),
  [FR-015](ix://agent-ix/spec-artifacts-process/FR-015), and
  [FR-016](ix://agent-ix/spec-artifacts-process/FR-016), and the existing
  carry-selection behavior in [FR-162](./FR-162-anomaly-report.md) and
  authored release rationale in
  [FR-163](./FR-163-anomaly-report-release-rationale.md); it SHALL NOT define
  a parallel anomaly record type. It SHALL NOT read or calculate the
  Git-derived change-attribution fields from those reports.
- A missing intended-use record, self-test run, validation-report reference,
  or anomaly inventory SHALL be stated as an open sub-goal. Absence SHALL NOT
  be interpreted as a passing result or as absence of anomalies.
- The strategy SHALL show recorded status and anomaly facts as source-owned
  values. It SHALL NOT infer trustworthiness from them, assign a confidence
  level, or change the assurance case's existing auditor verdict.
- A confidence strategy SHALL appear only when the input case identifies a
  producing tool; it SHALL NOT guess tool identity from a Git remote,
  executable path, or host state.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-224-AC-1 | A fixture with an identified evidence-producing tool emits a strategy linked to its IntendedUse, retained self-test run, validation report, and known anomaly records, preserving source ids and tool versions. | Test |
| FR-224-AC-2 | An absent or unlinked required record emits an open sub-goal naming the missing record kind and never emits a passing confidence result. | Test |
| FR-224-AC-3 | Changing a source anomaly or self-test record changes only the displayed source facts and does not change the auditor verdict or create a confidence score. | Test |
| FR-224-AC-4 | A case with no identified producing tool emits no inferred tool-confidence node. | Test |

## Dependencies

- **Upstream**: [FR-216](./FR-216-selftest-run-record.md), [FR-217](./FR-217-validation-report.md), [FR-219](./FR-219-validation-report-tool-evidence.md), [FR-220](./FR-220-external-tool-configuration-index.md)
- **Downstream**: [PLAT-1447](https://linear.app/agent-ix/issue/PLAT-1447), [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647)
