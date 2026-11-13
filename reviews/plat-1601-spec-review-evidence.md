---
id: SR-20261113052926
title: "Evidence review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin; PR #881; FR-142-AC-19..AC-27, FR-144-AC-16..AC-20, FR-149-AC-1..AC-11, NFR-030-AC-4"
review_set: all
---

# Evidence review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

Ran `quoin advise --json` and reviewed its recommendations for the changed obligations. The new test-based join and report criteria match catalog recommendations. Three adapter criteria authored as Inspection have catalog recommendations for executable evidence and should be reconsidered by the human spec owner.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `quoin advise --json` recommends test evidence for FR-149-AC-9 (I/O boundary), AC-10 (captured report fixtures), and AC-11 (retained record or success output), while all three are authored as Inspection. Confirm whether inspection alone demonstrates the no-file/subprocess/network boundary and each captured-report outcome; otherwise split the mixed obligations or update their verification methods. These are advisory catalog mismatches, not automatic failures. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:118; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:119; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:120 |

## Evidence details

The deterministic advisor recommends integration testing, unit testing, or BDD examples for the adapter-boundary criteria. The changed FR-142 and FR-144 criteria declare Test and their recommended methods agree. `quoin matrix --repo . --json` currently finds no tagged tests for changed FR-142, FR-144, FR-149, or NFR-030 criteria; this is expected before implementation on this spec-only PR.
