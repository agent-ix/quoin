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
- Every stored collection in the repository's measurement store. A
  collection whose file name is not its `collectionId` is refused: the
  intake order is keyed by file name.
- An intake order — collection ids grouped by position, earliest first —
  and its **order source**:

  | source | what it is |
  | --- | --- |
  | `git-first-parent-add` | the first-parent commit that added each file under the store; what `quoin measurement verify` supplies |
  | `git-shallow` | the repository is a shallow clone, so first-add commits are unknown; no position is used and the result carries `order_unattested` |
  | `caller-supplied` | positions a caller of the `measurement.verify` operation sent without naming a source |
  | `none` | no order |

- Optionally, a claimed verdict.

### The attestation boundary

The store records no intake order, and a collection's `timestamp` and
`collectionId` are the producer's to choose. The order the checker trusts
is therefore only as independent as its source: `git-first-parent-add` is an
order the producer cannot change after publishing without rewriting the
first-parent history of the branch the store is read from. It attests
nothing about a collection before it is committed (every uncommitted
collection is unpositioned), and nothing in a shallow clone. `caller-supplied`
positions are used as given and attest nothing; a consumer granting credit
reads `orderSource` before it trusts `orderAttested`.

### Tamper facts (PLAT-985)

`quoin measurement verify` also gathers four facts from the store's git
history, the same way it gathers the intake order — only the caller has git,
so the checker itself stays pure and takes these as inputs it does not
compute:

- a collection that was once added to the store's history and no longer
  exists there (`collection_deleted`), attributed to a plan by reading the
  content at the commit before it was removed;
- a collection whose stored file a commit edited after the one that first
  added it (`collection_edited`);
- a collection's recorded `verificationStack.protectedApparatus` digest for
  this plan disagreeing with `git show <sourceRevision>:<path>` — the file's
  real bytes at the commit the collection claims to be from
  (`apparatus_forged`), which catches a collection written by hand rather
  than through intake's own resolver (FR-110);
- the plan's own document changing its `objective`, `estimator`,
  `decision_rule` or `protected_apparatus` between two committed revisions
  that share a `definition_version` (`definition_changed_without_version_bump`,
  engineering-assurance's `definition_change_without_version_bump`).

Each degrades to "nothing found" rather than an error when git cannot answer
— outside a work tree, a shallow clone missing the revision in question, or a
path git does not track — the same posture the intake order takes: a caller
with no git access attests nothing, rather than manufacturing a false tamper
finding.

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
| `orderSource` | string | where the intake order came from, one of the sources above |
| `counts` | object | `collectionsConsidered`, `regressedRuns`, `observationsRecomputed`, `observationsAsserted`, `orderAttested`, `orderUnattested` |

Every member is always present. `accept` exits 0. `reject` exits 1 with the
complete document on stdout and a `CORE_REJECTED` diagnostic; `inconclusive`
exits 1 with the complete document and a `CORE_INCONCLUSIVE` diagnostic.

### Reason codes

| code | verdict | when |
| --- | --- | --- |
| `no_decision_rule` | inconclusive | the plan states no `decision_rule` |
| `no_estimator` | inconclusive | the plan states no `estimator` |
| `no_collections` | inconclusive | no stored collection measured the plan's metric under its id and `definition_version` |
| `no_value` | inconclusive | the candidate's observation carries no measured value |
| `population_unstated` | inconclusive | no `population`, `examined` or `complete`; no `matched` under `proportion` or `count`; or no `repetitions` when the plan requires more than one |
| `population_incomplete` | inconclusive | `complete: false` |
| `population_empty` | inconclusive | `examined: 0` under a plan with no `minimum_population` |
| `no_prior` | inconclusive | a baseline rule has no earlier usable run |
| `order_unattested` | inconclusive | the order source is `git-shallow`; or the candidate, or a `prior-collection` rule's prior, shares its intake position with another run, or a run on one side of it states a timestamp on the other side |
| `constant_predictor_rows_absent` | inconclusive | a `constant-predictor` baseline needs per-item answers by answer family, which no collection carries |
| `rule_not_evaluable` | inconclusive | engineering-assurance could not evaluate the rule on these numbers |
| `unit_unsupported` | inconclusive | a `proportion` observation's unit is not `fraction` or `fraction of …` |
| `slice_missing` | inconclusive | a slice an earlier run measured under this definition is absent from the candidate |
| `observation_missing` | inconclusive | a collection with the candidate's `subject` and `scope`, not before it in intake order, carries no observation of the plan |
| `apparatus_unrecorded` | inconclusive | in a protected series, the candidate or an earlier run recorded no protected apparatus (FR-110) |
| `rule_not_met` | reject | the rule does not hold for the candidate |
| `value_disagrees_with_rows` | reject | a stored `value` is inconsistent with the estimate recomputed from `matched` and `examined`, in any run |
| `population_below_minimum` | reject | the candidate's `examined` — `0` included — is below `minimum_population` |
| `repetitions_short` | reject | the candidate's `repetitions` is below the plan's |
| `population_malformed` | reject | `examined`, `matched` or `repetitions` is not a whole number, or `matched` exceeds `examined`, in any run |
| `rerun_until_pass` | reject | a regressed run with the candidate's own apparatus preceded it |
| `apparatus_edit` | reject | an earlier run under the same `definition_version` recorded a different protected apparatus than the candidate (FR-110) |
| `claimed_verdict_disagrees` | reject | the claimed verdict is not the checker's |
| `apparatus_forged` | reject | a run's recorded protected-apparatus digest for this plan disagrees with `git show <sourceRevision>:<path>` (PLAT-985) |
| `collection_deleted` | reject | a collection that measured this plan was added to the store's git history and no longer exists there (PLAT-985) |
| `collection_edited` | reject | a run's stored file was edited by a commit after the one that first added it (PLAT-985) |
| `definition_changed_without_version_bump` | reject | the plan's objective, estimator, decision rule or protected apparatus changed between two committed revisions sharing a `definition_version` (PLAT-985) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-108-AC-1 | A `MeasurementPlan`'s `statistical_design.estimator` and `.decision_rule` are read when present, as engineering-assurance's `Estimator` and `DecisionRule`. A value engineering-assurance refuses, a `constant-predictor` baseline under an estimator other than `proportion`, or a comparator that disagrees with the plan's `objective` refuses the plan load as `QM-PLAN-INVALID`, naming the member. | Test (TC-1780, TC-1781) |
| FR-108-AC-2 | Every collection holding an observation of the plan's metric under the plan's id and `definition_version` is a run, and `collectionsConsidered` counts them whatever else the plan lacks. Runs are ordered by intake position, with unpositioned runs after every positioned one; the stated `timestamp` only breaks ties. The candidate is the last run. The candidate, or a `prior-collection` rule's prior, is `order_unattested` when it shares its intake position with another run or when a run before it states a later timestamp (or one after it an earlier one); a lone run needs no order. A slice an earlier run measured that the candidate lacks is `slice_missing`, and a collection of the candidate's subject and scope, not before it, that carries no observation of the plan is `observation_missing`: a dropped measurement never lets an older pass stand. | Test (TC-1782, TC-1789, TC-1791, TC-1797, TC-1799, TC-1800, TC-1803) |
| FR-108-AC-3 | The rule is applied to every run against the runs before it; each run it does not hold for is listed in `regressedRuns` and counted. `prior-collection` is the latest earlier usable run's estimate and `best-seen` the maximum (`gt`, `ge`) or minimum (`lt`, `le`) of the earlier usable estimates, each computed from recomputed estimates; a run that is not usable evidence contributes none. From a run before the candidate only `value_disagrees_with_rows` and `population_malformed` are findings; its other shortfalls exclude it from baselines and are otherwise history. `constant-predictor` is `inconclusive` with `constant_predictor_rows_absent`. A regressed run whose source revision, configuration digest, tool, corpus revision and verification-stack lock and executable digests equal the candidate's, followed by a candidate the rule holds for, is `rerun_until_pass`. | Test (TC-1784, TC-1785, TC-1790, TC-1792, TC-1797, TC-1802) |
| FR-108-AC-4 | An observation's estimate is recomputed from its `population` — `matched / examined` for `proportion`, `matched` for `count` — and a `proportion` or `count` observation with no `matched` is `population_unstated`. The stored `value` is consistent when it is the recomputed `f64` or lies within half a unit of its last stated decimal (read from its shortest round-trip spelling) of the exact quotient; otherwise the run is `value_disagrees_with_rows`. The rule is applied to the recomputed estimate, never the stored value. A `proportion` whose unit is not `fraction` or `fraction of …` is `unit_unsupported`. `mean`, `median` and `ratio` take the stored value and count the observation as asserted. `observationsRecomputed` and `observationsAsserted` count every run's observations. An incomplete or unstated population, including an unstated `complete`, is `inconclusive`, never `accept`; under a `minimum_population` an `examined` below it, `0` included, is `population_below_minimum`, and with no minimum `examined: 0` is `population_empty`; a population short of `repetitions` or malformed is `reject`. | Test (TC-1782, TC-1784, TC-1786..TC-1788, TC-1793, TC-1794, TC-1798, TC-1801, TC-1804) |
| FR-108-AC-5 | The verdict is `reject` when any finding's reason is a reject reason, otherwise `inconclusive` when any is an inconclusive reason, otherwise `accept`. A claimed verdict that differs adds `claimed_verdict_disagrees` and the verdict is `reject`. | Test (TC-1783, TC-1789, TC-1795) |
| FR-108-AC-6 | The result is the `quoin.measurement-verdict.v1` document under Outputs, every member present, and each reason, verdict and order-source spelling round-trips. | Test (TC-1795, TC-1805) |
| FR-108-AC-7 | `quoin measurement verify` exits 0 on `accept`, and 1 with the complete document on `reject` (`CORE_REJECTED`) or `inconclusive` (`CORE_INCONCLUSIVE`). An unknown plan id, and a collection filed under a name that is not its `collectionId`, are refused (exit 2); an unknown claimed verdict or order source is a bad request (exit 3); each with no verdict. A plan recorded through `quoin measurement record` and verified is accepted; the same store with one observation's stored value edited is rejected with `value_disagrees_with_rows`. | Test (TC-1796, TC-1797, TC-1806) |
| FR-108-AC-8 | `quoin measurement verify` takes the intake order from `git log --first-parent --diff-filter=A` over the store and reports `orderSource: git-first-parent-add`. Outside a git work tree it reports `none` and says why on stderr; in a shallow clone it reports `git-shallow` and `order_unattested` and says so on stderr; any other git failure fails the command rather than yielding an empty order. | Test (TC-1797, TC-1805) |
| FR-108-AC-9 | `quoin measurement verify` reads four tamper facts from the store's git history (PLAT-985): a collection this plan governed that was added and later removed is `collection_deleted`, attributed by reading its content at the commit before removal; a collection's stored file edited by a commit after the one that added it is `collection_edited`; a run's recorded protected-apparatus digest for this plan disagreeing with `git show <sourceRevision>:<path>` is `apparatus_forged`; and the plan's own document changing its `objective`, `estimator`, `decision_rule` or `protected_apparatus` between two committed revisions sharing a `definition_version` is `definition_changed_without_version_bump`. Each is `false`/empty, not a false positive, when git cannot answer. | Test (TC-1892..TC-1896) |

## Leaf re-scoring and the asserted-only share (PLAT-961, PLAT-985)

A row's outcome is re-scored from the retained artifact, not trusted from the
producer's own pass/fail, exactly where the artifact is mechanical: a
`proportion` or `count` observation's `matched`/`examined` counts
([`rows::assess`](../../rust/crates/quoin-measurement/src/verify/rows.rs) —
FR-108-AC-4). `mean`, `median` and `ratio` carry no row data any retained
collection states (no per-item values, no two totals of a ratio), so their
stored `value` is taken as asserted rather than re-scored — there is nothing
mechanical to re-derive it from. `counts.observationsRecomputed` and
`counts.observationsAsserted` report the split for every run counted, so the
share of rows that were only asserted is `observationsAsserted /
(observationsRecomputed + observationsAsserted)`, always present in the
verdict document (Outputs, above).

## Known limits

- **Prior-collection laundering.** `prior-collection` compares the candidate
  with the latest earlier usable run and nothing else. A producer can lower
  that bar by recording a deliberately poor run under a *different* apparatus
  (a new source revision is enough) just before the candidate: it is not a
  rerun of the candidate's apparatus, so `rerun_until_pass` does not fire,
  and it becomes the prior. The regression is still listed in
  `regressedRuns` and counted, so it is visible; it is not refused. A plan
  that must not be gamed this way uses `best-seen` or a threshold.
- **A rounded aggregate is judged at its own precision.** A stored `1` states
  no decimals, so it is consistent with any quotient from 0.5 to 1.5. This is
  harmless to the verdict — the rule sees the recomputed estimate — but it
  means `value_disagrees_with_rows` catches only a stored value that is wrong
  at the precision it claims.
- **Constant-predictor baseline is still blocked.** MP-222/PLAT-932's formula
  — a size-weighted mean of the best-constant agreement per answer family —
  needs a producer to retain each item's own answer, grouped by family; no
  producer does today, so `constant_predictor_rows_absent` stays the
  checker's only answer for it. See PLAT-985's ticket comments for which
  producer change this depends on.
- **A forged record naming a source revision this repository never received
  is unattested, not caught.** `apparatus_forged` needs `git show
  <sourceRevision>:<path>` to succeed; a `sourceRevision` from a fetch this
  clone never did, or from another repository entirely, makes the check
  unable to answer, and it reports nothing rather than guessing. The same is
  true of `definition_changed_without_version_bump` over a shallow plan
  history and `collection_edited`/`collection_deleted` in a shallow clone —
  all four tamper facts need the same full first-parent history the intake
  order does.

## Constraints

- **FR-108-CON-1**: The checker performs no I/O and reads no clock; the
  command's I/O is the store read and the `git` calls for the intake order
  and the tamper facts (PLAT-985).
- **FR-108-CON-2**: The decision rule's comparator and margin semantics are
  engineering-assurance's; the checker calls `DecisionRule::holds` and states
  no comparison of its own.
- **FR-108-CON-3**: `rawEvidence` is not read: no schema says what its
  members mean.

## Dependencies

- FR-044 defines the plan-governed store the checker reads.
- engineering-assurance FR-020 and FR-021 own the `objective`, `estimator`
  and `decision_rule` types and the rule's evaluation, and FR-021's
  `definition_change_without_version_bump` is what
  `definition_changed_without_version_bump` (PLAT-985) applies over the
  plan's git history.
- FR-110 owns `protected_apparatus` resolution and digesting at intake;
  `apparatus_forged` (PLAT-985) checks the recorded digest against the
  store's own git history rather than trusting the record.
