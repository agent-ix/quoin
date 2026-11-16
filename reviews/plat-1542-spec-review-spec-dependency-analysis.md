---
id: SR-20261115210128
title: "Dependency analysis of FR-202"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# Dependency analysis of FR-202

## Summary

Ticket: PLAT-1542. Checked FR-202's `relationships` edges (StR-010 traces_to, US-024 implements, FR-191 extends, spec-objects-security FR-026/FR-027 references) and its Dependencies section against what the generator reads. The edges match the FR-196 to FR-201 pattern; FR-026/FR-027 exist on the companion spec-objects-security PLAT-1542 branch and name quoin FR-202 downstream. spec-artifacts-process FR-018 on its PLAT-1542 branch adds `threat-model` to `document_kind` (FR-018-AC-17). No other open quoin PR uses FR-202.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-202 reads `connection`, `data_sink`, `carries`, `crosses` and `delivers_to` (spec-objects-architecture, PLAT-1532/PLAT-1440), the `covers` verb (PLAT-1657) and `protects`/`safeguards` (spec-objects-security FR-025), but its Upstream list names none of them, and the Description calls the relation report 'existing' although FR-027's Engine impact says the rules cannot be evaluated until quire-rs delivers `flow_rules` (PLAT-1537/PLAT-1538). Add these to Upstream and drop 'existing'. | spec/functional/FR-202-dossier-threat-model-content.md:98-102 |

## Verdict

One low finding: the Upstream list omits the dependencies FR-027 itself calls hard blockers.

Units: FR-202-relationships (examined); FR-202-Dependencies (examined); FR-202-Description (examined); FR-027-Engine-impact (context_only).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 Upstream still omits spec-objects-architecture objects, PLAT-1657 and FR-025, and line 28 still says 'existing relation report'. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
