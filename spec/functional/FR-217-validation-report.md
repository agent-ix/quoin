---
id: FR-217
title: "Generate a tool validation report from retained runs"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-216"
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
- The report SHALL preserve each case's `pass`, `fail`, `error`, or
  `not-executed` status.
- A `not_high_process_risk` feature MAY rely on retained release test evidence
  only when that evidence is present as a versioned run record.
- If a required run record or configuration index is absent, the report SHALL
  state that it was not provided.
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
| FR-217-AC-2 | Given a missing run record or external-tools configuration index, the report says `not provided` for that evidence and does not mark it as passing. | Test |
| FR-217-AC-3 | Given unchanged retained run records and rating inputs, two report invocations produce byte-identical Markdown and JSON. | Test |
| FR-217-AC-4 | The generated report contains a `Sign-off` section and does not contain a generated acceptance or compliance decision. | Test |

## Dependencies

- **Upstream**: [FR-216](./FR-216-selftest-run-record.md) defines retained results; [FR-030](./FR-030-evidence-store.md) defines the evidence store.
- **Downstream**: [FR-218](./FR-218-validation-report-completeness-and-signoff.md) defines report completeness and signature verification.
