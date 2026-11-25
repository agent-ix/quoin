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
- The case selection supplied as zero or more `--case <case-id>` options
- An optional `--out <path>` destination for the run result

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
- `quoin selftest --run` SHALL run every embedded case when no `--case` is
  supplied. When one or more `--case <case-id>` options are supplied, it SHALL
  run only the named cases, once each, in embedded declaration order. The
  runner SHALL reject an unknown case id before any case starts, with an error
  that names the id.
- Every case process SHALL have a wall-clock limit of 60 seconds. When a case
  exceeds that limit, the runner SHALL terminate and reap the case process and
  its descendants, record that case as `error` with detail identifying
  `timeout`, and continue with the remaining selected cases.
- `quoin selftest --run` SHALL execute cases on Linux and macOS. On Windows or
  an unknown or otherwise unsupported operating system, the command SHALL return an
  `unsupported-platform` error before starting a case and SHALL NOT report a
  case as passed. `quoin selftest --list` MAY list embedded cases on an
  unsupported operating system; listing SHALL NOT imply that they can run
  there.
- The runner SHALL label a case `pass` only when all expected outputs match.
- The runner SHALL label a case `fail` when an expected output does not match.
- The runner SHALL label a case `error` when case execution fails.
- The runner SHALL return a non-zero exit status when any case is `fail` or
  `error`, while retaining the result for every case that ran.
- The runner SHALL retain the canonical run record defined by
  [FR-216](./FR-216-selftest-run-record.md). When `--out <path>` is supplied,
  it SHALL write those same JSON bytes to that path instead of standard output;
  otherwise it SHALL write the JSON bytes to standard output when a repository
  evidence store is available. When no repository evidence store is available,
  it SHALL reject the run before starting any case unless `--out <path>` is
  supplied, as required by FR-216.
- The high-process-risk features identified by the applicable
  `FeatureRiskRating` records SHALL each have at least one self-test case
  traced to an acceptance criterion of that feature.
- A feature rated `not_high_process_risk` MAY rely on retained test evidence
  only when the evidence record identifies the tool version and the evidence
  exists in the tool's evidence store.
- The quire-cli repository SHALL provide the corresponding `quire selftest`
  entry point for cases it owns. Its binary MAY embed quire-rs-owned case
  assets exposed by the linked quire-rs crate at build time; quire-cli SHALL
  NOT copy or maintain those cases. Listing or running a quire-rs-owned case
  is test evidence only; it SHALL NOT create or imply an accepted
  `FeatureRiskRating`. Ratings are accepted only through the named-person
  workflow defined for the validation kit. The validation report's retained
  engineering-assurance and filament-analyze run evidence is specified by
  [FR-219](./FR-219-validation-report-tool-evidence.md).

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
| FR-215-AC-6 | Given no `--case` option, `quoin selftest --run` runs every embedded case; given repeated `--case <id>` options, it runs only the named cases once each in embedded declaration order; an unknown id is rejected before any case starts and is named in the error. | Test |
| FR-215-AC-7 | Given a case that exceeds 60 seconds, the runner terminates and reaps its process and descendants, records `error` with timeout detail, continues to the next selected case, and exits non-zero. | Test |
| FR-215-AC-8 | Given `--out <path>`, the command writes the retained FR-216 run record's exact JSON bytes to that path and writes no run-result JSON to stdout; without `--out` and with a repository evidence store, it writes those bytes to stdout; without a repository evidence store it rejects before starting any case. | Test |
| FR-215-AC-9 | Given a quire-rs-owned case embedded by quire-cli, its source and trace tags remain in quire-rs and the listed case is evidence of test outcomes only; it does not create or imply an accepted `FeatureRiskRating`. | Inspection |
| FR-215-AC-10 | Given `quoin selftest --run` on Linux or macOS, the runner executes selected cases; given Windows or another unsupported operating system, it returns `unsupported-platform` before starting any case and reports no case as passed. Listing embedded cases on an unsupported operating system does not report execution success. | Test |

## Dependencies

- **Upstream**: [FR-112](./FR-112-npm-distribution-from-release-assets.md) defines the shipped quoin binary; [FR-030](./FR-030-evidence-store.md) defines the evidence store used for retained results.
- **Downstream**: [FR-216](./FR-216-selftest-run-record.md) records each run; [FR-217](./FR-217-validation-report.md) presents its results.
