---
id: FR-236
title: "Retain versioned self-test run records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
---

# FR-236: Retain versioned self-test run records

## Description

When the tool validation kit runs a self-test or real-tool test suite, it shall
retain one run record so a later report can identify what ran and what each
case found.

## Inputs

- Tool name and version output
- Operating system and architecture
- Run timestamp and suite name
- Per-case outcome, traced acceptance criteria, and detail
- External-tool configuration index reference, when the run uses external tools

## Outputs

- A canonical JSON run record at `<evidence-store>/runs/<suite>/<timestamp>.json`,
  where `<evidence-store>` is the nearest ancestor of the current working
  directory containing `spec/evidence/`, or at an explicit `--out` path when
  no repository evidence store is available
- A link from each run record to the tool-validation report that consumes it

## Behavior

- For each suite run, the record shall contain the tool name, the exact version
  output observed during the run, the operating system, the architecture, the
  suite name, the run timestamp, and the result for each case.
- For each case result, the record shall contain its id, `pass`, `fail`,
  `error`, or `not-executed` status, traced acceptance criteria, and a detail
  field explaining a failure, error, or non-execution.
- When a case process writes to standard error, the case result shall contain
  a `stderr` string with that captured output. Before JSON serialization, the
  recorder shall decode the captured bytes as UTF-8 and replace each invalid
  sequence with U+FFFD, making the representation deterministic and JSON-safe.
- The runner shall not compare captured standard-error output with expected
  output or use it to determine case status.
- When a suite passes, fails, errors, or is not executed because a required
  tool is unavailable, the recorder shall retain a run record.
- When no repository evidence store is available, the runner shall require
  `--out <path>` as the retained record location. Before starting any case, it
  shall reject a run without `--out`; it shall not emit a stdout-only,
  unretained result.
- When locating a repository evidence store, the runner shall walk from the
  current working directory to its ancestors and select the nearest directory
  containing `spec/evidence/`. It shall not invoke Git to locate the store.
  When the store or suite directory does not exist, the runner shall create
  `spec/evidence/runs/` and the suite directory.
- Each run record shall identify the retained evidence location and shall not
  claim that an unavailable or unexecuted tool passed.
- When any listed external tool participates in the run, the run record shall
  carry the external-tools configuration index reference defined by
  [PLAT-1641](https://linear.app/agent-ix/issue/PLAT-1641).
- A serialized run record shall not store a source commit digest, a source
  revision pin, or a report-content digest.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-236-AC-1 | A successful run record contains the tool name and version output, OS, architecture, suite, timestamp, and every case result. | Test |
| FR-236-AC-2 | A failed, errored, or not-executed run is retained with the corresponding status and detail. | Test |
| FR-236-AC-3 | When a run involves an external tool, its record contains the PLAT-1641 configuration index reference used by that run. This is a cross-repository validation-kit follow-up; quoin selftest does not invoke external tools. | Test |
| FR-236-AC-4 | A serialized run record contains no source commit digest, source revision pin, or report-content digest. | Test |
| FR-236-AC-5 | Given no repository evidence store, `selftest --run --out <path>` retains the canonical run record at that path; without `--out`, the runner reports the missing retained destination and starts no case. | Test |
| FR-236-AC-6 | Given nested ancestors with `spec/evidence/`, the recorder writes under the nearest one without invoking Git and creates its `runs/<suite>/` directories; when no ancestor has `spec/evidence/`, it uses required `--out`. | Test |
| FR-236-AC-7 | Given two case runs with identical exit code, stdout, and expected files but different stderr bytes, both case statuses are identical and each run record contains its actual stderr as a JSON-safe UTF-8 string, replacing invalid byte sequences with U+FFFD. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) defines canonical retained run records; [PLAT-1641](https://linear.app/agent-ix/issue/PLAT-1641) owns the external-tools configuration index consumed by the cross-repository validation-kit follow-up.
- **Downstream**: [FR-217](./FR-217-validation-report.md) consumes retained run records.
