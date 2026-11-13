---
id: SR-20261113071107
title: "Spec review of quoin PR 854 (FR-136 fixing change)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@af5b091b; spec/functional/FR-136-anomaly-record-mapping.md"
review_set: subset
---

# Spec review of quoin PR 854

## Summary

Ticket: PLAT-1527. Scoped to `git diff origin/main...HEAD` at af5b091b: FR-136 Behavior gains
four statements on the fixing change (lines 87-96), one statement is deleted, and AC-13 and AC-14
are added. Read against the companion PR agent-ix/spec-artifacts-process#145 (FR-014, FR-015 at
67f6422). Tools: quoin 0.30.1, quire 0.44.0 (engine 0.59.1). `quire validate` passes on FR-136
with one unrelated warning (`semantic.unused-import` in module spec-artifacts-app).

Review set: base checklist (this file), plus integrity and ears-conformance in their own files.

Brief checks: no hash, commit, author or date is recorded in the document (the fixing change is
derived from git history); no compatibility layer, fallback or shim; no paywalled standard text.

## Verdict

**FAIL, four medium and one low finding.** The core of the change is sound: the fixing change
comes from git history, and the document holds no hash. The selection rule does not cover
histories that have no `assessed` entry, or a non-`fixed` closure that went through `resolved`.
Removing the general author/date ban leaves part of AC-1 with no statement behind it. "Expose"
names no surface.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rule "the first `statusHistory` entry whose `status` is `resolved` after the most recent entry whose `status` is `assessed`" is undefined when no `assessed` entry exists. That happens on legal histories: a first entry of `resolved` (AC-4), and `identified` then `resolved` squashed (AC-3). Read literally, the anomaly is fixed but `fixingChange` is absent. State the rule for that case, for example "after the most recent `assessed` entry, or from the start of the history when there is none", and add the case to AC-13 | spec/functional/FR-136-anomaly-record-mapping.md:87-89, :162 |
| FND-002 | medium | The rule is scoped to "an anomaly resolved as `fixed`". The absence rule, though, names only "no such entry" and "pending". `resolved` then `closed` with `resolution` `wont-fix` is a legal FR-014 move, and that history has a `resolved` entry after its last `assessed`. One implementer would expose `fixingChange`, another would not. State that `fixingChange` is absent unless the current `resolution` is `fixed`, and add the case to AC-14 | spec/functional/FR-136-anomaly-record-mapping.md:87-92, :163 |
| FND-003 | medium | The diff deletes "Quoin SHALL NOT take any author or date from a field typed into the document." The new statement at line 93 bans only "a hash of the fix". AC-1 still requires "no author or date typed in the document is read" for `statusHistory`, and no statement backs that clause now. Restore the deleted statement next to the new one | spec/functional/FR-136-anomaly-record-mapping.md:93, :150 |
| FND-004 | medium | "Quoin SHALL expose the fixing change as the record key `fixingChange`" names no surface. `quoin anomaly check` prints diagnostics only, `quoin anomaly refs` prints `{ anomaly, kind, impact }`, and the record is extracted by quire-rs, not by Quoin. Nothing says where a consumer reads `fixingChange`, so AC-13 has no observable output. Name the output that carries it: a `--json` view of `quoin anomaly check`, a library type, or a field of the refs entry | spec/functional/FR-136-anomaly-record-mapping.md:90-92, :162 |
| FND-005 | low | "Quoin SHALL NOT check that the severity, the `## Fix Verification` links and the fixing change agree beyond the checks stated here". FR-136 states no such check, so "beyond the checks stated here" refers to nothing, and a reader will look for one. Say "Quoin SHALL NOT check that ... agree" | spec/functional/FR-136-anomaly-record-mapping.md:94-96 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "The fixing change of an anomaly resolved as `fixed` (spec-artifacts-process FR-015) SHALL be the first `statusHistory` entry whose `status` is `resolved` after the most recent entry whose `status` is `assessed`. Quoin SHALL expose the fixing change as the record key `fixingChange` ... Quoin SHALL NOT read a hash of the fix from the document. Quoin SHALL NOT check that the severity, the `## Fix Verification` links and the fixing change agree beyond the checks stated here"}
- {id: FR-136-AC-13, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "For a history `identified`, `assessed`, `resolved`, `closed` (resolution `fixed`), `fixingChange` holds the commit, author, authored date, `signature` and `signatureCode` of the `resolved` entry; for a history `resolved`, `assessed`, `resolved`, it is the second `resolved` commit; no hash typed in the document is read."}
- {id: FR-136-AC-14, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "`fixingChange` is absent for an anomaly not yet `resolved`, for a pending-only `resolved` working-tree status, and for a file outside git, and `quoin anomaly check` emits no diagnostic for its absence."}
- {id: FR-136-AC-1, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "... yields four `statusHistory` entries in order, each with author, authored date, `signature` and `signatureCode`, and a file renamed along the way keeps its entries; no author or date typed in the document is read."}
- {id: FR-136-AC-3, path: spec/functional/FR-136-anomaly-record-mapping.md, role: context_only, excerpt: "not emitted for `identified` then `resolved` (squashed), `identified` then `closed` with `fixed` (squashed through `resolved`), `assessed` then `closed` with `fixed`"}
- {id: FR-136-AC-4, path: spec/functional/FR-136-anomaly-record-mapping.md, role: context_only, excerpt: "A history whose first entry is `resolved` or `closed` yields no diagnostic."}
- {id: FR-014, path: "agent-ix/spec-artifacts-process#145 spec/functional/FR-014-anomaly-archetype-and-state-lifecycle.md", role: context_only, excerpt: "When `status` is `resolved`, `resolution` SHALL be `fixed`. ... `resolved` to `closed` ... legal moves"}
