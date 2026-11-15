---
id: SR-20261114002436
title: "Code review — PLAT-1615 M2 dossier document content (quoin#929)"
type: SpecReview
analysis: code-review
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, spec/functional/FR-196..FR-201, spec/functional/index.md, spec/non-functional/NFR-041-m2-dossier-states-only-recorded-values.md, spec/non-functional/index.md, spec/stakeholder/StR-010-reviewers-receive-documents-generated-from-records.md, spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-191", type: references }
  - { target: "ix://agent-ix/quoin/FR-196", type: references }
  - { target: "ix://agent-ix/quoin/FR-199", type: references }
---

## Summary

Ticket: PLAT-1615. PR agent-ix/quoin#929 at b0a203c0, diff `origin/main...HEAD` (12 files, spec-only, no Rust or production code). Reviewed the diff for internal correctness: links, cross-document consistency of the FR-191 command frame with the six new content FRs, and wording that two implementers would read differently. No Rust changed, so the rust-review lane did not apply.

## Verdict

**FAIL** — one cross-requirement contradiction (FR-191's gap-subject rule vs FR-196's document-level `no_evidence`) and one broken relative link. The FR-191 amendment otherwise keeps the M1 ACs intact and scopes them to M1-only registration (AC-2, AC-8), the index entries match the file titles, and StR-010 / US-024 carry the new `satisfied_by` edges and M2 wording consistently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-196 emits `no_evidence` "for the document" on an empty matrix, but FR-191 says a `no_evidence` gap names a criterion; the two requirements contradict. | spec/functional/FR-196-dossier-verification-report-content.md:68-69; spec/functional/FR-191-dossier-command.md:101 |
| FND-002 | medium | FR-199's Dependencies link `./FR-145-risk-advice-command.md` is broken; the file is `FR-145-risk-advise-command.md`. | spec/functional/FR-199-dossier-risk-management-content.md:86 |
| FND-003 | low | FR-191 bundle order "in registered order ... as their generators register" can be read as order of registration rather than the fixed id order AC-18 pins. | spec/functional/FR-191-dossier-command.md:54 |

## Coverage

- Examined: FR-191 statement, Inputs, Outputs, bundle-order behavior, selection, sources, document set, gaps section, AC-2, AC-8, AC-18, AC-19, Dependencies; FR-196..FR-201 statements and Dependencies; NFR-041 statement; StR-010 need and VC-1..VC-4; US-024 story, context, EX-1, EX-4; both index files.
- Link check: every relative link in the diff resolved against the tree except FR-199:86.
- Tools: `quire validate` and `quoin` could not be run in this session (permission layer requires approval for both); no builds or tests by brief.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-196 now emits a document-level `no_requirements` for an empty matrix (as FR-194 does), and FR-191's subject rule names the document for `no_requirements`, matching FR-191:90. |
| FND-002 | fixed | FR-199 Dependencies now links `./FR-145-risk-advise-command.md`. |
| FND-003 | fixed | FR-191 bundle order is now "this fixed dossier order, independent of generator registration time". |
