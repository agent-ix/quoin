---
id: SR-20261113083118
title: "Dependency review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #857 (git diff origin/main...plat-1528-spec-quoin at 56f23309): spec/functional/FR-162, FR-163, FR-164, spec/non-functional/NFR-035 and both index edits"
---

# Dependency review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)

## Summary

Ticket: PLAT-1528. Method: spec-review/spec-dependency-analysis, scoped to `git diff origin/main...HEAD` of agent-ix/quoin#857 at 56f23309.
Read against quoin FR-136 and FR-147 as merged, spec-artifacts-process FR-014 and FR-015 on main, and FR-016 from
the companion PR agent-ix/spec-artifacts-process#147.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-162/FR-163 read severity, title and releaseRationale from the Anomaly record, but on main the record comes from FR-136's temporary reader (CR note, PLAT-1415; rust/crates/quoin-anomaly/src/record.rs), which reads only status, resolution, closed_by, safety_impact and the assessment table, and the extractor that replaces it (agent-ix/quire-rs#393) is in no Dependencies list here. FR-136's fixingChange derivation is also absent from main's code (no fixing-change symbol in quoin-anomaly). PLAT-1585 must widen a reader FR-136 marks as a temporary exception, or wait for #393. Name the dependency and say which. | spec/functional/FR-162-anomaly-report.md:33 |

## Verdict

One dependency gap (record reader).

Checked and clean: every relationships edge resolves (FR-136, FR-144, FR-147, spec-artifacts-process FR-014/FR-015 on main, FR-016 in the companion PR #147, which must merge before or with this PR); FR-147 is context only and is not changed; the dossier export is a downstream reader only; no compatibility layer or migration path is introduced; FR-162-AC-12 forbids copying module fixtures into quoin.

Examined (role examined): FR-162-AC-1, FR-162-AC-2, FR-162-AC-3, FR-162-AC-4, FR-162-AC-5, FR-162-AC-6, FR-162-AC-7, FR-162-AC-8, FR-162-AC-9, FR-162-AC-10, FR-162-AC-11, FR-162-AC-12, FR-162-AC-13, FR-163-AC-1, FR-163-AC-2, FR-163-AC-3, FR-163-AC-4, FR-163-AC-5, FR-163-AC-6, FR-163-AC-7, FR-163-AC-8, FR-163-AC-9, FR-164-AC-1, FR-164-AC-2, FR-164-AC-3, FR-164-AC-4, FR-164-AC-5, FR-164-AC-6, FR-164-AC-7, FR-164-AC-8, FR-164-AC-9, FR-164-AC-10, FR-164-AC-11, FR-164-AC-12, NFR-035-AC-1, NFR-035-AC-2, NFR-035-AC-3, NFR-035-AC-4, NFR-035-AC-5, and every Behavior statement of FR-162, FR-163, FR-164 and NFR-035.
Context only: FR-136, FR-147, spec-artifacts-process/FR-014, spec-artifacts-process/FR-016.
