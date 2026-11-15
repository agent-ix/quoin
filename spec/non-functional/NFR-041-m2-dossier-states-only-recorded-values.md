---
id: NFR-041
title: "M2 dossier documents state recorded values without deriving verdicts"
type: NFR
quality_attribute: safety
relationships:
  - target: "ix://agent-ix/quoin/FR-196"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-197"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-198"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-199"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-200"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-201"
    type: "constrains"
---

# NFR-041: M2 dossier documents state recorded values without deriving verdicts

## Statement

The six M2 dossier generators SHALL render only source-reported values and
explicit gaps; they SHALL NOT calculate a new score, percentage, rank, software
class, risk acceptability or release verdict.

## Scope

- Applies to Markdown and `--json` reports for `verification-report`, `soup`,
  `anomalies`, `risk-management`, `risk-management-plan` and
  `release-record`, including regeneration before export.
- Source-authored strings and declared fields such as the policy's
  `outside_acceptable` flag remain data and are copied verbatim. They are not
  new decisions by the dossier.
- Extends [NFR-040](./NFR-040-dossier-is-deterministic-and-records-only.md).

## Rationale

A generated document can look like a person's release or risk decision. A
plausible default would conceal missing records and mislead its reviewer.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Generated values without a matching source value or explicit gap | 0 | 0 | Test |
| Newly computed score, percentage, rank, class, acceptability or release verdict fields | 0 | 0 | Inspection |
| Source strings changed by the generator | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-041-AC-1 | A fixture with absent method, support, safety-impact, policy, decider and release-verdict values renders `not stated`, `unknown` or the relevant named gap in all six documents; no output supplies a plausible value. | Test |
| NFR-041-AC-2 | Inspection of the six generated report shapes finds no new score, percentage, rank, software-class, risk-acceptability or release-verdict member. The policy's source `outside_acceptable` field is allowed only as a verbatim policy field. | Inspection |
| NFR-041-AC-3 | Changing a source-authored meaning or policy region string changes the corresponding output by the same bytes and changes no other source value. | Test |
| NFR-041-AC-4 | A `not_released` EA result remains `not_released` in both Markdown and JSON; a risk with no decision never becomes `accepted` or `acceptable`. | Test |

## Verification

Fixture tests compare source data with each generated document and JSON report.
Inspection of the dossier output models checks that no forbidden derived field
is introduced.

## Dependencies

- **Upstream**: [NFR-040](./NFR-040-dossier-is-deterministic-and-records-only.md)
  and [FR-196](../functional/FR-196-dossier-verification-report-content.md)
  through [FR-201](../functional/FR-201-dossier-release-record-content.md).
