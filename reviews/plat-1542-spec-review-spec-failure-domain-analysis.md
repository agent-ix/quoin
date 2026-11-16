---
id: SR-20261115210129
title: "Failure-domain analysis of FR-202"
type: SpecReview
analysis: failure-domain
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# Failure-domain analysis of FR-202

## Summary

Ticket: PLAT-1542. Looked for unstated failure modes and edge cases in how FR-202 turns flow-report denials into rows and gaps: severity levels, sinks, empty or overlapping models, and unreadable sources. Covered and sound: no model (AC-5), one element in two models (AC-10), unreadable record (AC-9), accepted vs undecided risk (AC-3), absent `decided_by` (AC-8). quire-rs FR-096 (origin/main cd817f0) confirms the report carries each denial's resolved `severity`, includes `warning` denials, and drops `off` ones.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-202 takes rows and gaps from the findings 'at their resolved severity' and handles `off`, but FR-027 lets a project lower each check to `warning`, and the FR-096 report lists warning denials too. FR-202 does not say whether a warning denial yields a `GAP:` row and a gap (making DOS-TM `incomplete`), a row only, or neither, nor whether the row shows the severity. Decide it and add an AC. | spec/functional/FR-202-dossier-threat-model-content.md:67-68 |
| FND-002 | medium | `### Covered sinks` must show 'the classifications reaching' each sink, which per FR-027 is an `any_path` walk (incoming `delivers_to` then outgoing `carries`, plus the sink's own `carries`). For a sink with a denial the report's `matched` documents carry it, but for a sink that has a threat there is no denial, so the generator must walk the graph itself, while FR-202 says it SHALL NOT recompute a check and lists no such source. State where the classifications come from (the report's `facts`, a named walk, or direct `carries` only). | spec/functional/FR-202-dossier-threat-model-content.md:49-50 |
| FND-003 | low | `### Covered sinks` lists a sink's 'controls' but, unlike the crossings table, no rule says which: controls that `protects` the sink, controls that `mitigates` its threats, or both, nor whether safeguards are shown. Define it as the crossings Controls cell does. | spec/functional/FR-202-dossier-threat-model-content.md:49-50 |

## Verdict

Two medium edge cases and one low: warning-severity denials, the source of 'classifications reaching' a sink, and an undefined 'its controls' for sinks.

Units: FR-202-B-severity (examined); FR-202-Outputs-sinks (examined); FR-202-Inputs (examined); FR-202-AC-4 (examined); FR-202-AC-5 (examined); FR-202-AC-9 (examined); FR-202-AC-10 (examined).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 still does not say how a `warning`-severity denial is rendered. |
| FND-002 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 still names no source for 'the classifications reaching' a sink. |
| FND-003 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 still does not define which controls `### Covered sinks` lists. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
