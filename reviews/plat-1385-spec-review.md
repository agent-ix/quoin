---
id: SR-17078
title: "Spec review of quoin PR 798 (template requirement removal)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@9a6555ac; spec/spec.md, spec/*/index.md, spec/matrix.md, spec/functional/FR-116, FR-121, FR-122, FR-123, spec/non-functional/NFR-027, plan/PLAN-008/**"
review_set: subset
---

# Spec review of quoin PR 798

## Summary

Ticket: PLAT-1385. Removed: FR-076..FR-083, NFR-018/019/020, US-021, StR-008, PLAN-008 with
TASK-045..051, and their matrix rows. No surviving spec, plan or skill file carries an
`ix://agent-ix/quoin/` relationship to a removed id (historical SpecReviews under `reviews/` keep
theirs, as reviews are never edited). Indexes and `spec/spec.md` drop the entries; the
re-headed spec.md group holds only the withdrawn FR-084..FR-092. The NFR-027 edit drops the
NFR-020 citation without changing its rule. The Dependencies-line edits in FR-116/121/122/123 are
accurate: quire-rs FR-076 is `relationships-extraction` (typed relation keys) and quire-rs FR-078
is `qualified-evidence-binding` (the evidence bridge), matching each line's usage. `quire validate`
(quire 0.39.0) passes on FR-116 and FR-122.

## Verdict

**APPROVE with one low finding.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR qualifies bare `FR-076`/`FR-078` as quire-rs only in Dependencies lines; the same bare ids remain in the bodies and ACs and now name ids that no longer exist in quoin: FR-116 (FR-078, 8 places incl. AC-17), FR-121 (FR-076-AC-9), FR-122 (FR-076-CON-4, FR-076, AC-6), FR-123 (FR-076, AC-6), matrix TC-2114 | spec/functional/FR-116-owner-qualified-quire-evidence.md:44,57,69,70,99,207,216,252; spec/functional/FR-121-record-key-sections-mapping.md:62; spec/functional/FR-122-architecture-record-keys-mapping.md:36,42,103; spec/functional/FR-123-model-table-sections-mapping.md:79,101; spec/matrix.md:1350 |
