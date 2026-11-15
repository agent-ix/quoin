---
id: SR-20261114002439
title: "EARS analysis — PLAT-1615 M2 dossier requirements (quoin#929)"
type: SpecReview
analysis: ears-conformance
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-191, FR-196..FR-201, spec/non-functional/NFR-041"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-197", type: references }
  - { target: "ix://agent-ix/quoin/FR-200", type: references }
---

## Summary

Ticket: PLAT-1615. Checked each new or edited SHALL statement for an EARS pattern (ubiquitous, event-driven `When`, unwanted-behavior `If ... then`, state-driven `While`), a single system subject, one obligation per statement, and each new AC for atomicity.

## Verdict

**FAIL** — two statements put the SHALL on an external system rather than Quoin, so Quoin cannot be tested against them, and two ACs are compound or leave the outcome unstated. The six FR statements are well-formed event-driven requirements (`When quoin dossier generate selects <kind> ...`), NFR-041 is a clean ubiquitous constraint, and the Behavior bullets use `If`/`When` consistently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "If `--binding` is absent, the source SHALL refuse the request" places the obligation on FR-144's command, not on the dossier; it restates source behavior as a Quoin requirement. | spec/functional/FR-197-dossier-soup-content.md:57 |
| FND-002 | medium | "When a policy region lacks `outside_acceptable`, the upstream module validation SHALL reject it" obliges spec-objects-safety, and FR-200-AC-4 tests that upstream module rather than Quoin. | spec/functional/FR-200-dossier-risk-management-plan-content.md:63-64,76 |
| FND-003 | low | FR-191-AC-19 combines two behaviors (M2 gap parity between Markdown and JSON, and exit 2 for an unregistered kind) in one criterion. | spec/functional/FR-191-dossier-command.md:139 |
| FND-004 | low | FR-197-AC-5 says that without `--as-of` "the command refuses the usage before writing" without the exit code, and joins it to the separate missing-binding case. | spec/functional/FR-197-dossier-soup-content.md:71 |

## Coverage

- Examined: FR-191 amended statement and Behavior bullets; FR-196..FR-201 Description statements and every Behavior bullet; NFR-041 Statement; all new ACs (FR-191-AC-18/19, FR-196-AC-1..8, FR-197-AC-1..7, FR-198-AC-1..7, FR-199-AC-1..8, FR-200-AC-1..6, FR-201-AC-1..8, NFR-041-AC-1..4) and edited FR-191-AC-2/AC-8, StR-010-VC-1/VC-4.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The "source SHALL refuse" bullet is gone; FR-197 now states the dossier's own behavior for absent, as-of-less and unreadable bindings. |
| FND-002 | fixed | FR-200 now obliges the dossier: an invalid policy record, including a region without `outside_acceptable`, gives source exit 3; AC-4 tests the dossier. |
| FND-003 | fixed | FR-191-AC-19 now covers only gap parity; the unregistered-kind exit 2 is its own FR-191-AC-20. |
| FND-004 | fixed | FR-197-AC-5 now covers only the no-binding gap with exit 1; the missing `--as-of` case is FR-197-AC-8 with exit 2. |
