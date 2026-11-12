---
id: FR-131
title: "A plan-declared population set is the denominator of its percentage metric"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-131: A plan-declared population set is the denominator of its percentage metric

## Description

Where a `MeasurementPlan` declares `statistical_design.population_set`, Quoin SHALL
take the plan's percentage metric over that set of ids: an observation states the
ids it examined and the ids that matched as `population.examined_ids` and
`population.matched_ids`, and intake and `quoin measurement verify` SHALL accept the
observation only when `examined_ids` is exactly the declared set and `matched_ids`
is drawn from it. A plan whose estimator is `proportion` and that declares neither a
set nor `statistical_design.population_exempt` SHALL be inconclusive.

## Rationale

Where a plan declares its population set, the denominator is checkable. A ratio such as "100% of failure paths emit an actionable message" is a number
about whichever population the producer happened to count. Five passing tests
over five chosen paths report 5 of 5. A population the plan enumerates, and
that a producer cannot shrink, makes the denominator a checkable set rather than
an asserted count. The producer's lists are evidence about the set, never the set.

## Behavior

- `statistical_design.population_set` is `{ "ids": [...] }`: a non-empty list of distinct, non-empty strings. engineering-assurance's plan schema declares the member, and `population_exempt`, as optional members of `statistical_design` (engineering-assurance#206). The set is mechanical to generate from a code property (for example the error-code taxonomy) by whatever authors the plan.
- A plan with no `population_set` is not held to any enumeration, except that a `proportion` plan declaring none must instead state `statistical_design.population_exempt`: a non-empty reason its denominator is not a set of ids (for example, repeated passes of a live service). A `proportion` plan stating neither is inconclusive in `report` (`population_set_undeclared`) and in `quoin measurement verify` (`population_set_undeclared`), whatever its observations say.
- The set is generated from the code property it enumerates and compared with the plan by a test, so a hand-typed list cannot drift from it: the commands of the `quoin` command catalogue (NFR-003), the workflow commands crossed with the ways `ix-flow` fails (NFR-007), and the manifest-fixture kinds that are corrupt or missing (NFR-008).
- `examined_ids` and `matched_ids` are lists of distinct, non-empty strings. `examined` and `matched` counts are stated, numeric, and equal their lists' lengths.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-131-AC-1 | A plan's `statistical_design.population_set` is read when present. A value that is not an object with exactly an `ids` list, an empty list, an empty id, a repeated id, or a non-string id refuses the plan load as `QM-PLAN-INVALID`, naming the member. A plan without it loads exactly as before. | Test (TC-2400, TC-2401) |
| FR-131-AC-2 | Under a plan declaring a `population_set`, intake of a new collection admits a `measured` observation whose `population.examined_ids` is exactly the declared set and whose `population.matched_ids` is a subset of it, with each stated count equal to its list's length. | Test (TC-2402) |
| FR-131-AC-3 | Under such a plan, a `measured` observation stating no `examined_ids`, `matched_ids`, `examined` or `matched` is refused as `QM-POPULATION-UNSTATED`, naming the member, and one whose `population` is not an object as `QM-POPULATION-MALFORMED`; a list that is not a list of strings or lists an id twice or empty, and a count that is not a number or disagrees with its list, are refused as `QM-POPULATION-MALFORMED`. A `not_computed` observation is held to none of this. | Test (TC-2403, TC-2404) |
| FR-131-AC-4 | Under such a plan, a list naming an id outside the declared set is refused as `QM-POPULATION-OUTSIDE-SET`, naming the ids; a producer's `examined_ids` omitting any declared id, including a producer choosing a convenient subset, is refused as `QM-POPULATION-PARTIAL`, naming the omitted ids. A stated count never substitutes for the list. | Test (TC-2405, TC-2406) |
| FR-131-AC-5 | `quoin measurement verify` applies FR-131-AC-3 and FR-131-AC-4 to the observations it assesses under a plan declaring a `population_set`: an unstated list or count is `population_unstated` (inconclusive), a malformed list or disagreeing count `population_malformed`, an id outside the set `population_outside_set`, and an omitted id `population_partial` (each reject). | Test (TC-2407) |
| FR-131-AC-6 | A refusal under a plan declaring a `population_set` stays bounded however large the set or the input: a finding names at most five ids, each cut to 64 characters, followed by a count of the rest, and a refusal spells out at most 100 findings followed by a count of the remainder. | Test (TC-2408) |
| FR-131-AC-7 | A plan's `statistical_design.population_exempt` is read when present. A value that is not a non-empty string, or a plan declaring both a `population_set` and a `population_exempt`, refuses the plan load as `QM-PLAN-INVALID`, naming the member. | Test (TC-3000) |
| FR-131-AC-8 | A plan with `estimator: proportion` that declares neither a `population_set` nor a `population_exempt` yields no usable value in a stage verdict (inconclusive `population_set_undeclared`), and `quoin measurement verify` is inconclusive `population_set_undeclared` for it; a plan declaring either, or a non-proportion estimator, is unaffected. | Test (TC-3001, TC-3002) |
| FR-131-AC-9 | NFR-003-M-1 and NFR-003-M-2 are measured by plans MP-250 and MP-251 over the commands of the `quoin` catalogue, enumerated from the binary itself; each plan's declared set equals that enumeration, and the producer's observation, probing every command with an unknown flag, is accepted at intake and read by `quoin measurement verify`. | Test (TC-3003) |
| FR-131-AC-10 | NFR-007-M-1 is measured by MP-252 over every workflow command crossed with `ix-flow` absent and `ix-flow` failing; the declared set equals the enumeration, and the producer's observation, running each pair against the real binary, is accepted at intake and read by `quoin measurement verify`. | Test (TC-3004) |
| FR-131-AC-11 | NFR-008-M-1 and NFR-008-M-2 are measured by plans MP-253 (corrupt manifests abort assembly) and MP-254 (manifest-less candidates are skipped) over the manifest fixture kinds the producer builds; each declared set equals that enumeration, and the producer's observation, running every candidate through `quoin catalog list`, is accepted at intake and read by `quoin measurement verify`. | Test (TC-3005) |
| FR-131-AC-12 | MP-244, MP-219, MP-229 and MP-230 declare population sets taken from their sources: MP-244 the criterion-strength fixtures (and the aggregate `graded_corpus_collection` stores names exactly them), MP-219 the frozen labels, MP-229 and MP-230 the bars they combine, each a plan in `spec/assurance`; a test fails when a declared set differs. | Test (TC-3006) |
| FR-131-AC-13 | MP-219 is measured by replaying every label of `bench/span-breadth-v1-labels.json` through the property classifier of the repository the label names: the producer reads the ids from the label file, states each label as examined, and states as matched those whose statement (the label's own `statement`, whitespace collapsed) is unchanged and whose reviewed spans (or reviewed refusal) the classifier reproduces; each unmatched label carries its reason (row missing, statement drift, or outcome differs). A label whose repository checkout is not supplied makes the replay refuse; it never states a smaller examined set. The replay over the full label set needs a quire-rs checkout and is not part of the default gate. | Test (TC-3300, TC-3301) |
| FR-131-AC-14 | MP-229 and MP-230 observations are produced from the bars a gate run computed: the producer states each bar's plan id as examined, those met as matched, and validates the observation against the plan before it is kept, so an omitted bar is refused at intake and an unmet bar stays out of `matched_ids`. | Test (TC-3302) |

## Constraints

- No id list, digest or manifest is stored beyond the plan's declaration and the observation's lists.
- Naming a plan's set for a given metric is the plan author's; FR-131-AC-9 to FR-131-AC-11 bind the five NFR metrics. NFR-002-M-2 (duplicate-module lists sorted) is a single-run property of one catalog and is not a population percentage.

## Dependencies

- FR-044 supplies plan-governed intake; FR-108 supplies the verifier.
