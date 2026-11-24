---
id: FR-220
title: "Record external tool versions in the validation report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-217"
    type: "depends_on"
---

# FR-220: Record external tool versions in the validation report

## Description

The validation report SHALL identify external tools used by each retained run
and their observed versions, so a customer can interpret tool results in the
configuration in which they were produced.

## Behavior

- The report SHALL reference the external-tools configuration index defined by
  PLAT-1641.
- The index SHALL cover git, ssh, gitsign, Syft, OSV-Scanner, pandoc/typst,
  Z3, and cvc5.
- For each listed tool used by a run, the report SHALL show the version
  observed during that run.
- A run that requires an external tool that is unavailable SHALL record the
  tool as `not-executed`; the report SHALL NOT count it as a pass.
- The validation report SHALL include versioned retained runs from
  engineering-assurance and filament-analyze when those tools appear in the
  applicable intended-use records.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-220-AC-1 | Given a run that uses git, ssh, gitsign, Syft, OSV-Scanner, pandoc/typst, Z3, or cvc5, the report lists the used tool and its observed version from the PLAT-1641 index. | Test |
| FR-220-AC-2 | Given an unavailable required external tool, the result is `not-executed` and the report does not present it as passing. | Test |
| FR-220-AC-3 | Given retained engineering-assurance and filament-analyze runs linked from their intended-use records, the report includes both tool names, versions, and outcomes. | Test |

## Dependencies

- **Upstream**: PLAT-1641 defines the external-tools configuration index; PLAT-1817 records local real-tool runs.
- **Downstream**: [FR-217](./FR-217-validation-report.md) presents the indexed tool data.
