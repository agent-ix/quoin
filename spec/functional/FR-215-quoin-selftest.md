---
id: FR-215
title: "Run quoin self-tests against the installed binary"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-112"
    type: "extends"
---

# FR-215: Run quoin self-tests against the installed binary

## Description

The quoin CLI SHALL let a user list and run the self-test cases shipped with
the installed binary, so a user can verify the binary in its own installation
environment without checking out the source repository.

## Inputs

- The installed quoin binary
- An optional output path for the run result
- The case selection, when a user requests selected cases

## Outputs

- The case list for `quoin selftest --list`
- A per-case run result for `quoin selftest --run`
- A non-zero exit status when any selected case fails or errors

## Behavior

- Each quoin case SHALL be a directory under `selftest/cases/<case-id>/` in
  quoin's source repository, with `run.sh`, `input/`, and `expected/` entries.
- The `expected/` directory SHALL contain `exit_code`, `stdout`, and any
  expected files the case checks.
- Each case script SHALL read the quoin binary path from `$QUOIN`, use only
  `$CASE_TMP` for files it creates, and include at least one trace marker that
  identifies a requirement acceptance criterion.
- The self-test runner SHALL compare the case's exit code, stdout bytes, and
  each expected file byte for byte.
- The runner SHALL label a case `pass` only when all expected outputs match.
- The runner SHALL label a case `fail` when an expected output does not match.
- The runner SHALL label a case `error` when case execution fails.
- The runner SHALL return a non-zero exit status when any case is `fail` or
  `error`, while retaining the result for every case that ran.
- The high-process-risk features identified by the applicable
  `FeatureRiskRating` records SHALL each have at least one self-test case
  traced to an acceptance criterion of that feature.
- A feature rated `not_high_process_risk` MAY rely on retained test evidence
  only when the evidence record identifies the tool version and the evidence
  exists in the tool's evidence store.
- The quire-cli repository SHALL provide the corresponding `quire selftest`
  entry point for cases it owns, including quire-rs cases embedded by
  quire-cli. Engineering-assurance and filament-analyze test evidence SHALL
  be included in the validation report under [FR-219](./FR-219-validation-report-tool-evidence.md).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-215-CON-1 | The runner SHALL materialize cases from the installed binary's embedded resources. | Security | Inspection |
| FR-215-CON-2 | The runner SHALL NOT check out a repository or copy cases from another repository. | Security | Inspection |
| FR-215-CON-3 | The self-test result SHALL state test outcomes only. | Compliance | Inspection |
| FR-215-CON-4 | The self-test result SHALL NOT assign a process-risk rating, score, class, or customer-approval decision. | Compliance | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-215-AC-1 | Given a listed case, `quoin selftest --run` executes its script using the installed binary and records a result for the case. | Test |
| FR-215-AC-2 | Given an expected stdout file that differs from actual stdout by one byte, the case result is `fail` and names the case id and its traced criteria. | Test |
| FR-215-AC-3 | Given a case process that exits non-zero or cannot execute, the result is respectively `fail` or `error`, and the command exits non-zero. | Test |
| FR-215-AC-4 | Given a high-process-risk FR without a case traced to one of its acceptance criteria, the coverage check names that FR as uncovered. | Test |
| FR-215-AC-5 | Given a test case without a trace marker, `quire matrix --strict` over `selftest/` reports the case as untagged. | Test |

## Dependencies

- **Upstream**: [FR-112](./FR-112-npm-distribution-from-release-assets.md) defines the shipped quoin binary; [FR-030](./FR-030-evidence-store.md) defines the evidence store used for retained results.
- **Downstream**: [FR-216](./FR-216-selftest-run-record.md) records each run; [FR-217](./FR-217-validation-report.md) presents its results.
