---
id: SR-20261113071108
title: "Integrity review of quoin PR 854 (FR-136 fixing change)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin@af5b091b; spec/functional/FR-136-anomaly-record-mapping.md"
review_set: subset
---

# Integrity review of quoin PR 854

## Summary

Ticket: PLAT-1527. Checked the FR-136 amendment against the companion PR
agent-ix/spec-artifacts-process#145 (FR-014 and FR-015 at 67f6422), the module's
`mappings.yaml` and the emitted `Anomaly.json` at spec-artifacts-process v0.31.0, which is what
quoin resolves.

## Verdict

**FAIL, one high and one medium finding.** On ownership the two PRs agree. The module owns the
presence and shape of `severity` and `## Fix Verification`. Quoin owns the fixing change, which
comes from git. Neither document records a hash, author or date. The two PRs disagree on where
the fixing change lives. They also disagree on whether a squashed `fixed` closure has a fixing
change at all.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-136 exposes the fixing change "as the record key `fixingChange`". The companion FR-015 says "The `Anomaly` type SHALL NOT carry a commit hash, author or date for the fix, or a field that names the fixing change", and FR-015-AC-16 checks that by Inspection. The module's `mappings.yaml` also says the git-derived `statusHistory` "is not a property of this record". The emitted `Anomaly.json` is sealed (`unevaluatedProperties: {not: {}}`), so a record that carries `fixingChange` fails it. Make `fixingChange` a Quoin-derived value outside the record, beside `statusHistory` (and drop "record key"), or amend FR-015 and the mapping in #145 to allow it | spec/functional/FR-136-anomaly-record-mapping.md:90-92; spec-artifacts-process#145 FR-015 Behavior, FR-015-AC-16 |
| FND-002 | medium | Squashed `assessed` then `closed` with `fixed`, and `identified` then `closed` with `fixed`, are legal (AC-3), and FR-136's own rationale says these repositories squash-merge. Those histories have no `resolved` entry, so FR-136 gives no `fixingChange`. FR-015 in #145 says the fixing change "is the commit that moved `status` to `resolved` in the git history of the file, which quoin FR-136 reads", and FR-136 depends on that. For a single-commit fix that lands closed, the two disagree. Either take the first entry that reaches `resolved`, or `closed` with `fixed`, after the last `assessed`, or state in both documents that a squashed closure has no fixing change, and add an AC | spec/functional/FR-136-anomaly-record-mapping.md:87-89, :163; spec-artifacts-process#145 FR-015 Description |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Quoin SHALL expose the fixing change as the record key `fixingChange`, holding that entry's `commit`, `author`, `authoredAt`, `signature` and `signatureCode`, absent when no such entry exists or when `resolved` is only pending."}
- {id: FR-136-AC-14, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "`fixingChange` is absent for an anomaly not yet `resolved`, for a pending-only `resolved` working-tree status, and for a file outside git"}
- {id: FR-015, path: "agent-ix/spec-artifacts-process#145 spec/functional/FR-015-anomaly-safety-impact-assessment.md", role: context_only, excerpt: "The change that made the fix is not recorded in the document: it is the commit that moved `status` to `resolved` in the git history of the file, which quoin FR-136 reads. ... The `Anomaly` type SHALL NOT carry a commit hash, author or date for the fix, or a field that names the fixing change"}
- {id: FR-014, path: "agent-ix/spec-artifacts-process#145 spec/functional/FR-014-anomaly-archetype-and-state-lifecycle.md", role: context_only, excerpt: "When `status` is `assessed`, `resolved` or `closed`, `severity` SHALL be present. ... The key `statusHistory` is not authored anywhere: quoin FR-136 derives it from git."}
- {id: mappings.yaml, path: "agent-ix/spec-artifacts-process spec_artifacts_process/mappings.yaml", role: context_only, excerpt: "`statusHistory` is not authored anywhere and is not a property of this record"}
```
