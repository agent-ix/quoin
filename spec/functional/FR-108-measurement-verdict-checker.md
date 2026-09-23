---
id: FR-108
title: "Independent measurement-verdict checker"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-107"
    type: "references"
---

# FR-108: Independent measurement-verdict checker

## Description

`quoin measurement verify --plan <id>` SHALL decide a `MeasurementPlan`'s
verdict from the plan and every stored collection that measured it,
independent of the producer that measured it, and SHALL report the verdict
as `accept`, `reject` or `inconclusive` with typed reason codes and counts.

The checker is pure: it takes the plan, every stored collection with its
intake position, and an optional claimed verdict, and reads no file, clock,
network or environment. It computes no estimate with producer code: it
recomputes `proportion` and `count` from each observation's `population`, and
applies the plan's decision rule through engineering-assurance's own
`DecisionRule::holds` (engineering-assurance FR-021).

## Rationale

Nothing decided a measurement verdict from the data. A producer's own
aggregate `value`, its own ordering of its own runs, and its own claim about
the outcome were all taken as stated. A small checker that shares nothing
with the producer, and says `inconclusive` rather than `accept` whenever the
data cannot carry a verdict, is what lets a verdict be granted credit.

## Inputs

- One `MeasurementPlan`: its `id`, `metric`, `definition_version`,
  `objective`, and `statistical_design.estimator`, `.decision_rule`,
  `.minimum_population` and `.repetitions`.
- Every stored collection in the repository's measurement store.
- An intake order: collection ids grouped by the git commit that first added
  each file under the store, earliest first. The store records no intake
  order of its own.
- Optionally, a claimed verdict.

## Outputs

A `quoin.measurement-verdict.v1` JSON document, the one engineering-assurance
reads:

| member | type | meaning |
| --- | --- | --- |
| `schema` | string | `quoin.measurement-verdict.v1` |
| `planId`, `definitionVersion` | string | the plan checked |
| `verdict` | `accept` \| `reject` \| `inconclusive` | the checker's verdict |
| `reasons` | string[] | every distinct reason code, in the declaration order of the table below; empty exactly when `accept` |
| `claimed` | string \| null | the claimed verdict, when one was given |
| `candidate` | string \| null | the collection id decided: the last run in intake order |
| `decisions` | object[] | one per slice of the candidate: `dimensions`, `estimate`, `estimateBasis` (`recomputed` \| `asserted`), `baseline` (number \| null), `holds` (boolean \| null) |
| `findings` | object[] | every reason with where it was found: `reason`, `collectionId` (null for a plan-level reason), `dimensions` (null for a collection- or plan-level reason) |
| `regressedRuns` | string[] | the runs the rule does not hold for against their own history, in intake order |
| `counts` | object | `collectionsConsidered`, `regressedRuns`, `observationsRecomputed`, `observationsAsserted`, `orderAttested`, `orderUnattested` |

Every member is always present. `accept` exits 0; `reject` and `inconclusive`
exit 1 with the complete document on stdout and a `CORE_NOT_ACCEPTED`
diagnostic.

### Reason codes

| code | verdict | when |
| --- | --- | --- |
| `no_decision_rule` | inconclusive | the plan states no `decision_rule` |
| `no_estimator` | inconclusive | the plan states no `estimator` |
| `no_collections` | inconclusive | no stored collection measured the plan's metric under its id and `definition_version` |
| `no_value` | inconclusive | the candidate's observation carries no measured value |
| `population_unstated` | inconclusive | no `population`, `examined` or `complete`, or no `repetitions` when the plan requires more than one |
| `population_incomplete` | inconclusive | `complete: false` |
| `population_empty` | inconclusive | `examined: 0` |
| `no_prior` | inconclusive | a baseline rule has no earlier usable run |
| `order_unattested` | inconclusive | the candidate, or a `prior-collection` rule's prior, shares its intake position with another run |
| `constant_predictor_rows_absent` | inconclusive | a `constant-predictor` baseline needs per-item answers by answer family, which no collection carries |
| `rule_not_evaluable` | inconclusive | engineering-assurance could not evaluate the rule on these numbers |
| `rule_not_met` | reject | the rule does not hold for the candidate |
| `value_disagrees_with_rows` | reject | a stored `value` differs from the estimate recomputed from `matched` and `examined`, in any run |
| `population_below_minimum` | reject | `examined` is below `minimum_population`, in any run |
| `repetitions_short` | reject | `repetitions` is below the plan's, in any run |
| `population_malformed` | reject | `examined`, `matched` or `repetitions` is not a whole number, or `matched` exceeds `examined`, in any run |
| `rerun_until_pass` | reject | a regressed run with the candidate's own apparatus preceded it |
| `claimed_verdict_disagrees` | reject | the claimed verdict is not the checker's |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-108-AC-1 | A `MeasurementPlan`'s `statistical_design.estimator` and `.decision_rule` are read when present, as engineering-assurance's `Estimator` and `DecisionRule`. A value engineering-assurance refuses, a `constant-predictor` baseline under an estimator other than `proportion`, or a comparator that disagrees with the plan's `objective` refuses the plan load as `QM-PLAN-INVALID`, naming the member. | Test (TC-1780, TC-1781) |
| FR-108-AC-2 | Every collection holding an observation of the plan's metric under the plan's id and `definition_version` is a run. Runs are ordered by intake position — the git commit that first added the collection file — with unpositioned runs after every positioned one; the stated `timestamp` only breaks ties. The candidate is the last run. When the candidate, or a `prior-collection` rule's prior, shares its intake position with another run, the verdict is not `accept` and carries `order_unattested`. A lone run needs no order. | Test (TC-1782, TC-1791, TC-1797) |
| FR-108-AC-3 | The rule is applied to every run against the runs before it; each run it does not hold for is listed in `regressedRuns` and counted. `prior-collection` is the latest earlier usable run's estimate and `best-seen` the maximum (`gt`, `ge`) or minimum (`lt`, `le`) of the earlier usable estimates, each computed from recomputed estimates; a run whose stored value disagrees with its rows contributes none. `constant-predictor` is `inconclusive` with `constant_predictor_rows_absent`. A regressed run whose source revision, configuration digest, tool, corpus revision and verification-stack lock and executable digests equal the candidate's, followed by a candidate the rule holds for, is `rerun_until_pass`. | Test (TC-1784, TC-1785, TC-1790, TC-1792, TC-1797) |
| FR-108-AC-4 | An observation's estimate is recomputed from its `population` — `matched / examined` for `proportion`, `matched` for `count` — and the stored `value` must equal it exactly, or the run is `value_disagrees_with_rows`. `mean`, `median` and `ratio` take the stored value and count the observation as asserted. `observationsRecomputed` and `observationsAsserted` count every run's observations. An empty, incomplete or unstated population, including an unstated `complete`, is `inconclusive`, never `accept`; a population below `minimum_population`, short of `repetitions` or malformed is `reject`. | Test (TC-1782, TC-1784, TC-1786..TC-1788, TC-1793, TC-1794) |
| FR-108-AC-5 | The verdict is `reject` when any finding's reason is a reject reason, otherwise `inconclusive` when any is an inconclusive reason, otherwise `accept`. A claimed verdict that differs adds `claimed_verdict_disagrees` and the verdict is `reject`. | Test (TC-1783, TC-1789, TC-1795) |
| FR-108-AC-6 | The result is the `quoin.measurement-verdict.v1` document under Outputs, every member present, and each reason and verdict spelling round-trips. | Test (TC-1795) |
| FR-108-AC-7 | `quoin measurement verify` exits 0 on `accept`; on `reject` or `inconclusive` it exits 1 with the complete document and a `CORE_NOT_ACCEPTED` diagnostic. An unknown plan id is refused (exit 2) and an unknown claimed verdict is a bad request (exit 3), each with no verdict. A plan recorded through `quoin measurement record` and verified is accepted; the same store with one observation's stored value edited is rejected with `value_disagrees_with_rows`. | Test (TC-1796, TC-1797) |

## Constraints

- **FR-108-CON-1**: The checker performs no I/O and reads no clock; the
  command's I/O is the store read and one `git log` for the intake order.
- **FR-108-CON-2**: The decision rule's comparator and margin semantics are
  engineering-assurance's; the checker calls `DecisionRule::holds` and states
  no comparison of its own.
- **FR-108-CON-3**: `rawEvidence` is not read: no schema says what its
  members mean.

## Dependencies

- FR-044 defines the plan-governed store the checker reads.
- engineering-assurance FR-020 and FR-021 own the `objective`, `estimator`
  and `decision_rule` types and the rule's evaluation.
