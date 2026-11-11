---
id: SR-20261111201429
title: "Spec review of quoin PR 817 FR-132 AC-8..13 and FR-108 reason-table amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@d9a17f68; spec/functional/FR-132-paired-run-metrics.md, spec/functional/FR-108-measurement-verdict-checker.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-132"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "reviews"
---

# Spec review of quoin PR 817

## Summary

Ticket: PLAT-1404. This review covers the FR-132 Behavior bullets, FR-132-AC-8..13, the
Constraints edit, and the FR-108 reason-table rows: four new rows and the narrowed
`no_decision_rule` row. FR-107 is untouched: no FR-107 path is in the diff. That matches
the coder's note that gate plans must load with a `decision_rule`, so a ruleless paired
gate plan cannot occur.

## Verdict

**Sound overall. One medium and one low on FR-108, one medium and one low on FR-132.** The
ACs are atomic enough and testable, and every AC is backed. The mapping is honest:
zero comparisons never reads as zero difference.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-108's core text was not amended for paired plans. Its Description says the checker "applies the plan's decision rule through engineering-assurance's own `DecisionRule::holds`". Its `decisions` member is "one per slice of the candidate". A paired verdict uses neither: it decides without a rule and adds a count entry that is not a slice. FR-108 also has no relationship to FR-132. FR-132 says "This verdict is FR-108's", but FR-108 does not say so | spec/functional/FR-108-measurement-verdict-checker.md:16-30, spec/functional/FR-108-measurement-verdict-checker.md:123 |
| FND-002 | medium | The FR-132 Behavior bullet says the "Paired runs" section prints "the plan, relation, status, count, comparable slices, `minimum_pairs` and the reasons". The table columns are Metric, Plan, Status, Differing, Comparable, Required and Detail, and `pairedRuns` JSON has no relation member. Either drop "relation" from the bullet or add the column | spec/functional/FR-132-paired-run-metrics.md:40, rust/crates/quoin-measurement/src/report/paired_render.rs:18 |
| FND-003 | low | FR-132 lets `verify` take the last two collections in intake order and `report` take the last two in stored order (timestamp, then id). The two can name different pairs. Reproduced: intake a, b, c, where c's stated timestamp is the earliest. `verify` returns reject `[order_unattested, paired_run_differs]` while the report prints `observed`, differing 0. Specify one order for both, or state that the report may differ and `verify` governs | spec/functional/FR-132-paired-run-metrics.md:38 |
| FND-004 | low | The FR-108 `no_decision_rule` row says the code applies when "the plan states no `decision_rule` and declares no `paired_run`". FR-132 limits the exemption to plans stating neither `decision_rule` nor `estimator`, and the code follows FR-132. The two specs disagree for a paired plan that has an estimator but no rule. See code-review SR-20261111201427 FND-002 | spec/functional/FR-108-measurement-verdict-checker.md:137 |
