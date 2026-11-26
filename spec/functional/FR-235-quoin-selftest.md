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
Quoin CLI shall list or run command-level self-test runners associated with
the installed release, so the user can exercise the exact installed binary
without checking out the source repository.

## Inputs

- The installed quoin binary
- The installed command's exact `--version` output
- The target operating system and architecture
- The case selection supplied as zero or more `--case <case-id>` options
- An optional `--out <path>` destination for the run result

## Outputs

- The case list for `quoin selftest --list`
- A per-case run result for `quoin selftest --run`
- A non-zero exit status when any selected case fails or errors

## Behavior

- Each supported release SHALL publish compiled self-test runner files for
  each supported operating-system and architecture target. A runner file SHALL
  be associated with exactly one Quoin release version and target.
- Each release runner SHALL execute a configured subset of existing
  command-level tests from the normal suite. The release configuration SHALL
  identify test IDs already present in that suite; it SHALL NOT define a
  separate set of self-test cases or duplicate their command behavior.
- A test is eligible for the customer runner only when it invokes the installed
  command as a subprocess and its executable dependency closure meets the
  runner asset boundary below. Tests that link to library implementations
  remain internal normal-suite evidence.
- When `quoin selftest --run` is requested, it SHALL select only the runner
  file matching the installed command's exact `--version` output and the
  current target. It SHALL fetch and execute that release asset when the
  matching runner is not already available locally.
- A selected runner SHALL invoke the installed Quoin command through its
  executable path; it SHALL NOT test library behavior by linking to Quoin's
  implementation crates. Tests that link and exercise those libraries remain
  internal normal-suite evidence and SHALL NOT be shipped as customer
  self-test runners.
- Each selected test SHALL invoke the installed command through its executable
  path and SHALL carry its normal-suite requirement trace markers. Its test
  source, input, and expected results SHALL remain in the normal suite.
- A runner SHALL invoke only test IDs present in the release configuration.
  It SHALL NOT materialize a separate case directory or duplicate inputs and
  expected outputs into the release asset.
- When a runner executes a selected test, it SHALL compare the test's recorded
  command outcome using the normal-suite assertion.
- Test runner assets SHALL be standalone command-invoking harnesses.
- Test runner assets SHALL NOT link private library implementation code.
- Test runner assets SHALL NOT contain private data not already present in the
  installed command.
- When the release asset set is prepared, the release owner SHALL inspect its
  bundled files and dependency closure against this boundary.
- When a case process writes to standard error, the runner shall capture that
  output separately from stdout and include it in the case's FR-236 run-record
  result. The runner shall not compare standard-error output with expected
  output or use it to determine case status.
- When a user runs `quoin selftest --run` without `--case`, the runner SHALL run
  every configured test ID. When one or more `--case <case-id>` options are
  supplied, it SHALL run only the named configured tests once each, in
  release-configuration order. When a requested test ID is absent from the
  release configuration, the command SHALL return an error naming that ID
  before any test starts.
- Each case process shall have a wall-clock limit of 60 seconds. When a case
  exceeds that limit, the runner shall terminate and reap the case process and
  its descendants, record that case as `error` with detail identifying
  `timeout`, and continue with the remaining selected cases.
- When `quoin selftest --run` is requested on Linux for a supported target, the
  command SHALL execute the selected cases using the matching release runner.
- When `quoin selftest --run` is requested on macOS, Windows, or another
  unsupported operating system, the command SHALL return an
  `unsupported-platform` error before starting a case.
- When the target is unsupported, the matching release runner is absent, or
  the asset cannot be fetched, the command SHALL return an error before a case
  starts.
- When no matching runner is available, the command SHALL NOT report a case as
  passed. `quoin selftest --list` MAY list supported release-runner targets
  without implying that an unavailable target can run locally.
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
- The read-only `quoin validation ratings --coverage` check joins applicable
  supplier `FeatureRiskRating` values to local self-test `# Trace:` markers.
  For each feature, it SHALL report its rating state and values separately from
  whether a local self-test marker names one of its existing acceptance
  criteria. The self-test coverage value SHALL be `covered` when one or more
  valid markers name its criteria and `not_in_selftest` otherwise. This value
  is informational: it SHALL NOT depend on the feature's rating, make any
  feature require a dedicated self-test case, or cause a non-zero exit because
  no local marker exists. A feature without an applicable rating remains
  `unrated` in the separate rating state. A marker with malformed syntax or a
  reference to a nonexistent local feature or acceptance criterion is reported
  as invalid.
- The check reports the rating values and links it reads; it does not change
  rating acceptance state or produce retained-run evidence. Retained run
  evidence remains governed by [FR-236](./FR-236-selftest-run-record.md).
- Quoin owns the `quoin selftest` entry point and its runner assets. A related
  CLI may provide a corresponding self-test entry point for its own command.
  A release may also provide runner assets for a linked engine when those
  runners invoke the installed command; the CLI SHALL NOT copy or maintain the
  linked engine's source cases. Listing or running a linked-engine case is
  test evidence only; it SHALL NOT create or imply an accepted
  `FeatureRiskRating`. Ratings are accepted only through the named-person
  workflow defined for the validation kit. The validation report's retained
  producer evidence is specified by
  [FR-219](./FR-219-validation-report-tool-evidence.md).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-235-CON-1 | Whenever the runner selects a self-test asset, it uses only the release runner whose version and target match the installed command and host. | Security | Inspection |
| FR-235-CON-2 | While running a self-test, the runner does not check out a source repository or copy case assets between repositories. | Security | Inspection |
| FR-235-CON-3 | Whenever the runner reports a self-test result, the result states test outcomes only. | Compliance | Inspection |
| FR-235-CON-4 | When the runner reports a self-test result, it does not assign a process-risk rating, score, class, or customer-approval decision. | Compliance | Inspection |
| FR-235-CON-5 | A published self-test runner is a standalone command-invoking harness without private library linkage or private data beyond content already present in the installed command. | Security | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-235-AC-1 | Given a listed case and an available runner matching the installed command's exact version and host target, `quoin selftest --run` executes it against the installed binary and records a result for the case. | Test |
| FR-235-AC-2 | Given an expected stdout file that differs from actual stdout by one byte, the case result is `fail` and names the case id and its traced criteria. | Test |
| FR-235-AC-3 | Given a case process that exits non-zero or cannot execute, the result is respectively `fail` or `error`, and the command exits non-zero. | Test |
| FR-235-AC-4 | Given a high-process-risk FR with no local self-test marker, `quoin validation ratings --coverage` reports its rating separately from `selftest_coverage: not_in_selftest`, lists no risk-based uncovered item, and exits 0 when there are no malformed markers or unreadable spec documents. | Test |
| FR-235-AC-5 | Given a normal-suite command-level test with no trace marker, the static requirement matrix reports its criterion as `untagged`. | Test |
| FR-235-AC-6 | Given no `--case` option, `quoin selftest --run` runs every configured normal-suite test ID; given repeated `--case <id>` options, it runs only the named configured tests once each in release-configuration order; an unknown ID is rejected before any test starts and is named in the error. | Test |
| FR-235-AC-7 | Given a case that exceeds 60 seconds, the runner terminates and reaps its process and descendants, records `error` with timeout detail, continues to the next selected case, and exits non-zero. | Test |
| FR-235-AC-8 | Given `--out <path>`, the command writes the retained FR-236 run record's exact JSON bytes to that path and writes no run-result JSON to stdout; without `--out` and with a repository evidence store, it writes those bytes to stdout; without a repository evidence store it rejects before starting any case. | Test |
| FR-235-AC-9 | Given a case owned by a linked engine and invoked through the installed Quoin command, its source and trace tags remain with that engine and the result reports test outcomes only; it does not create or imply an accepted `FeatureRiskRating`. | Inspection |
| FR-235-AC-10 | Given `quoin selftest --run` on Linux, a matching version-and-target runner is used; when the version or target has no release runner, or the runner cannot be fetched, execution returns an error before a case starts and reports no case as passed. Given macOS, Windows, or another unsupported operating system, it returns `unsupported-platform` before a case starts. | Test |
| FR-235-AC-11 | Given `quoin validation ratings --coverage`, a not-high-process-risk or unrated FR without a local self-test marker has `selftest_coverage: not_in_selftest` without failing; an unrated FR remains unrated in its separate rating state; malformed or unknown `# Trace:` markers are reported as invalid and cause a non-zero exit. | Test |
| FR-235-AC-12 | Given a published self-test runner, its executable and declared runtime files contain no private library linkage or private data beyond content already present in the installed command; the normal-suite library test harness is not published as a customer self-test runner. | Inspection |

## Dependencies

- **Upstream**: [FR-112](./FR-112-npm-distribution-from-release-assets.md) defines the shipped quoin binary; [FR-030](./FR-030-evidence-store.md) defines the evidence store used for retained results.
- **Downstream**: [FR-236](./FR-236-selftest-run-record.md) records each run; [FR-217](./FR-217-validation-report.md) presents its results.
- [PLAT-1640](https://linear.app/agent-ix/issue/PLAT-1640) owns the cross-repository `quire matrix --strict` check over each tool repository's self-test source. Quoin owns `quoin validation ratings --coverage` and its `quoin selftest` entry point; each related CLI owns its own command-level runner assets.
