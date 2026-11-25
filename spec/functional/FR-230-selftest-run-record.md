---
id: FR-230
title: "Retain versioned self-test run records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
---

# FR-230: Retain versioned self-test run records

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

- A canonical JSON run record at `<evidence-store>/runs/<suite>/<timestamp>.json`,
  where `<evidence-store>` is the nearest ancestor of the current working
  directory containing `spec/evidence/`, or at an explicit `--out` path when
  no repository evidence store is available
- A link from each run record to the tool-validation report that consumes it

## Behavior

- A run record SHALL contain the tool name, the exact version output observed
  during the run, the operating system, the architecture, the suite name, the
  run timestamp, and the result for each case.
- A case result SHALL contain its id, `pass`, `fail`, `error`, or `not-executed`
  status, traced acceptance criteria, and a detail field explaining a failure,
  error, or non-execution.
- Each case result SHALL contain a `stderr` string with that case process's
  captured standard-error output. The recorder SHALL decode the captured bytes
  as UTF-8 and replace each invalid sequence with U+FFFD before serializing the
  string as JSON, so the representation is deterministic and JSON-safe.
- The runner SHALL NOT compare captured standard-error output with expected
  output or use it to determine the case status.
- The recorder SHALL retain a run record when a suite passes, fails, errors,
  or is not executed because a required tool is unavailable.
- When no repository evidence store is available, `--out <path>` SHALL be
  required and SHALL be the retained record location. The runner SHALL reject
  a run without `--out` before starting any case when no repository evidence
  store is available; it SHALL NOT silently emit a stdout-only, unretained
  result.
- The runner SHALL locate a repository evidence store by walking from the
  current working directory to its ancestors and selecting the nearest
  directory containing `spec/evidence/`. It SHALL NOT invoke Git to locate the
  store. It SHALL create `spec/evidence/runs/` and the suite directory when
  they do not exist.
- The run record SHALL identify the retained evidence location.
- The run record SHALL NOT claim an unavailable or unexecuted tool passed.
- The run record SHALL carry the external-tools configuration index reference
  defined by PLAT-1641 when any listed external tool participates in the run.
- The run record SHALL NOT store a source commit digest, a source revision pin,
  or a report-content digest.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-230-AC-1 | A successful run record contains the tool name and version output, OS, architecture, suite, timestamp, and every case result. | Test |
| FR-230-AC-2 | A failed, errored, or not-executed run is retained with the corresponding status and detail. | Test |
| FR-230-AC-3 | A run involving an external tool records the PLAT-1641 configuration index reference used by the run. | Test |
| FR-230-AC-4 | A serialized run record contains no source commit digest, source revision pin, or report-content digest. | Test |
| FR-230-AC-5 | Given no repository evidence store, `selftest --run --out <path>` retains the canonical run record at that path; without `--out`, the runner reports the missing retained destination and starts no case. | Test |
| FR-230-AC-6 | Given nested ancestors with `spec/evidence/`, the recorder writes under the nearest one without invoking Git and creates its `runs/<suite>/` directories; when no ancestor has `spec/evidence/`, it uses required `--out`. | Test |
| FR-230-AC-7 | Given two case runs with identical exit code, stdout, and expected files but different stderr bytes, both case statuses are identical and each run record contains its actual stderr as a JSON-safe UTF-8 string, replacing invalid byte sequences with U+FFFD. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) defines canonical retained run records; PLAT-1641 owns the external-tools configuration index.
- **Downstream**: [FR-217](./FR-217-validation-report.md) consumes retained run records.
