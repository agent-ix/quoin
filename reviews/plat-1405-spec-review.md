---
id: SR-20261112024901
title: "Spec review of quoin PR 827 (PLAT-1405 MP-219 span-breadth label adjudication)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin PR 827; bench/span-breadth-v1-labels.json (FR-030-AC-13, FR-031-AC-23, FR-030-AC-3, FR-030-AC-8, FR-057-AC-4), spec/assurance/MP-219-span-breadth.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/MP-219"
    type: "reviews"
---

# Spec review of quoin PR 827

## Summary

Ticket: PLAT-1405. Reviewer: Claude Opus 5, an independent agent review delegated by Peter. This is not a human adjudication. I judged each redrafted label against the current spec row and the MP-219 span definition before I looked at classifier output. Tool: quoin 0.30.1-5-ge402352b, quire CLI 0.40.0.

## Verdict

**Five labels pass. One of them needed its spans corrected. The replay reproduces 37 of 38 labels, and the one miss is a classifier defect, not a label error.**

| Label | Verdict |
| --- | --- |
| FR-030-AC-13 | pass. domain "store file", precondition "that exists and is not readable JSON", oracle to the end of the statement |
| FR-031-AC-23 | corrected, then pass. The appositive "an engine predating quire-rs CR-091" is part of the condition, so it closes the precondition and the oracle opens at "sets `degraded`". The redraft split the condition at "an engine \| predating" and used character offsets where the file uses byte offsets |
| FR-030-AC-3 | pass. refusal `span:refused-no-explicit-domain`: two coordinated clauses, no quantified noun domain |
| FR-030-AC-8 | pass. refusal `span:refused-no-explicit-domain`: the subject is the `gc` command, with chained behaviours |
| FR-057-AC-4 | pass. refusal `span:refused-no-explicit-domain`: the subject is a gerund, the same as the reviewed "Loading two paths ..." and "Replacing a section's content ..." labels |

The deletions of FR-029-AC-12 (withdrawn) and FR-029-AC-14 (row gone) are correct. Replacing FR-048-AC-2 is also correct: its row now states an opaque UUID and no longer states a content-derived id. The file holds 38 labels, 19 with spans and 19 refusals, across 8 property shapes and 2 repositories, which matches MP-219.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The classifier opens FR-031-AC-23's oracle at the appositive participle "predating", which moves part of the condition into the oracle. The label keeps the correct spans, so MP-219 replays 37 of 38 and its `ge 1.0` rule is not met until the classifier is fixed. Filed as PLAT-1470 | bench/span-breadth-v1-labels.json, spec/functional/FR-031-catalog-driven-advisor.md:141 |
