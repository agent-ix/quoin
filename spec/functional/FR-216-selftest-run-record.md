---
id: FR-216
title: "Retain versioned self-test run records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
---

# FR-216: Retain versioned self-test run records

## Description

The tool validation kit SHALL retain one run record for each self-test or
real-tool test run, so a later report can identify what ran and what each case
found.

## Inputs

- Tool name and version output
- Operating system and architecture
- Run timestamp and suite name
- Per-case outcome, traced acceptance criteria, and detail
- External-tool configuration index reference, when the run uses external tools

## Outputs

- A canonical JSON run record under `spec/evidence/runs/<suite>/<timestamp>.json`
  when a repository evidence store is available, or at an explicit `--out`
  path when no repository evidence store is available
- A link from each run record to the tool-validation report that consumes it

## Behavior

- A run record SHALL contain the tool name, the exact version output observed
  during the run, the operating system, the architecture, the suite name, the
  run timestamp, and the result for each case.
- A case result SHALL contain its id, `pass`, `fail`, `error`, or `not-executed`
  status, traced acceptance criteria, and a detail field explaining a failure,
  error, or non-execution.
- The recorder SHALL retain a run record when a suite passes, fails, errors,
  or is not executed because a required tool is unavailable.
- When no repository evidence store is available, `--out <path>` SHALL be
  required and SHALL be the retained record location. The runner SHALL reject
  a run without `--out` before starting any case when no repository evidence
  store is available; it SHALL NOT silently emit a stdout-only, unretained
  result.
- The run record SHALL identify the retained evidence location.
- The run record SHALL NOT claim an unavailable or unexecuted tool passed.
- The run record SHALL carry the external-tools configuration index reference
  defined by PLAT-1641 when any listed external tool participates in the run.
- The run record SHALL NOT store a source commit digest, a source revision pin,
  or a report-content digest.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-216-AC-1 | A successful run record contains the tool name and version output, OS, architecture, suite, timestamp, and every case result. | Test |
| FR-216-AC-2 | A failed, errored, or not-executed run is retained with the corresponding status and detail. | Test |
| FR-216-AC-3 | A run involving an external tool records the PLAT-1641 configuration index reference used by the run. | Test |
| FR-216-AC-4 | A serialized run record contains no source commit digest, source revision pin, or report-content digest. | Test |
| FR-216-AC-5 | Given no repository evidence store, `selftest --run --out <path>` retains the canonical run record at that path; without `--out`, the runner reports the missing retained destination and starts no case. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) defines canonical retained run records; PLAT-1641 owns the external-tools configuration index.
- **Downstream**: [FR-217](./FR-217-validation-report.md) consumes retained run records.
