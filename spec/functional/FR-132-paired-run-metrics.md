---
id: FR-132
title: "Paired-run metrics computed from two compared collections"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-132-AC-1 | Under `equal`, `compare_paired_runs` observes the count of comparable slices whose two values are not equal; identical runs observe 0. | Test (TC-2500) |
| FR-132-AC-2 | Under `within_delta`, a slice violates the relation when its two values differ by more than `delta`, in either direction; a difference equal to `delta` is admitted. | Test (TC-2501) |
| FR-132-AC-3 | Under `not_worse`, a slice violates the relation when the later value is worse than the earlier under the plan's `objective` direction; the same two runs count differently when the direction or the order of the runs reverses. | Test (TC-2502) |
| FR-132-AC-4 | A pair with fewer comparable slices than `minimum_pairs` is `inconclusive` and states no count: zero comparisons never read as zero variance. A slice only one run states counts as one violation of every relation and is not a comparison, so it is counted even when enough shared slices remain. A slice either run states without a value is neither agreement nor disagreement: it is not a comparison, and the pair is `inconclusive` with no count. | Test (TC-2503, TC-2507, TC-2508) |
| FR-132-AC-5 | A pair whose comparison is blocked (definition changed, profile changed, apparatus changed, incomplete population) is `incomparable`, states no count, and names each blocking reason. | Test (TC-2504) |
| FR-132-AC-6 | A plan stating no `paired_run` yields no paired-run observation. | Test (TC-2505) |
| FR-132-AC-7 | A `paired_run` that is not an object, lacks `relation`, names an unknown relation, states a `minimum_pairs` that is not a whole number of at least 1, states a `delta` that is not a number greater than 0, any member other than those three, `delta` without `within_delta` or no `delta` with `within_delta`, or declares `not_worse` without an `objective` that ranks two values, refuses the plan load as `QM-PLAN-INVALID`. A plan stating no `minimum_pairs` loads and requires 1. | Test (TC-2506, TC-2509) |

## Constraints

- No second comparison engine: every slice is compared by FR-044's comparison.
- This layer emits a count and a status; it emits no quality verdict.

## Dependencies

- FR-044 supplies collection comparison and its blocking reasons.
- The separately installed engineering-assurance module defines `MeasurementPlan`
  and its `paired_run` member.
