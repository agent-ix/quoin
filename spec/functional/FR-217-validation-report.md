---
id: FR-217
title: "Generate a tool validation report from retained runs"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-236"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
---

# FR-217: Generate a tool validation report from retained runs

## Description

The quoin CLI SHALL generate Markdown and JSON validation reports from
retained tool run records, so a customer can review the versioned evidence
collected for the intended use.

## Inputs

- One or more retained run records
- The applicable intended-use and feature-risk-rating records
- The external-tools configuration index reference, when applicable

## Outputs

- `validation-report.md`
- `validation-report.json`

## Behavior

- The command `quoin validation report --results <file>... --out <dir>` SHALL
  write both report formats to the requested directory.
- The report SHALL contain the tool versions as run, the environment, the
  external-tools configuration index reference, the per-case outcomes, and a
  per-FR table with columns `FR`, `Rating`, `Cases`, and `Result`.
- The report SHALL identify the retained run records it consumes.
- Each retained run record SHALL preserve the anomaly records embedded in the
  binary that produced that run, together with whether the set was provided
  and the producer version. The producer version SHALL equal the run record's
  `toolVersion`.
- The report SHALL resolve the `Known anomalies` section from the anomaly sets
  retained with its run records and SHALL NOT substitute anomaly records
  embedded in the report generator's current build.
- When an anomaly set's producer version does not match its run record's
  `toolVersion`, the report SHALL treat that set as `not provided`.
- Quoin SHALL embed only its own `Anomaly` records from `spec/anomalies/` in
  its binary at build time.
- The quoin validation report generator SHALL render the `Known anomalies`
  section from the selection returned by `quoin_anomaly::build_report`,
  preserving FR-162's carried-anomaly selection. Each row SHALL state `Tool`,
  `Anomaly`, `Title`, `Severity`,
  `Safety impact`, `Affected FRs`, and `Status`, using authored values and
  `not stated` where a value is absent. A `safety_impact` of `unassessed` SHALL
  be shown as `unassessed`.
- A row whose assessment targets an FR rated `high_process_risk` SHALL be
  marked `affects high-process-risk feature` and SHALL appear before rows that
  do not target such an FR. Rows within each group SHALL be ordered by anomaly
  id. The section SHALL not report a score.
- When a run record carries an anomaly set and the selection is empty, the
  section SHALL state `none known`.
- When a run record does not carry an anomaly set or its producer version does
  not match, the section SHALL state `not provided`.
- Each tool binary SHALL embed only that tool's own anomaly records. The
  `quire-cli` binary SHALL also embed the `quire-rs` records exposed by its
  linked crate and SHALL NOT embed Quoin's records.
- The report SHALL preserve each case's `pass`, `fail`, `error`, or
  `not-executed` status.
- A `not_high_process_risk` feature MAY rely on retained release test evidence
  only when that evidence is present as a versioned run record.
- If a required run record is absent, the report SHALL state that it was not
  provided. [FR-220](./FR-220-external-tool-configuration-index.md) defines
  how the report resolves external-tool index evidence.
- The report SHALL NOT present missing evidence as a passing result.
- When the report generator receives the same retained run records and rating
  inputs again, it SHALL produce byte-identical Markdown and JSON files.
- The report SHALL contain a `Sign-off` section for a named person's
  statement.
- The report generator SHALL NOT make that statement for the person.
- The report SHALL NOT state that the tool is compliant or accepted for a
  customer's intended use.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-217-AC-1 | Given retained run records for two tools, the command writes valid Markdown and JSON containing each tool version, environment, per-case results, and per-FR coverage rows. | Test |
| FR-217-AC-2 | Given a missing required run record, the report says `not provided` for that evidence and does not mark it as passing. | Test |
| FR-217-AC-3 | Given unchanged retained run records and rating inputs, two report invocations produce byte-identical Markdown and JSON. | Test |
| FR-217-AC-4 | The generated report contains a `Sign-off` section and does not contain a generated acceptance or compliance decision. | Test |
| FR-217-AC-5 | Given a version-matched retained anomaly set with an open anomaly assessed against a `high_process_risk` FR, an open anomaly assessed against a `not_high_process_risk` FR, a closed `fixed` anomaly, and a closed `wont-fix` anomaly, the `Known anomalies` section renders exactly the two open anomalies and the closed `wont-fix` anomaly, highlights the high-risk row first, and omits the closed `fixed` anomaly. | Test |
| FR-217-AC-6 | Given a retained empty anomaly set whose producer version matches its run record, the section states `none known`. | Test |
| FR-217-AC-7 | Given a carried anomaly with `safety_impact: unassessed`, the section renders `unassessed` and does not supply a default value. | Test |
| FR-217-AC-8 | Given a quire-cli binary embedding quire-cli and quire-rs records, the section lists those records and no Quoin records. | Test |
| FR-217-AC-9 | Given a run record with no retained anomaly set or a set whose producer version differs from `toolVersion`, the section states `not provided` and does not use the report generator's build-time set. | Test |

## Dependencies

- **Upstream**: [FR-236](./FR-236-selftest-run-record.md) defines retained results; [FR-030](./FR-030-evidence-store.md) defines the evidence store.
- **Anomaly selection**: [FR-162](./FR-162-anomaly-report.md) defines which anomaly records a release carries; records are defined by `spec-artifacts-process` FR-014, FR-015 and FR-016.
- **Downstream**: [FR-218](./FR-218-validation-report-completeness-and-signoff.md) defines report completeness and signature verification.
