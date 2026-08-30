---
id: SR-051
title: "Evidence-method review of issue 288 semantic type-fit audit requirements"
type: SpecReview
analysis: evidence
scope: "FR-051..FR-055, NFR-015..NFR-016, TM-001 TC-1156..TC-1194"
review_set: all
---

# Evidence-method review of issue 288 semantic type-fit audit requirements

## Summary

The deterministic advisor was run with Quoin 0.22.5 and Quire 0.31.0
(`cli 4f6ed024`, `engine 0.46.0@ca7362d4`) against the installed catalog. Its recommendations support
property tests for universal denominators and round trips, snapshot tests for stable output, fuzzing
at parser surfaces, integration tests at I/O boundaries, and inspection for the human promotion gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-042 | low | Planned test methods match the observable claims; NFR-016's manual gate remains Inspection despite the advisor's generic numeric-threshold heuristic. | TC-1156..TC-1194; NFR-016-M-3 |

## Method allocation

| Claim shape | Evidence |
| --- | --- |
| Finite vocabulary and per-record invariants | Unit and property tests over adversarial fixtures |
| Denominator equality and equal-input determinism | Property tests |
| Parser failure retention | Unit fixtures plus fuzz/property coverage |
| Canonical rendered report | Snapshot test generated from the same canonical object |
| Read-only filesystem boundary | Integration test with read-only inputs and before/after state |
| Retained campaign census and reconciliation | Static validation against pinned evidence |
| Major-interference promotion | Named maintainer inspection |

The advisor initially failed when forced to the stale local process-module checkout because that
manifest is incompatible with the active engine. The successful run used the installed catalog;
FR-051 requires both tool and source identities so this version skew cannot be hidden in the audit.
