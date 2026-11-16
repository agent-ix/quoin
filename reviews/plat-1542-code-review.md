---
id: SR-20261115210124
title: "Code review of the dossier threat-model document spec (quoin PR 934)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# Code review of the dossier threat-model document spec (quoin PR 934)

## Summary

Ticket: PLAT-1542. Spec-only diff (3 files, +111/-5): new FR-202, FR-191 amendments for `DOS-TM`, index line. Reviewed the diff text for correctness against itself, FR-191 and the upstream spec-objects-security FR-026/FR-027 on the companion PLAT-1542 branch (head c8d56c4). No production code, so the Rust lane does not apply. `quire validate --scope .` over the three changed files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-202 Behavior says the generator SHALL read only `threat_model` documents, which contradicts its own Inputs (threat, risk, control, connection, data_sink, trust_boundary and data_classification documents) and every table cell it must fill; read literally the Threats, Controls and Acceptance cells cannot be rendered. The intent, per spec-objects-security FR-026, is that only committed (person-authored) `threat_model` and `threat` documents count and an uncommitted draft does not; say that instead. | spec/functional/FR-202-dossier-threat-model-content.md:75-75 |
| FND-002 | medium | FR-191 closes the source set to the readers 'named in FR-192 to FR-201' and lists thirteen names. FR-202 adds two sources (`threat-model-records`, `flow-findings`) but the diff does not extend that sentence, so FR-202-AC-9's 'exit 3 naming the source' names a source FR-191 does not admit. FR-191 Inputs also still say the common `--scope`/`--module` select only 'anomaly and risk records'. Extend both. | spec/functional/FR-191-dossier-command.md:80-80 |

## Verdict

Two defects. FR-202's 'read only `threat_model` documents' sentence contradicts its own Inputs, and FR-191's closed source list is not extended with the two new sources that FR-202-AC-9 expects exit 3 to name. The FR-191 bundle order, gap_kind list extension and AC-18/AC-22 rescoping are correct; FR-191-AC-23 is a sound new criterion; the index line matches the FR-202 title.

Units: FR-202 (examined); FR-191 (examined); FR-202-B-read-only (examined); FR-202-Inputs (examined); FR-191-Sources (examined); FR-191-Bundle-order (examined); FR-191-AC-18 (examined); FR-191-AC-22 (examined); FR-191-AC-23 (examined); FR-202-AC-9 (examined); FR-026 (context_only).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 Behavior line 75 still reads 'The generator SHALL read only `threat_model` documents in the corpus'. |
| FND-002 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-191 line 80 still limits sources to those 'named in FR-192 to FR-201' and line 36 still says --scope/--module select 'anomaly and risk records'. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
