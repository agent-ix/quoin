---
id: SR-20261123202511
title: "Integrity review of release snapshot risk-completeness and anomaly-gate open findings"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-166-release-snapshot-command.md, spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1033). Integrity review of the added inputs, behaviors and acceptance criteria
in FR-166 (AC-12, AC-13) and FR-167 (AC-15 to AC-19): completeness, consistency of the cross-references to FR-164,
FR-169 and FR-170, atomicity and testability. The relationships and Dependencies sections were updated consistently
with the new `depends_on` edges; AC ids are unique and in sequence. Two low gaps remain in AC coverage and wording.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | The refusal covers every `quoin risk-summary` exit-2 cause (module that does not load, type declared by no loaded module, record contract break) but always names `--repo`, even when the failing input is a `--module DIR` root; FR-166-AC-13 tests only the `residual_risk` record case. Name `--module` for module failures and add one module case to AC-13. | FR-166-AC-13, FR-166, FR-169 |
| FND-002 | low | FR-166-AC-12 and FR-167-AC-15 say "a control with no incoming `verifies` edge", but FR-170 places control findings only on rows holding the control, that is a control some post-control estimate of a situation `assumes`; a control no estimate assumes yields no item. The ACs should state the control is assumed by a post-control estimate so the fixture is not ambiguous. | FR-167-AC-15, FR-166-AC-12, FR-170 |

## Verdict

Low findings only; neither blocks merge on its own.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The refusal names the failing root (`--repo` or the `--module DIR`) and FR-166-AC-13 adds a `--module DIR` that does not load. |
| FND-002 | fixed | FR-166-AC-12 and FR-167-AC-15 now require a control that a post-control estimate assumes; the AC's cross-check against `quoin risk-summary` over the same fixture forces that estimate to estimate a situation, so the fixture is unambiguous. |
