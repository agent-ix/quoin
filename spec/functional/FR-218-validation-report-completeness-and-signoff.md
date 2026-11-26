---
id: FR-218
title: "Report validation coverage and verify a person's sign-off"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-217"
    type: "depends_on"
---

# FR-218: Distinguish report completeness from test outcomes and verify sign-off

## Description

The validation report SHALL distinguish whether its declared release evidence
is complete from whether its tests pass and from whether requirements are
covered. It SHALL let a named person sign the report, so evidence gaps remain
visible and acceptance remains a human decision.

## Behavior

- The evidence status SHALL be `complete` when each declared release-suite run
  and required producer result is present and contains a result for each
  expected test. It SHALL be `incomplete` when a required run or result is
  missing, unreadable, malformed, or not executed.
- The report SHALL show aggregate and per-test outcomes separately from
  evidence status.
- A failed, errored, skipped, or not-executed test result SHALL remain visible
  as a test outcome and SHALL NOT be hidden by evidence status.
- The report SHALL list uncovered requirements from computed requirement
  coverage.
- An uncovered requirement SHALL NOT by itself make evidence status
  `incomplete`.
- A feature-risk rating SHALL NOT define a dedicated test or make evidence
  status `incomplete`.
- Test selection and evidence completeness SHALL NOT depend on
  `FeatureRiskRating` values. Ratings remain descriptive report metadata.
- The report generator SHALL write a report with `incomplete` status.
- An incomplete report SHALL name each missing or unreadable run, missing test
  result, and not-executed tool. Independently, it SHALL list uncovered
  requirements and all failed, errored, skipped, or not-executed tests.
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
| FR-218-AC-1 | Given every declared release-suite run and expected test result is present, the evidence status is `complete` and the command exits 0 even when a test failed or a requirement is uncovered. Test outcome and coverage remain separately reported. | Test |
| FR-218-AC-2 | Given a required run or expected test result is missing, the report is still written with evidence status `incomplete`, names the missing evidence, and the command exits 1. | Test |
| FR-218-AC-3 | Given complete run records with a failing test, an errored test, and an uncovered requirement, the report remains evidence-complete, lists each distinct outcome and the uncovered requirement, and does not infer a test requirement from risk rating. | Test |
| FR-218-AC-4 | Given a signed report commit with a mapped signer matching `rated_by` and the required meaning trailer, sign-off verification reports the signature facts as valid. | Test |
| FR-218-AC-5 | Given an unsigned commit, unmapped signer, signer different from `rated_by`, or missing trailer, sign-off verification reports `not_accepted` and names the reason. | Test |
| FR-218-AC-6 | The report contains neither a report digest nor a commit identifier and does not decide customer acceptability. | Test |

## Dependencies

- **Upstream**: [FR-217](./FR-217-validation-report.md) defines the report; Engineering Assurance FR-036 and FR-037 define `signature-verify` behavior.
- **Downstream**: PLAT-1639 implements the self-test runner and report.
