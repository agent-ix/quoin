---
id: FR-132
title: "Paired-run metrics computed from two compared collections"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "references"
---

# FR-132: Paired-run metrics computed from two compared collections

## Description

A metric whose claim is that two runs agree (repeat-reconcile mutations, catalog
ordering variance across runs) SHALL be an observation computed from a paired
comparison of two measurement collections under a relation the governing
`MeasurementPlan` states, never an assertion made by a single run. A plan states
the relation in an optional `paired_run` block: `relation` (`equal`,
`within_delta` or `not_worse`), `minimum_pairs` (a whole number of at least 1, 1 when the plan states none)
and, exactly when the relation is `within_delta`, a `delta` greater than 0.

## Rationale

A single run observes one member of a distribution over runs and cannot tell
two agreeing runs from one run. A pair that compared nothing must not read as
zero variance. The comparison itself is FR-044's; this requirement only
reduces its slices to the plan's stated relation.

## Behavior

- Quoin compares the two collections with the FR-044 comparison, so a moved
  definition, a changed build profile (FR-044-AC-15) or moved protected
  apparatus blocks the pair as it blocks any comparison.
- The observed value is the number of comparable slices of the plan's metric
  that violate the relation; the metric's target is 0.
- A slice that only one run states is a violation of every relation; a slice either run states without a value makes the pair `inconclusive`.
- `not_worse` ranks values by the plan's `objective` direction.
- `quoin measurement verify` and `quoin report` evaluate a plan that declares `paired_run` over the plan's latest two collections, so the count is computed by the CLI and never asserted by the producer. Both select a plan's collections with one function: a collection belongs to the plan when it holds an observation of the plan's metric under the plan's id and `definition_version`. The latest two are the last two in the order `verify` attests (intake order, FR-108-AC-2). `report` obtains the order from the same git history by the same function as `verify` and chooses and attests the pair by the same function, so a plan with three or more collections is decided by `report` exactly as `verify` decides it. Where the order of the latest two cannot be attested (the relation is `not_worse`, or more than two collections measured the plan, and the order is missing, shallow, tied or contradicted by the stated timestamps) `report` states the pair `inconclusive` with `order_unattested` and no count. A repository that is not a git work tree, or where git cannot run, yields no order and the same result; the report still renders. With two collections under `equal` or `within_delta` no order is needed and both name the same pair, except in a shallow clone (FR-108-AC-8), which attests no order for any plan. The `--portfolio` view reads no git and keeps `order_unattested` for such pairs.
- The paired relation is the plan's decision: a plan that declares `paired_run` and states no `decision_rule` is not `no_decision_rule`, whatever its `estimator`; one that states a `decision_rule` but no `estimator` keeps `no_estimator`. A plan that states a `decision_rule` and an `estimator` is judged by both, and a plan without `paired_run` is judged as before.
- `verify` maps the pair to its verdict: a count of 0 is `accept`; a count above 0 is `reject` with `paired_run_differs`; an `incomparable` pair is `inconclusive` with `paired_run_incomparable`; an `inconclusive` pair is `inconclusive` with `paired_run_inconclusive`; fewer than two collections is `inconclusive` with `paired_run_unpaired`. The count is a `decisions` entry marked `kind: "paired_run"` (empty `dimensions`, `estimate` the count, `baseline` 0, `holds` whether it is 0), which a slice decision never carries. This verdict is FR-108's; the FR-132 layer still emits only a count and a status.
- `report` prints a "Paired runs" section only when an active plan declares `paired_run`, with the metric, plan, relation, status (`observed`, `incomparable`, `inconclusive` or `unpaired`), count, comparable slices, `minimum_pairs` and the reasons; the JSON `pairedRuns` carries the same, `relation` included.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-132-AC-1 | Under `equal`, `compare_paired_runs` observes the count of comparable slices whose two values are not equal; identical runs observe 0. | Test |
| FR-132-AC-2 | Under `within_delta`, a slice violates the relation when its two values differ by more than `delta`, in either direction; a difference equal to `delta` is admitted. | Test |
| FR-132-AC-3 | Under `not_worse`, a slice violates the relation when the later value is worse than the earlier under the plan's `objective` direction; the same two runs count differently when the direction or the order of the runs reverses. | Test |
| FR-132-AC-4 | A pair with fewer comparable slices than `minimum_pairs` is `inconclusive` and states no count: zero comparisons never read as zero variance. A slice only one run states counts as one violation of every relation and is not a comparison, so it is counted even when enough shared slices remain. A slice either run states without a value is neither agreement nor disagreement: it is not a comparison, and the pair is `inconclusive` with no count. | Test |
| FR-132-AC-5 | A pair whose comparison is blocked (definition changed, profile changed, apparatus changed, incomplete population) is `incomparable`, states no count, and names each blocking reason. | Test |
| FR-132-AC-6 | A plan stating no `paired_run` yields no paired-run observation. | Test |
| FR-132-AC-7 | A `paired_run` that is not an object, lacks `relation`, names an unknown relation, states a `minimum_pairs` that is not a whole number of at least 1, states a `delta` that is not a number greater than 0, any member other than those three, `delta` without `within_delta` or no `delta` with `within_delta`, or declares `not_worse` without an `objective` that ranks two values, refuses the plan load as `QM-PLAN-INVALID`. A plan stating no `minimum_pairs` loads and requires 1. | Test |
| FR-132-AC-8 | `quoin measurement verify` on a plan declaring `paired_run`, with two recorded collections, observes the FR-132 count through `compare_paired_runs`: a count of 0 is `accept` (exit 0) with one `decisions` entry whose `estimate` is 0 and `holds` true; a count above 0 is `reject` with `paired_run_differs` (exit 1) and `holds` false. That entry carries `kind: "paired_run"`. Under `within_delta` a difference inside `delta` is `accept` and one beyond it is `reject`. | Test |
| FR-132-AC-9 | A plan declaring `paired_run` and no `decision_rule` yields no `no_decision_rule`, whatever its `estimator`, and with no estimator either, no `no_estimator` or `no_collections`; a plan stating a rule but not an estimator keeps `no_estimator`; a plan stating a rule and an estimator is judged by both; a plan without `paired_run` is verified exactly as before, reason `no_decision_rule` included. | Test |
| FR-132-AC-10 | With only one collection of the plan, or none, `verify` is `inconclusive` with `paired_run_unpaired` and states no count. With three or more, the pair is the latest two in intake order: an earlier differing collection does not change the result. | Test |
| FR-132-AC-11 | A pair blocked by a changed build profile is `inconclusive` with `paired_run_incomparable` and states no count and no `decisions` entry; a pair with fewer comparable slices than `minimum_pairs` or a slice without a value is `inconclusive` with `paired_run_inconclusive`; a slice only one collection states is counted as a violation and rejects with `paired_run_differs`. | Test |
| FR-132-AC-12 | When the relation is `not_worse` or the plan has more than two collections, a pair whose place in the intake order is not attested (FR-108-AC-2) adds `order_unattested` to `verify`; two collections under `equal` or `within_delta` need no order, except in a shallow clone, which attests no order for any plan (FR-108-AC-8) and adds `order_unattested` to every paired plan with two or more collections. `report` never states a count `verify` would not reach. | Test |
| FR-132-AC-13 | `quoin report` prints the "Paired runs" section for each active plan declaring `paired_run` over the plan's latest two collections, with the relation, status and count of FR-132-AC-8 to AC-11 (`unpaired` for fewer than two collections), and the report JSON carries the same as `pairedRuns`; a report with no such plan renders and serialises without it. | Test |
| FR-132-AC-15 | `quoin report` takes the intake order from git by the same function as `verify`, so for a plan with two or more collections whose order is attested it chooses the same two collections and states the same count and status as `verify`, whatever an earlier collection holds and under `not_worse` in either direction. | Test |
| FR-132-AC-16 | Where `verify` adds `order_unattested` to a paired plan (a tie, a shallow clone, a stated timestamp that contradicts the intake order, or no git order at all) `report` states the pair `inconclusive` with `order_unattested` and no count; where git cannot answer (not a work tree, or git cannot run) `report` still renders and says why on stderr. | Test |
| FR-132-AC-14 | `verify` and `report` select a plan's collections with the same rule: an observation of the plan's id, metric and `definition_version`. A collection of another definition version, or of another plan sharing the metric, is not one of the pair in either. | Test |

## Constraints

- No second comparison engine: every slice is compared by FR-044's comparison, in the CLI too: `verify` and `report` call `compare_paired_runs`.
- This layer emits a count and a status; it emits no quality verdict.

## Dependencies

- FR-044 supplies collection comparison and its blocking reasons.
- The separately installed engineering-assurance module defines `MeasurementPlan`
  and its `paired_run` member.
