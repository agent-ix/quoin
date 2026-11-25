---
id: SR-2026112016223475846355058
title: "code review review of PLAT-1821 snapshot anomaly batch"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-anomaly/src/carried.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-dossier/src/release_record.rs, rust/crates/quoin-dossier/src/release_snapshot.rs, changed tests"
review_set: subset
---

## Summary

The frozen-head Rust review found a duplicated carried-anomaly predicate and a stale public snapshot field comment. The focused build evidence was supplied by the PR; no additional builds or tests were run.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | DOS-REL reimplements the FR-162 carried predicate instead of sharing the anomaly predicate, allowing Known classification to drift from snapshot/gate semantics. | rust/crates/quoin-dossier/src/release_record.rs:297-300 |
| FND-002 | medium | The public anomalies field documentation says it contains only carried anomalies, but snapshots now also include closed/fixed non-carried anomalies. | rust/crates/quoin-dossier/src/release_snapshot.rs:44-45 |

## Assurance Context

- Applicable control: `spec/assurance/AP-201-finding-quality.md`; this change affects release snapshot anomaly selection, FR-164 gate findings, and DOS-REL rendering.
- AP-201 context is current in the checked-out repository; linked context was available and no active exceptions were identified for these paths.
- AP-201 recommends evidence, integrity, and scope-boundary lenses; those lenses were applied in separate review artifacts. Its suggested full gate was not rerun because the review dispatch explicitly prohibited additional tests/builds; only the supplied focused locked-build result is cited.
