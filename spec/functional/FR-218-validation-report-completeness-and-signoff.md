---
id: FR-218
title: "Report validation coverage and verify a person's sign-off"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-217"
    type: "depends_on"
---

# FR-218: Report validation coverage and verify a person's sign-off

## Description

The validation report SHALL distinguish complete evidence from incomplete
evidence and SHALL let a named person sign the report, so evidence gaps remain
visible and acceptance remains a human decision.

## Behavior

- The report status SHALL be `complete` only when each high-process-risk FR has
  at least one passing case and every required tool run is present.
- The report status SHALL be `incomplete` when a high-process-risk FR has no
  passing case, a case failed or errored, a required tool result is missing,
  or required evidence was not executed.
- The report generator SHALL write a report with `incomplete` status.
- An incomplete report SHALL name each uncovered FR, failed case, errored case,
  missing tool result, and not-executed tool.
- The command SHALL exit 1 when the report is incomplete.
- The command SHALL exit 0 when the report is complete.
- A person SHALL accept the validation record by committing the report with
  `git commit -S` and the trailer
  `Signed-off-meaning: tool-validation-accepted`.
- The sign-off verifier SHALL call Engineering Assurance `signature-verify`
  on the commit that added the report when invoked as
  `--verify-signoff --repo <dir> --allowed-signers <file>`.
- The sign-off verifier SHALL report whether the signature is valid and mapped,
  whether the principal matches `rated_by`, and whether the required meaning
  trailer is present.
- Sign-off verification SHALL report `not_accepted` with a reason when the
  signature is unsigned, invalid, unmapped, attributed to a different
  principal, or missing the required trailer.
- Sign-off verification SHALL report signature facts only.
- Sign-off verification SHALL NOT declare that the tool meets the customer's
  intended-use criteria.
- The report SHALL NOT store a report digest or commit identifier.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-218-AC-1 | Given all high-process-risk FRs have a passing case and all required tool runs are present, the report is `complete` and the command exits 0. | Test |
| FR-218-AC-2 | Given one high-process-risk FR has no passing case, the report is `incomplete`, names that FR, is still written, and the command exits 1. | Test |
| FR-218-AC-3 | Given a failed case, a missing run, or an unexecuted tool, the report remains written as `incomplete`, names each gap, and exits 1. | Test |
| FR-218-AC-4 | Given a signed report commit with a mapped signer matching `rated_by` and the required meaning trailer, sign-off verification reports the signature facts as valid. | Test |
| FR-218-AC-5 | Given an unsigned commit, unmapped signer, signer different from `rated_by`, or missing trailer, sign-off verification reports `not_accepted` and names the reason. | Test |
| FR-218-AC-6 | The report contains neither a report digest nor a commit identifier and does not decide customer acceptability. | Test |

## Dependencies

- **Upstream**: [FR-217](./FR-217-validation-report.md) defines the report; Engineering Assurance FR-036 and FR-037 define `signature-verify` behavior.
- **Downstream**: PLAT-1639 implements the self-test runner and report.
