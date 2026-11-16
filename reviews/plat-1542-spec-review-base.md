---
id: SR-20261115210125
title: "Base spec review of FR-202 and the FR-191 threat-model amendments"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# Base spec review of FR-202 and the FR-191 threat-model amendments

## Summary

Ticket: PLAT-1542. Checklist review of FR-202 and the FR-191 edits: requirement form, AC coverage of every Behavior bullet, determinism of the rendered bytes, and scope boundary against NFR-005 (person-authored threats). The no-scoring, no-adequacy and no-recompute boundaries are stated and tested (AC-8). The diff names no paywalled standard and carries no roadmap content.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-202 orders model sections (dossier order) and cell entries (dossier id order) but not the rows of `### Covered crossings`, the entries of `### Covered sinks`, the `GAP:` rows of `### Open findings`, or the part/boundary ids listed under `### Scope`. AC-7 requires byte-identical output, and every M2 content FR (FR-196 to FR-201) states its row order; two implementers here would emit different bytes. State dossier id order (or source order) for each. | spec/functional/FR-202-dossier-threat-model-content.md:40-52 |
| FND-002 | medium | Two Behavior bullets have no AC: listing a covered `part` or `trust_boundary` by id under `### Scope` (and giving it no crossing row), and a check set to `off` yielding no `GAP:` row and no gap. AC-10's second half tests one model only, so 'a threat considered by two models gets a row in each section and one gap' is also untested. Add ACs. | spec/functional/FR-202-dossier-threat-model-content.md:56-68 |

## Verdict

Two medium gaps: FR-202 leaves the order of rows and list items inside each model section unspecified although AC-7 requires byte-identical output, and two Behavior bullets have no AC. AC-1 to AC-10 are otherwise concrete, falsifiable and match the Behavior they test.

Units: FR-202 (examined); FR-202-Outputs (examined); FR-202-B-covers (examined); FR-202-B-severity (examined); FR-202-B-two-models (examined); FR-202-AC-1 (examined); FR-202-AC-2 (examined); FR-202-AC-3 (examined); FR-202-AC-4 (examined); FR-202-AC-5 (examined); FR-202-AC-6 (examined); FR-202-AC-7 (examined); FR-202-AC-8 (examined); FR-202-AC-9 (examined); FR-202-AC-10 (examined).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 Outputs still give no order for Covered crossings rows, Covered sinks entries, Open findings rows or Scope id lists. |
| FND-002 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 still has only AC-1 to AC-10; no AC tests the part/trust_boundary Scope listing, the `off` check, or a threat considered by two models. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-202-AC-14, added by the fix, requires one fixture with three findings in which `no_threat_model` precedes the gaps that name an id. `no_threat_model` arises only when no `threat_model` is selected, and then the three checks produce no finding (FR-202-AC-5, spec-objects-security FR-027-AC-5), and DOS-TM has no other gap source. No fixture can satisfy both halves, so the AC always fails or its second clause is vacuous. Drop the `no_threat_model` clause, or test that ordering only in FR-191 where other documents' gaps exist. | spec/functional/FR-202-dossier-threat-model-content.md:129-129 |
| FND-004 | low | The fix changed the row format to `GAP: <check> <subject> (<severity>)`, but FR-202-AC-2 still expects the row `GAP: crossing-without-threat CONN-1` with no severity suffix. A test asserting the AC text exactly fails against the new format. Update AC-2 to `GAP: crossing-without-threat CONN-1 (error)`. | spec/functional/FR-202-dossier-threat-model-content.md:117-117 |
