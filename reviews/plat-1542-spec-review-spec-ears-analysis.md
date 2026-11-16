---
id: SR-20261115210126
title: "EARS analysis of FR-202 and the FR-191 threat-model amendments"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #934 at 212507de: spec/functional/FR-202-dossier-threat-model-content.md, spec/functional/FR-191-dossier-command.md, spec/functional/index.md; context: agent-ix/spec-objects-security PLAT-1542 branch FR-026, FR-027"
---

# EARS analysis of FR-202 and the FR-191 threat-model amendments

## Summary

Ticket: PLAT-1542. Checked every new or edited SHALL statement in FR-202 Description and Behavior and in FR-191 (bundle order, gap_kind list, M3 gap subject) for EARS form: one trigger or state, one system response, no buried second requirement. Informative sentences without SHALL ('The checks are flow_rules data...', 'The document is committed and checked like other dossier documents') are explanatory and not counted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Controls-and-safeguards bullet joins two selection rules (controls that `protects` the connection, with safeguards; controls that `mitigates` a listed threat) in one SHALL, and ends with an untensed 'an absent entry reads `none`'. It does not say whether a control matched by both rules is listed once, whether mitigating controls show safeguards, or how the two groups are ordered together. Split it into two SHALL statements with one combined order and a dedup rule. | spec/functional/FR-202-dossier-threat-model-content.md:60-61 |

## Verdict

Statements are in ubiquitous or event-driven form with a named responder. One compound bullet: the Controls-and-safeguards rule joins two selection rules without saying how they combine.

Units: FR-202-Description (examined); FR-202-B-threats-cell (examined); FR-202-B-controls-cell (examined); FR-202-B-acceptance-cell (examined); FR-202-B-classification-cell (examined); FR-202-B-gap-mapping (examined); FR-202-B-unreadable (examined); FR-191-Bundle-order (examined); FR-191-M3-gap-subject (examined).

## Dispositions

Round 1, against PR head 212507de (no fix commit after the review).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | No fix round has landed: PR head is still 212507de, committed 2026-11-15T23:31:46Z, before this finding was posted (23:45Z); no later commit exists on any branch. FR-202 line 60-61 still joins the protects and mitigates rules in one bullet ending 'an absent entry reads `none`'. |

Round 2, against fix commit 6132ce8f (PR head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
