---
id: FR-235
title: "Run quoin self-tests against the installed binary"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-112"
    type: "extends"
---

# FR-235: Run quoin self-tests against the installed binary

## Description

When a user requests `quoin selftest --list` or `quoin selftest --run`, the
Quoin CLI shall list or run the self-test cases shipped with its installed
binary, so the user can verify that binary in its installation environment
without checking out the source repository.

## Inputs

- The installed quoin binary
- The case selection supplied as zero or more `--case <case-id>` options
- An optional `--out <path>` destination for the run result

## Outputs

- The case list for `quoin selftest --list`
- A per-case run result for `quoin selftest --run`
- A non-zero exit status when any selected case fails or errors

## Behavior

- Each time a case is embedded in the Quoin binary, it shall originate from a
  directory under `selftest/cases/<case-id>/` in Quoin's source repository,
  containing `run.sh`, `input/`, and `expected/` entries.
- Each time a case is embedded, its `expected/` directory shall contain
  `exit_code`, `stdout`, and any expected files the case checks.
- When a case script runs, it shall read the Quoin binary path from `$QUOIN`,
  use only `$CASE_TMP` for files it creates, and contain at least one trace
  marker identifying a requirement acceptance criterion.
- When the runner executes a case, it shall compare the case's exit code,
  stdout bytes, and each expected file byte for byte.
- When a case process writes to standard error, the runner shall capture that
  output separately from stdout and include it in the case's FR-236 run-record
  result. The runner shall not compare standard-error output with expected
  output or use it to determine case status.
- When a user runs `quoin selftest --run` without `--case`, the runner shall run
  every embedded case. When one or more `--case <case-id>` options are
  supplied, it shall run only the named cases once each, in embedded
  declaration order. Before starting any case, it shall reject an unknown case
  id with an error that names the id.
- Each case process shall have a wall-clock limit of 60 seconds. When a case
  exceeds that limit, the runner shall terminate and reap the case process and
  its descendants, record that case as `error` with detail identifying
  `timeout`, and continue with the remaining selected cases.
- When `quoin selftest --run` is requested on Linux, the runner shall execute
  the selected cases. When it is requested on macOS, Windows, or another
  unsupported operating system, the command shall return an
  `unsupported-platform` error before starting any case and shall not report a
  case as passed. On an unsupported operating system, `quoin selftest --list`
  may list embedded cases; the listing shall not imply that cases can run
  there.
- When all expected outputs for a case match, the runner labels the case
  `pass`; when an expected output differs, it labels the case `fail`; when
  case execution fails, it labels the case `error`.
- When any case is `fail` or `error`, the runner returns a non-zero exit status
  and retains the result for every case that ran.
- When a self-test run completes, the runner retains the canonical run record
  defined by [FR-236](./FR-236-selftest-run-record.md). When `--out <path>` is
  supplied, it writes those same JSON bytes to that path instead of standard
  output; otherwise, when a repository evidence store is available, it writes
  the JSON bytes to standard output. When no repository evidence store is
  available, it rejects the run before starting any case unless `--out <path>`
  is supplied, as required by FR-236.
- For each high-process-risk feature identified by the applicable
  `FeatureRiskRating` records, at least one self-test case is traced to an
  acceptance criterion of that feature.
- When a feature is rated `not_high_process_risk`, retained test evidence may
  support it only if the evidence record identifies the tool version and
  exists in the tool's evidence store.
- Quoin owns the `quoin selftest` entry point and its cases. The quire-cli
  repository provides the corresponding `quire selftest` entry point for the
  cases it owns. Its binary may embed quire-rs-owned case assets exposed by
  the linked quire-rs crate at build time; quire-cli shall not copy or maintain
  those cases. Listing or running a quire-rs-owned case is test evidence only;
  it shall not create or imply an accepted `FeatureRiskRating`. Ratings are
  accepted only through the named-person workflow defined for the validation
  kit. The validation report's retained engineering-assurance and
  filament-analyze run evidence is specified by
  [FR-219](./FR-219-validation-report-tool-evidence.md).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-235-CON-1 | Whenever the runner materializes a case, it does so from the installed binary's embedded resources. | Security | Inspection |
| FR-235-CON-2 | While materializing cases, the runner does not check out a repository or copy cases from another repository. | Security | Inspection |
| FR-235-CON-3 | Whenever the runner reports a self-test result, the result states test outcomes only. | Compliance | Inspection |
| FR-235-CON-4 | When the runner reports a self-test result, it does not assign a process-risk rating, score, class, or customer-approval decision. | Compliance | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-235-AC-1 | Given a listed case, `quoin selftest --run` executes its script using the installed binary and records a result for the case. | Test |
| FR-235-AC-2 | Given an expected stdout file that differs from actual stdout by one byte, the case result is `fail` and names the case id and its traced criteria. | Test |
| FR-235-AC-3 | Given a case process that exits non-zero or cannot execute, the result is respectively `fail` or `error`, and the command exits non-zero. | Test |
| FR-235-AC-4 | When the PLAT-1640 validation-kit trace-coverage check finds a high-process-risk FR with no self-test case traced to one of its acceptance criteria, it names that FR as uncovered. This is a cross-repository validation-kit check, not a Quoin runner check. | Test |
| FR-235-AC-5 | Given a test case without a trace marker, `quire matrix --strict` over `selftest/` reports the case as untagged. | Test |
| FR-235-AC-6 | Given no `--case` option, `quoin selftest --run` runs every embedded case; given repeated `--case <id>` options, it runs only the named cases once each in embedded declaration order; an unknown id is rejected before any case starts and is named in the error. | Test |
| FR-235-AC-7 | Given a case that exceeds 60 seconds, the runner terminates and reaps its process and descendants, records `error` with timeout detail, continues to the next selected case, and exits non-zero. | Test |
| FR-235-AC-8 | Given `--out <path>`, the command writes the retained FR-236 run record's exact JSON bytes to that path and writes no run-result JSON to stdout; without `--out` and with a repository evidence store, it writes those bytes to stdout; without a repository evidence store it rejects before starting any case. | Test |
| FR-235-AC-9 | Given a quire-rs-owned case embedded by quire-cli, its source and trace tags remain in quire-rs and the listed case is evidence of test outcomes only; it does not create or imply an accepted `FeatureRiskRating`. | Inspection |
| FR-235-AC-10 | Given `quoin selftest --run` on Linux, the runner executes selected cases; given macOS, Windows, or another unsupported operating system, it returns `unsupported-platform` before starting any case and reports no case as passed. Listing embedded cases on an unsupported operating system does not report execution success. | Test |

## Dependencies

- **Upstream**: [FR-112](./FR-112-npm-distribution-from-release-assets.md) defines the shipped quoin binary; [FR-030](./FR-030-evidence-store.md) defines the evidence store used for retained results.
- **Downstream**: [FR-236](./FR-236-selftest-run-record.md) records each run; [FR-217](./FR-217-validation-report.md) presents its results.
- [PLAT-1640](https://linear.app/agent-ix/issue/PLAT-1640) is the cross-repository validation-kit follow-up for the FR-235-AC-4 coverage check; Quoin owns its `quoin selftest` entry point, while quire-cli owns the corresponding `quire selftest` entry point for its cases.
