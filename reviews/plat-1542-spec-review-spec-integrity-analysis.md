---
id: SR-20261115210127
title: "Integrity analysis of the FR-191 threat-model amendments and FR-202"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# Integrity analysis of the FR-191 threat-model amendments and FR-202

## Summary

Ticket: PLAT-1542. Checked the FR-191 edits for internal consistency with FR-191's own ordering, gap-subject and dependency rules, the closed gap_kind list against FR-202's gap mapping, and the index entry. The four new gap kinds appear in FR-191's list in the order FR-202 uses; FR-191-AC-18 and AC-22 are correctly narrowed to 'only the M1 and M2 generators'; the index title equals FR-202's title.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-191 sorts gaps by (requirement id, criterion id, element id, clause id, gap kind position), maps only a part to element id, and names only `no_requirements`, `design_relations_unavailable` and `no_clause_binding` as id-less gaps that sort first. The diff adds gaps naming a connection, a sink or a threat, and a document-level `no_threat_model`, without saying which key slot their subject fills. The global gap order for DOS-TM, which AC-2 checks against `--json`, is therefore undefined. State that these subjects are the element id and that `no_threat_model` is id-less. | spec/functional/FR-191-dossier-command.md:89-92 |
| FND-002 | low | FR-191's Downstream line still lists only FR-192 to FR-201 although the diff names FR-202 in FR-191's Description and FR-202 `extends` FR-191. Add FR-202 as the M3 content requirement. | spec/functional/FR-191-dossier-command.md:157-157 |

## Verdict

One medium defect: FR-191's gap sort key has no slot for the M3 subjects. One low: FR-191's Downstream list omits FR-202.

Units: FR-191-Ordering (examined); FR-191-Gap-kinds (examined); FR-191-M3-gap-subject (examined); FR-191-Dependencies (examined); FR-191-AC-18 (examined); FR-191-AC-22 (examined); FR-202-index-entry (examined).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-191 Ordering still maps only a part to element id and names only three id-less gap kinds. |
| FND-002 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-191 Downstream still lists FR-192 to FR-201 only. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
