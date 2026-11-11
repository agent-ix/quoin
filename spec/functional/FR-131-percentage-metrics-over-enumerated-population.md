---
id: FR-131
title: "Percentage metrics are taken over a plan-enumerated population"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-131: Percentage metrics are taken over a plan-enumerated population

## Description

Where a `MeasurementPlan` declares `statistical_design.population_set`, Quoin SHALL
take the plan's percentage metric over that set of ids: an observation states the
ids it examined and the ids that matched as `population.examined_ids` and
`population.matched_ids`, and intake and `quoin measurement verify` SHALL accept the
observation only when `examined_ids` is exactly the declared set and `matched_ids`
is drawn from it.

## Rationale

A ratio such as "100% of failure paths emit an actionable message" is a number
about whichever population the producer happened to count. Five passing tests
over five chosen paths report 5 of 5. A population the plan enumerates, and
that a producer cannot shrink, makes the denominator a checkable set rather than
an asserted count. The producer's lists are evidence about the set, never the set.

## Behavior

- `statistical_design.population_set` is `{ "ids": [...] }`: a non-empty list of distinct, non-empty strings, in a member the engineering-assurance plan schema already leaves free-form. The set is mechanical to generate from a code property (for example the error-code taxonomy) by whatever authors the plan.
- A plan declaring none behaves exactly as before.
- `examined_ids` and `matched_ids` are lists of distinct, non-empty strings. A stated `examined` or `matched` count must equal its list's length.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-131-AC-1 | A plan's `statistical_design.population_set` is read when present. A value that is not an object with exactly an `ids` list, an empty list, an empty id, a repeated id, or a non-string id refuses the plan load as `QM-PLAN-INVALID`, naming the member. A plan without it loads exactly as before. | Test (TC-2400, TC-2401) |
| FR-131-AC-2 | Under a plan declaring a `population_set`, intake of a new collection admits a `measured` observation whose `population.examined_ids` is exactly the declared set and whose `population.matched_ids` is a subset of it, with each stated count equal to its list's length. | Test (TC-2402) |
| FR-131-AC-3 | Under such a plan, a `measured` observation stating no `examined_ids` or no `matched_ids` is refused as `QM-POPULATION-UNSTATED`, naming the member; a member that is not a list of strings, lists an id twice or empty, or disagrees with a stated count is refused as `QM-POPULATION-MALFORMED`. A `not_computed` observation is held to none of this. | Test (TC-2403, TC-2404) |
| FR-131-AC-4 | Under such a plan, a list naming an id outside the declared set is refused as `QM-POPULATION-OUTSIDE-SET`, naming the ids; a producer's `examined_ids` omitting any declared id, including a producer choosing a convenient subset, is refused as `QM-POPULATION-PARTIAL`, naming the omitted ids. A stated count never substitutes for the list. | Test (TC-2405, TC-2406) |
| FR-131-AC-5 | `quoin measurement verify` applies FR-131-AC-3 and FR-131-AC-4 to the observations it assesses under a plan declaring a `population_set`: an unstated list is `population_unstated` (inconclusive), a malformed one `population_malformed`, an id outside the set `population_outside_set`, and an omitted id `population_partial` (each reject). | Test (TC-2407) |

## Constraints

- No id list, digest or manifest is stored beyond the plan's declaration and the observation's lists.
- This criterion binds no metric to a set; naming a plan's set for a given metric is the plan author's.

## Dependencies

- FR-044 supplies plan-governed intake; FR-108 supplies the verifier.
