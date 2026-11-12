---
id: SR-20261112024902
title: "Spec review of quoin PR 828 (FR-136 Anomaly record mapping)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@b92fb76; spec/functional/FR-136-anomaly-record-mapping.md, spec/functional/index.md"
review_set: subset
---

# Spec review of quoin PR 828

## Summary

Ticket: PLAT-1414. Scoped to `git diff origin/main...HEAD` at b92fb76: a new FR-136 and one
index line. Read against the companion module PR agent-ix/spec-artifacts-process#139 (FR-014,
FR-015 at 36754ea) and the quoin requirements FR-136 depends on (FR-121, FR-104). Tools: quoin
0.30.1, quire 0.40.0 (engine 0.56.0). `quire validate` passes on FR-136 and the index with no
EARS warnings. The full `spec/**/*.md` run fails only on FR-113 and on matrix rows TC-1943 and
TC-1944, which were already failing and which this PR does not touch.

Review set: base checklist (this file), plus integrity and ears-conformance in their own files.
Cross-document contradictions are recorded in the integrity artifact, not here.

## Verdict

**FAIL, five medium and two low findings here; the integrity artifact has two high findings.**
FR-136 is consistent with the module on vocabulary, table headers, and who owns the document
shape. It does not copy a fixture: it requires reading the module's fixture and holding no copy.
The gaps are an undefined Target domain, an undefined reverse-reference surface, and a fixture
set that the module does not commit to ship.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-015 allows a `Requirement` row's `Target` to be an FR, an NFR, or one of its acceptance criteria. FR-136 resolves a Target only to "the `SemanticId` of the artifact it names", and no AC covers an AC-id target. The `None` row (Target `-`) has no stated mapping either: target absent, `-` verbatim, or no resolution attempted. State the resolution for an AC-id target and for a `None` row, and add an AC for each | spec/functional/FR-136-anomaly-record-mapping.md:41-43 |
| FND-002 | medium | The reverse-reference bullet and FR-136-AC-6 do not name the surface that "asking for the references" goes through: a quoin command, a record key, or a graph edge. No existing quoin FR defines a reverse-reference query. Two implementers would build different things, and AC-6 has no observable to test. Name the command or output, and its shape | spec/functional/FR-136-anomaly-record-mapping.md:49-50, :65 |
| FND-003 | medium | FR-136-AC-8 and the fixture bullet say tests read "an Anomaly golden fixture" from spec-artifacts-process. FR-014 and FR-015 commit only to a skeleton plus negatives for an illegal transition, a status/history disagreement, an unnamed closer, a missing assessment and a dangling link. AC-1 (three-row History), AC-3/AC-6 (a corpus holding FR-001 and HAZ-001) and AC-5 (kind mismatch) need fixtures the module does not promise. So either the module must ship them, which needs an FR-014/015 AC, or quoin authors its own test inputs, which AC-8's wording forbids. Name the fixtures the module owns and say which inputs quoin may author. Note: FR-121-CON-2 sources record-key golden fixtures from quire-rs `quire-fixtures`, a third owner | spec/functional/FR-136-anomaly-record-mapping.md:53-54, :67 |
| FND-004 | medium | FR-136-AC-2 needs a fixture whose frontmatter `status` differs from the last History `To`. FR-014-AC-6 makes that document fail validation. FR-136 does not say whether quoin extracts a record from a document that fails module validation, so AC-2 relies on unstated behaviour. State it, or have AC-2 use the module's existing status-disagreement negative explicitly | spec/functional/FR-136-anomaly-record-mapping.md:61 |
| FND-005 | medium | The dangling-reference and kind-mismatch diagnostics have no code, severity or `reason`. Every sibling mapping (FR-104, FR-121) names its codes on the FR-075 shape, and AC-4 and AC-5 cannot assert a diagnostic they cannot identify. Name each code (or reuse `semantic.invalid-model-cell` / `semantic.unresolved-target` with a reason) and give its severity | spec/functional/FR-136-anomaly-record-mapping.md:44-48 |
| FND-006 | low | FR-136 has no `implements` edge to a quoin US or StR. Its siblings (FR-121 implements US-020) trace upward. The quire-rs upstream is prose only and names no quire-rs FR. Add the upward edge and name the quire-rs requirement | spec/functional/FR-136-anomaly-record-mapping.md:4-14, :72 |
| FND-007 | low | The actor is "Quoin SHALL map". FR-121 and FR-123 split the work: quoin publishes the mapping as golden fixtures, and Quire implements the extraction. FR-136 does not say which binary extracts the record. The Description also asks quoin's commands to report the same dangling link that `quire validate` reports, with no rule against double reporting. State the actor split as the sibling FRs do | spec/functional/FR-136-anomaly-record-mapping.md:21-26 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Quoin SHALL specify how an Anomaly document maps to the record that quoin's consumers read, and how each safety-impact assessment link is reconciled against the requirement and hazard ids of the corpus"}
- {id: FR-136-AC-1, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "three-row History extracts history with three entries in order, first entry's from absent; status and safety_impact equal frontmatter"}
- {id: FR-136-AC-2, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "frontmatter status differs from last history to extracts both verbatim"}
- {id: FR-136-AC-3, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "corpus holding FR-001 and HAZ-001 ... two assessment entries whose target are the SemanticIds"}
- {id: FR-136-AC-4, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "row targeting FR-999 yields one dangling-reference diagnostic and an entry with unresolved target"}
- {id: FR-136-AC-5, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Hazard row targeting FR-001 yields a kind-mismatch diagnostic"}
- {id: FR-136-AC-6, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Asking for the references to HAZ-001 lists the anomaly that names it with its Impact"}
- {id: FR-136-AC-7, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "identified fixture with no assessment section extracts no assessment key and reports it not_applicable"}
- {id: FR-136-AC-8, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "fixture the tests read is resolved from the spec-artifacts-process checkout, and quoin contains no copy"}
- {id: spec/functional/index.md, path: spec/functional/index.md, role: examined, excerpt: "- [FR-136: Anomaly document mapping to record keys and corpus-resolved assessment links](./FR-136-anomaly-record-mapping.md)"}
- {id: FR-014, path: "agent-ix/spec-artifacts-process#139 spec/functional/FR-014-anomaly-archetype-and-state-lifecycle.md", role: context_only, excerpt: "Anomaly type, status vocabulary, History table From|To|By|Date|Note, skeleton plus negatives"}
- {id: FR-015, path: "agent-ix/spec-artifacts-process#139 spec/functional/FR-015-anomaly-safety-impact-assessment.md", role: context_only, excerpt: "safety_impact vocabulary, Kind|Target|Impact|Rationale, Requirement target is FR/NFR or one of its ACs"}
- {id: FR-121, path: spec/functional/FR-121-record-key-sections-mapping.md, role: context_only, excerpt: "record-key sections mapping; no frontmatter keys; SemanticId cells by FR-104 Target rules"}
```

## New findings (disposition pass 1)

Checked at f1ad73c (fix commits 9786a3fc, 5cf39797 and f1ad73c0), against spec-artifacts-process#139 at 0956b64.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | `statusHistory` is "one entry per commit that changed the value of `status`, in commit order", but the spec does not say which history is read: first-parent of the current branch, all reachable commits, or a history that follows renames. These repos squash-merge, so a PR that moves `identified` to `assessed` to `resolved` lands as one commit and is reported as `anomaly.illegal-transition`. A new anomaly committed already `assessed` gets `anomaly.illegal-first-state`. State the history walked (first-parent, `--follow`), and either state that each move must land in its own main-line commit or define how a squashed multi-move commit is judged | spec/functional/FR-136-anomaly-record-mapping.md:58-61, :75-76 |
| FND-009 | low | Git reports a signature check as `G`, `B`, `U`, `X`, `Y`, `R`, `E` or `N`, and the spec does not map those to `signed`, `unsigned` or `unknown`. A bad signature (`B`) or an expired key (`X`) could land in any of the three. Give the mapping | spec/functional/FR-136-anomaly-record-mapping.md:59-61 |
| FND-010 | low | AC-9 restricts tests to the module's documents. But AC-10 needs an anomaly with two `Hazard` rows, and AC-7 needs a hazard that no anomaly names. FR-015 promises one hazard and the AC-4 anomaly (one `Hazard` row), so neither document is promised. Either add them to FR-015's fixture list, or let quoin's tests author these variants | spec/functional/FR-136-anomaly-record-mapping.md:106-109 |
| FND-011 | low | Scope note, no owner decision needed: quoin owns its CLI, so naming new commands in its own FR is ordinary target-design authoring. However, `quoin anomaly refs` overlaps the existing `quoin graph` reverse-dependency views (FR-062), and `quoin anomaly check` is a located-defect gate of the same kind as `quoin validate`. Say why a new `anomaly` topic is used rather than a `graph` view and a `validate` gate, or move them there | spec/functional/FR-136-anomaly-record-mapping.md:47-52 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
