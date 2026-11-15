---
id: SR-20261114002444
title: "Risk and complexity analysis — PLAT-1615 M2 dossier requirements (quoin#929)"
type: SpecReview
analysis: risk-complexity
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-191, FR-196, FR-200"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-196", type: references }
  - { target: "ix://agent-ix/quoin/FR-200", type: references }
---

## Summary

Ticket: PLAT-1615. Looked for technical risk and volatility the new requirements bring to the implementation tickets: unbuilt upstream contracts, interactions with the FR-191 exit taxonomy once all ten generators register, and requirements that can never be satisfied.

## Verdict

**FAIL** — FR-200's permanent `no_plan_record` gap makes every default `generate` exit non-zero forever, and FR-196's core classification depends on a member no source has yet. The git-volatility risk in FR-198 and the missing-binding hard failure in FR-197 are recorded in SR-20261114002438 and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-200 always emits `no_plan_record`, so DOS-RMP is always `incomplete`; once it registers, `generate` with no `--doc` (FR-191-AC-18) always exits 1, and a caller can no longer tell a complete dossier from one with a real gap by exit code. | spec/functional/FR-200-dossier-risk-management-plan-content.md:59-62; spec/functional/FR-191-dossier-command.md:138 |
| FND-002 | medium | FR-196's `evidence_failed` vs `evidence_stale` split depends on a typed `failed-run` cause that does not exist in `FindingDetail` today (kind, summary, path, line, symbol only); until PLAT-1760 lands, FR-196-AC-3/AC-4 cannot be implemented. | spec/functional/FR-196-dossier-verification-report-content.md:33-35,55-61; rust/crates/quoin-assurance/src/matrix.rs:252-266 |

## Coverage

- Examined: FR-196 Inputs and Behavior, AC-3, AC-4; FR-200 plan-record decision, Behavior and AC-1/AC-2; FR-191 AC-18 and the exit-code table (context).
- Context only: FR-195-AC-3 (an incomplete document exits 1).

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-191 now states that unfiltered `generate` returns 1 because of `DOS-RMP`'s permanent gap and that an explicit `--doc` subset gets exit 0 when complete; FR-191-AC-22 tests both, and FR-200 cross-references it. A caller can again tell a complete subset from a real gap by exit code. |
| FND-002 | fixed | FR-196 now states the generator cannot register until PLAT-1760 lands, so AC-3/AC-4 are gated on that work rather than silently unimplementable. |
