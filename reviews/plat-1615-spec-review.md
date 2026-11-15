---
id: SR-20261114002438
title: "Spec review — PLAT-1615 M2 dossier document content (quoin#929)"
type: SpecReview
analysis: base
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-191, FR-196..FR-201, spec/non-functional/NFR-041, spec/stakeholder/StR-010, spec/usecase/US-024"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-197", type: references }
  - { target: "ix://agent-ix/quoin/FR-198", type: references }
  - { target: "ix://agent-ix/quoin/FR-199", type: references }
  - { target: "ix://agent-ix/quoin/FR-201", type: references }
---

## Summary

Ticket: PLAT-1615. Reviewed the six new content FRs, NFR-041, and the FR-191 / StR-010 / US-024 amendments for consistency with each other, with the M1 dossier specs (FR-191..FR-195, NFR-040) and with the source specs they render (FR-144, FR-162/163, FR-169/170, FR-166..168). Sub-analyses run as separate artifacts: EARS, integrity, object, dependency, evidence, risk.

## Verdict

**FAIL** — two high findings: FR-197 turns a missing input into a hard failure that StR-010 says must be a gap, and FR-198 renders a git-derived field that changes after the dossier is committed, breaking the commit-then-check flow. The plan-record decision (option b) is recorded with its reason, each FR names its source FR and library entry point, generic document names are used throughout, and no paywalled standard is named.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-197 makes an absent `--binding` exit 3 for the whole command, contradicting StR-010-VC-2/VC-4 (missing M2 input is a named gap), FR-195's `no_clause_binding` gap for the same clause-binding input, and FR-201's gap-not-error rule; with all ten registered, a default `generate` without a binding fails every document. | spec/functional/FR-197-dossier-soup-content.md:58-59,71 |
| FND-002 | high | FR-198 renders `rationaleChange`, which FR-163 derives from git against the default-branch tip (`pending` until merged), so a committed DOS-ANOM regenerates differently after merge or when the default branch advances, and `check`/`export` report `differs` (FR-191-AC-11); it also conflicts with FR-201's no-git stance. | spec/functional/FR-198-dossier-unresolved-anomalies-content.md:57-58,71; spec/functional/FR-163-anomaly-report-release-rationale.md:43-47 |
| FND-003 | medium | FR-191 says M2 generators add their own input flags, but FR-197, FR-198 and FR-199 never name the dossier-side flags for SOUP binding/records/support or risk/anomaly scope (or whether SOUP reuses `--clause-binding`); only FR-201 does. | spec/functional/FR-197-dossier-soup-content.md:26-32; spec/functional/FR-191-dossier-command.md:35 |
| FND-004 | medium | FR-201 passes the EA result to `quire export --approval`, while FR-191 passes the user's `--approval` values unchanged; behavior when both are given, or in a bundle export, is unstated. | spec/functional/FR-201-dossier-release-record-content.md:75-79; spec/functional/FR-191-dossier-command.md:53 |
| FND-005 | medium | FR-199 maps every FR-170 finding to `risk_chain_incomplete` and also emits `control_unverified` / `residual_risk_undecided`, but FR-170 already has `control-without-verification` and `residual-risk-undecided` tokens, so one defect yields two gaps with no stated rule. | spec/functional/FR-199-dossier-risk-management-content.md:53-58; rust/crates/quoin-risk-advice/src/summary.rs:69-76 |

## Coverage

- Examined: FR-191 amended statement, sources, gaps, AC-2/8/18/19; FR-196..FR-201 statements, Inputs, Outputs, Behavior, all ACs; NFR-041 statement and AC-1..AC-4; StR-010 need and VC-1..VC-4; US-024 story and EX-4.
- Context only: FR-144, FR-162, FR-163, FR-164, FR-169, FR-170, FR-195, NFR-040.
- Tools: `quire validate` and `quoin` sub-commands could not be run (permission layer); findings come from reading the documents and the source code.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Absent `--soup-binding` now writes the SOUP document with a document-level `no_clause_binding` gap and exit 1 (FR-197-AC-5); exit 3 is limited to an unreadable binding or builder failure (AC-9). FR-144's `--binding` is a clause-binding report (FR-144:38), so the gap kind fits. |
| FND-002 | fixed | FR-198 no longer renders `rationaleChange`/`fixingChange` and AC-4 asserts changing them leaves the bytes unchanged. |
| FND-003 | fixed | FR-191 Inputs now name `--soup-binding`, `--soup-records`, `--soup-support`, `--snapshot`, `--release-verify`; FR-197 maps them to FR-144 flags and reserves `--clause-binding` for the RMM; FR-198/FR-199 use the common `--scope`/`--module`. |
| FND-004 | fixed | FR-191 appends the EA result as one more `--approval` after user approvals, only when `release-record` is selected, including bundles; FR-191-AC-21 tests it; FR-201 matches. |
| FND-005 | fixed | FR-199 maps `control-without-verification` to `control_unverified` only, `residual-risk-undecided` to `residual_risk_undecided` only, every other finding to `risk_chain_incomplete`; AC-4 tests the exclusivity. |
