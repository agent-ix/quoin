---
id: SR-20261115183522
title: "Spec object review — PLAT-1760 auditor Finding `cause` field"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-167-release-snapshot-format.md (context), spec/functional/FR-032-evidence-auditor.md (context); PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. `spec-object-review` lens over the domain object PR #931
changes: the auditor `Finding`, which `quoin-finding-types` defines as
`AuditFinding` and which gains the `cause` member. This lens is not in the
module's `analysis` vocabulary, so `analysis: base` is recorded. Checked: who
owns the `Finding` object, whether the new value set is closed, and the
cross-references from consumers of `evidence_detail.findings`.

## Verdict

**CONDITIONAL**, with one medium finding and one low. No FR in `spec/functional/`
declares the auditor `Finding` as an `object:` (the seven `object:` FRs are
schemas for other records). `cause` is a new value vocabulary defined inline in
a consumer's Outputs bullet. That is acceptable for now if it is made
complete, but it is not complete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `cause` value set is never stated to be closed, and no rule covers an unknown value. `Finding.kind` is deliberately an open string, and `Finding` accepts unknown members (`other`, flatten, no `deny_unknown_fields`). FR-115 lists two values but does not say whether `matrix.build` (which reads the audit from stdin) refuses an unknown `cause`, such as `flaky-run`, with `BadRequest` or passes it through verbatim. | spec/functional/FR-115-evidence-backed-test-matrix.md:219-225 |
| FND-002 | low | FR-167's `open_findings` row says its source is FR-115 `evidence_detail.findings` and lists its members as `obligation, kind, summary, path, line, symbol`. FR-115 now adds `cause` to that source. FR-167 does not say whether the release snapshot carries or drops it, so the cross-reference no longer states the full member set of what it projects. | spec/functional/FR-167-release-snapshot-format.md:40 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-167's general absent-source rule says to emit null, while the new cause rules omit cause on non-stale findings. FR-115 also permits a stale-evidence finding without cause, but FR-167 does not say whether to emit null or omit the key. State the cause omission exception for both cases. | spec/functional/FR-167-release-snapshot-format.md:59, 65-66 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-115 now defines exactly two cause values and refuses an unknown cause or a cause on a non-stale finding (FR-115-AC-22..23). |
| FND-002 | fixed | FR-167's open_findings row, behavior and acceptance criteria now carry stale-evidence cause from FR-115 and omit it for other kinds. |
| FND-003 | still-open | The general FR-167 absent-value rule conflicts with omitting the optional cause. The cause-specific exception and stale-without-cause case need to be stated. |

Round 2, fix commit 76a6f3dc.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | FR-167 now says the command omits an absent cause and never serializes it as null; AC-12 covers a stale-evidence finding whose FR-115 source omits cause. FR-032 Outputs also names the optional cause. |
