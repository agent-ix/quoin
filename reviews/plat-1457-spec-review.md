---
id: SR-20261112014602
title: "Spec review of quoin PR 822 FR-132 Behavior and AC-12, FR-108-AC-8 amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@f847c866; spec/functional/FR-132-paired-run-metrics.md, spec/functional/FR-108-measurement-verdict-checker.md, spec/matrix.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-132"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "reviews"
---

# Spec review of quoin PR 822

## Summary

Ticket: PLAT-1457. This review covers the amended FR-132 Behavior bullet on how
`verify`/`report` choose the pair, FR-132-AC-12, the one sentence added to FR-108-AC-8, and
the TC-3600..3606 matrix rows. It applies the integrity and EARS lenses to the edited text.
TC-3600..3606 are unique against origin/main and open PRs.

## Verdict

**One high finding. FR-132 and FR-108 contradict each other for a shallow clone with two
collections.** This is the spec side of code-review FND-001. The single rule recommended
there keeps FR-108-AC-8 as written and qualifies FR-132's "two collections need no order"
clause. The other edits are sound:

- The `--portfolio` sentence states the boundary honestly.
- The TC rows each name one behaviour and one test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-132-AC-12 (and the Behavior bullet) says "two collections under `equal` or `within_delta` need no order", and lists "a shallow clone" among the cases where `verify` adds `order_unattested`. FR-108-AC-8 says a shallow clone reports `git-shallow` and `order_unattested` unconditionally. For a paired plan with two collections in a shallow clone the two ACs prescribe different answers, and the code follows each in a different command. Amend FR-132 to "two collections under `equal` or `within_delta` need no order, except in a shallow clone, where FR-108-AC-8 holds every plan `order_unattested` in both `verify` and `report`" | spec/functional/FR-132-paired-run-metrics.md:40, spec/functional/FR-132-paired-run-metrics.md:60, spec/functional/FR-108-measurement-verdict-checker.md:193 |
| FND-002 | low | The sentence added to FR-108-AC-8 ("`quoin report` obtains the order by the same function (FR-132-AC-12); where it cannot, it renders with no order") is report behaviour inside the verify-checker FR. It restates FR-132's bullet, and no test in AC-8's Verification column (TC-1797, TC-1805) asserts it. TC-3605 asserts it but is tagged to FR-132-AC-12 only. Drop the sentence and let FR-132 own it, or tag TC-3605 with FR-108-AC-8 as well | spec/functional/FR-108-measurement-verdict-checker.md:193 |
| FND-003 | low | FR-132-AC-12 is now compound. It states five claims in one AC: verify adds `order_unattested`, report takes the order by the same function, the two decide the same pair and count, report is inconclusive in four named cases, and two collections need no order. Each is testable, and TC-3600..3606 split them, but a failure cannot name which claim broke. Consider splitting the report-side claims into their own AC | spec/functional/FR-132-paired-run-metrics.md:60 |
