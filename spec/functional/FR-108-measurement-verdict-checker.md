---
id: FR-108
title: "Independent measurement-verdict checker"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-107"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-132"
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
`DecisionRule::holds`, or `DecisionRule::holds_on_interval` when the rule
states an `interval_level` (engineering-assurance FR-021). A plan that declares `paired_run` is decided by its paired relation instead, or in addition to a rule it states (FR-132).

`quoin-jev` SHALL provide a pure function that builds the complete graded-corpus
`MeasurementCollection` (the governed aggregate and one item row per graded
item) from a graded corpus, so that a producer's stored collection carries the
rows a `constant-predictor` baseline is recomputed from (PLAT-1021).

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

`quoin measurement verify` also gathers three facts from the store's git
history, the same way it gathers the intake order — only the caller has git,
so the checker itself stays pure and takes these as inputs it does not
compute:

- a collection that was once added to the store's history and no longer
  exists there, or that the uncommitted work tree removed
  (`collection_deleted`);
- a collection whose stored file was changed after intake first added it —
  by a later commit, by a delete and re-add under the same id, or by an
  uncommitted edit in the work tree (`collection_edited`);
- the plan's own document changing its `objective`, `estimator`,
  `decision_rule`, `statistical_design.answer_families` (a set: reordering is
  not a change) or `protected_apparatus` between two revisions that share
  a `definition_version` — adjacent committed revisions, each read at the
  path the document had in its own commit so a rename hides nothing, and the
  work-tree document against the last commit
  (`definition_changed_without_version_bump`, engineering-assurance's
  `definition_change_without_version_bump`).

A deleted or edited collection is attributed to a plan when that plan's id
appears among its observations' `planId`s in the content intake first added
*or* in the last content seen (before removal, or on disk now), so neither an
edit that moves a run off the plan nor one that corrupts the file before
deleting it leaves the tampering unattributed. Every value read from a
collection file reaches git after `--end-of-options`, so it is only ever a
revision, never an option.

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
| `decisions` | object[] | one per slice of the candidate, and for a plan declaring `paired_run` one more entry marked `kind: "paired_run"` whose `estimate` is the FR-132 count (a slice decision carries no `kind`): `dimensions`, `estimate`, `estimateBasis` (`recomputed` \| `asserted`), `baseline` (number \| null), `holds` (boolean \| null); when the rule states `interval_level` and an interval decided the slice, also `intervalBound` (`lower` \| `upper`), `intervalBoundValue` and `intervalLevel` (the level the deciding interval states, not the plan's required level, which is in the plan; FR-108-AC-11), otherwise absent |
| `findings` | object[] | every reason with where it was found: `reason`, `collectionId` (null for a plan-level reason), `dimensions` (null for a collection- or plan-level reason) |
| `regressedRuns` | string[] | the runs the rule does not hold for against their own history, in intake order |
| `orderSource` | string | where the intake order came from, one of the sources above |
| `counts` | object | `collectionsConsidered`, `regressedRuns`, `observationsRecomputed`, `observationsAsserted`, `orderAttested`, `orderUnattested` |

Every top-level member is always present; the three `decisions` interval members (`intervalBound`, `intervalBoundValue`, `intervalLevel`) are present only on a slice decided on an interval (FR-108-AC-11). Nothing is added at the top level. `accept` exits 0. `reject` exits 1 with the
complete document on stdout and a `CORE_REJECTED` diagnostic; `inconclusive`
exits 1 with the complete document and a `CORE_INCONCLUSIVE` diagnostic.

### Reason codes

| code | verdict | when |
| --- | --- | --- |
| `no_decision_rule` | inconclusive | the plan states no `decision_rule` and declares no `paired_run`; a paired plan is exempt whatever its `estimator` (FR-132-AC-9) |
| `no_estimator` | inconclusive | the plan states no `estimator` |
| `no_collections` | inconclusive | no stored collection measured the plan's metric under its id and `definition_version` |
| `no_value` | inconclusive | the candidate's observation carries no measured value |
| `population_unstated` | inconclusive | no `population`, `examined` or `complete`; no `matched` under `proportion` or `count`; or no `repetitions` when the plan requires more than one |
| `population_incomplete` | inconclusive | `complete: false` |
| `population_empty` | inconclusive | `examined: 0` under a plan with no `minimum_population` |
| `paired_run_unpaired` | inconclusive | the plan declares `paired_run` and fewer than two collections measured it (FR-132-AC-10) |
| `paired_run_incomparable` | inconclusive | the plan declares `paired_run` and its latest two collections cannot be compared (FR-132-AC-11) |
| `paired_run_inconclusive` | inconclusive | the plan declares `paired_run` and its latest two collections hold fewer comparable slices than `minimum_pairs`, or a slice without a value (FR-132-AC-11) |
| `no_prior` | inconclusive | a baseline rule has no earlier usable run |
| `order_unattested` | inconclusive | the order source is `git-shallow`; or the candidate, or a `prior-collection` rule's prior, shares its intake position with another run, or a run on one side of it states a timestamp on the other side |
| `constant_predictor_rows_absent` | inconclusive | a `constant-predictor` baseline needs per-item answers by answer family, and the run being decided carries no per-item observations for its metric (PLAT-1016) |
| `constant_predictor_rows_malformed` | inconclusive | one of the run's constant-predictor item observations lacks a non-empty string `item_id`, `family` or `expected`, states a `contested` that is not an array of strings, or repeats another's `item_id` (PLAT-1016) |
| `constant_predictor_rows_mismatch` | inconclusive | the run's constant-predictor item observations do not number the governed observation's own `population.examined` (PLAT-1016) |
| `constant_predictor_families_unstated` | inconclusive | a `constant-predictor` baseline is decided under a plan whose `statistical_design` states no `answer_families` (PLAT-1020) |
| `constant_predictor_family_unexpected` | inconclusive | the run's constant-predictor item rows carry a `family` the plan's `answer_families` does not declare, such as two declared families written under a new name (PLAT-1020) |
| `constant_predictor_family_missing` | inconclusive | a family the plan's `answer_families` declares has no constant-predictor item row, such as two declared families merged under one declared name (PLAT-1020) |
| `external_reference_unsupplied` | inconclusive | a baseline rule reads `external-reference`, a per-dimension value the checker resolves from a source outside the plan at evaluation time, and the checker has no such source (PLAT-1032) |
| `interval_unstated` | inconclusive | the plan's rule states `interval_level` and the candidate's observation states no valid `interval` (EA-26) |
| `rule_not_evaluable` | inconclusive | engineering-assurance could not evaluate the rule on these numbers |
| `population_set_undeclared` | inconclusive | the plan's estimator is `proportion` and it declares neither a `population_set` nor a `population_exempt` (FR-131-AC-8) |
| `unit_unsupported` | inconclusive | a `proportion` observation's unit is not `fraction` or `fraction of …` |
| `slice_missing` | inconclusive | a slice an earlier run measured under this definition is absent from the candidate |
| `observation_missing` | inconclusive | a collection with the candidate's `subject` and `scope`, not before it in intake order, carries no observation of the plan |
| `apparatus_unrecorded` | inconclusive | in a protected series, the candidate or an earlier run recorded no protected apparatus (FR-110) |
| `paired_run_differs` | reject | the plan declares `paired_run` and its latest two collections violate the relation in at least one slice (FR-132-AC-8) |
| `rule_not_met` | reject | the rule does not hold for the candidate |
| `value_disagrees_with_rows` | reject | a stored `value` is inconsistent with the estimate recomputed from `matched` and `examined`, in any run |
| `population_below_minimum` | reject | the candidate's `examined` — `0` included — is below `minimum_population` |
| `interval_malformed` | reject | under a plan whose rule states `interval_level`, a run states an `interval` that does not satisfy FR-044-AC-10, in any run (EA-26) |
| `interval_level_short` | reject | the candidate's `interval` level is below the rule's `interval_level` (EA-26) |
| `repetitions_short` | reject | the candidate's `repetitions` is below the plan's |
| `population_malformed` | reject | `examined`, `matched` or `repetitions` is not a whole number, or `matched` exceeds `examined`, in any run; or, under a plan declaring a `population_set` (FR-131), an id list is not a list of distinct non-empty strings or a count disagrees with its list |
| `population_outside_set` | reject | under a plan declaring a `population_set` (FR-131), an `examined_ids` or `matched_ids` id is not in the set |
| `population_partial` | reject | under a plan declaring a `population_set` (FR-131), the candidate did not examine every id of the set |
| `rerun_until_pass` | reject | a regressed run with the candidate's own apparatus preceded it |
| `apparatus_edit` | reject | an earlier run under the same `definition_version` recorded a different protected apparatus than the candidate (FR-110) |
| `claimed_verdict_disagrees` | reject | the claimed verdict is not the checker's |
| `collection_deleted` | reject | a collection that measured this plan was added to the store's git history and no longer exists there (PLAT-985) |
| `collection_edited` | reject | a run's stored file was edited by a commit after the one that first added it (PLAT-985) |
| `definition_changed_without_version_bump` | reject | the plan's objective, estimator, decision rule, answer families or protected apparatus changed between two committed revisions sharing a `definition_version` (PLAT-985) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-108-AC-1 | A `MeasurementPlan`'s `statistical_design.estimator` and `.decision_rule` are read when present, as engineering-assurance's `Estimator` and `DecisionRule`. A value engineering-assurance refuses, a `constant-predictor` baseline under an estimator other than `proportion`, or a comparator that disagrees with the plan's `objective` refuses the plan load as `QM-PLAN-INVALID`, naming the member. | Test |
| FR-108-AC-2 | Every collection holding an observation of the plan's metric under the plan's id and `definition_version` is a run, and `collectionsConsidered` counts them whatever else the plan lacks. Runs are ordered by intake position, with unpositioned runs after every positioned one; the stated `timestamp` only breaks ties. The candidate is the last run. The candidate, or a `prior-collection` rule's prior, is `order_unattested` when it shares its intake position with another run or when a run before it states a later timestamp (or one after it an earlier one); a lone run needs no order. A slice an earlier run measured that the candidate lacks is `slice_missing`, and a collection of the candidate's subject and scope, not before it, that carries no observation of the plan is `observation_missing`: a dropped measurement never lets an older pass stand. | Test |
| FR-108-AC-3 | The rule is applied to every run against the runs before it; each run it does not hold for is listed in `regressedRuns` and counted. `prior-collection` is the latest earlier usable run's estimate and `best-seen` the maximum (`gt`, `ge`) or minimum (`lt`, `le`) of the earlier usable estimates, each computed from recomputed estimates; a run that is not usable evidence contributes none. From a run before the candidate only `value_disagrees_with_rows`, `population_malformed` and `interval_malformed` are findings; its other shortfalls exclude it from baselines and are otherwise history. `constant-predictor` is computed from the run being decided itself, not from history (PLAT-1016; see the section below), and is `inconclusive` with `constant_predictor_rows_absent` when that run carries no per-item observations for its metric (`constant_predictor_rows_malformed` and `constant_predictor_rows_mismatch` under FR-108-AC-10). `external-reference` is a per-dimension value the checker resolves from a source outside the plan at evaluation time; the checker has none to read, so this baseline is always `inconclusive` with `external_reference_unsupplied`, never computed from history or treated as `no_prior` (PLAT-1032). A regressed run whose tool and corpus revision equal the candidate's, followed by a candidate the rule holds for, is `rerun_until_pass`. | Test |
| FR-108-AC-4 | An observation's estimate is recomputed from its `population` — `matched / examined` for `proportion`, `matched` for `count` — and a `proportion` or `count` observation with no `matched` is `population_unstated`. The stored `value` is consistent when it is the recomputed `f64` or lies within half a unit of its last stated decimal (read from its shortest round-trip spelling) of the exact quotient; otherwise the run is `value_disagrees_with_rows`. The rule is applied to the recomputed estimate, never the stored value. A `proportion` whose unit is not `fraction` or `fraction of …` is `unit_unsupported`. `mean`, `median` and `ratio` take the stored value and count the observation as asserted. `observationsRecomputed` and `observationsAsserted` count every run's observations. An incomplete or unstated population, including an unstated `complete`, is `inconclusive`, never `accept`; under a `minimum_population` an `examined` below it, `0` included, is `population_below_minimum`, and with no minimum `examined: 0` is `population_empty`; a population short of `repetitions` or malformed is `reject`. | Test |
| FR-108-AC-5 | The verdict is `reject` when any finding's reason is a reject reason, otherwise `inconclusive` when any is an inconclusive reason, otherwise `accept`. A claimed verdict that differs adds `claimed_verdict_disagrees` and the verdict is `reject`. | Test |
| FR-108-AC-6 | The result is the `quoin.measurement-verdict.v1` document under Outputs, every member present, and each reason, verdict and order-source spelling round-trips. | Test |
| FR-108-AC-7 | `quoin measurement verify` exits 0 on `accept`, and 1 with the complete document on `reject` (`CORE_REJECTED`) or `inconclusive` (`CORE_INCONCLUSIVE`). An unknown plan id, and a collection filed under a name that is not its `collectionId`, are refused (exit 2); an unknown claimed verdict or order source is a bad request (exit 3); each with no verdict. A plan recorded through `quoin measurement record` and verified is accepted; the same store with one observation's stored value edited is rejected with `value_disagrees_with_rows`. | Test |
| FR-108-AC-8 | `quoin measurement verify` takes the intake order from `git log --first-parent --diff-filter=A` over the store and reports `orderSource: git-first-parent-add`. Outside a git work tree it reports `none` and says why on stderr; in a shallow clone it reports `git-shallow` and `order_unattested` and says so on stderr; any other git failure fails the command rather than yielding an empty order. | Test |
| FR-108-AC-9 | `quoin measurement verify` reads three tamper facts from the store's git history (PLAT-985): a collection this plan governed that was added and later removed (committed, or in the work tree) is `collection_deleted`; a collection's stored file changed after intake added it (a later commit, a delete and re-add under the same id, or an uncommitted edit) is `collection_edited`; each is attributed by the plan ids in its first-added and last-seen content; and the plan's own document changing its `objective`, `estimator`, `decision_rule` or `protected_apparatus` between two revisions sharing a `definition_version` — across a rename, and from the last commit to the work tree — is `definition_changed_without_version_bump`. Each is `false`/empty, not a false positive, when git cannot answer or the history is honest. A fourth fact is FR-110's: each earlier run whose protected files differ from the candidate's between their two `sourceRevision`s, as `git diff` reports, is `apparatus_edit`; no digest is recorded or compared. | Test |
| FR-108-AC-10 | A `constant-predictor` baseline is computed from the run being decided: every one of its own observations under `{metric}.constant-predictor-item` (`quoin_measurement::constant_predictor_item_metric`) carrying the same `planId`/`definitionVersion`, grouped by their `dimensions.family`. For each family, the best constant's hit count is the maximum, over every label recorded as some item's `dimensions.expected` or a member of its `dimensions.contested`, of the items whose `expected` equals that label or whose `contested` contains it. The baseline is the sum of each family's best-constant hits over the total item count across every family (MP-222/PLAT-932's size-weighted mean of per-family agreement — see the worked example in `engineering_assurance/skeletons/MeasurementPlan.md`). A run with no such observations is `constant_predictor_rows_absent`; one where any such observation lacks a non-empty string `item_id`, `family` or `expected`, states a `contested` that is not an array of strings, or repeats an `item_id` is `constant_predictor_rows_malformed`; one whose item count is not the governed observation's own `population.examined` is `constant_predictor_rows_mismatch` — no baseline is computed from a partial set. Intake admits an item observation under its governed metric's plan (same id, definition and lifecycle checks, no population minimum); a plan whose own `metric` ends in `.constant-predictor-item` is refused as `QM-PLAN-INVALID`; `compare` leaves item observations out. | Test |
| FR-108-AC-11 | When the plan's `decision_rule` declares `interval_level`, every run is judged by engineering-assurance's `DecisionRule::holds_on_interval` (engineering-assurance FR-021-AC-14) on that run's own `interval`, the candidate for its slice decision and every earlier run for regression detection under FR-108-AC-3; `DecisionRule::holds` and the point estimate are never used for such a rule. A run the rule does not hold for at its unfavourable bound is regressed exactly as a failing point estimate is, so run 1 failing at its lower bound, the same apparatus re-run, and run 2 passing is `rerun_until_pass`. An earlier run for which `holds_on_interval` gives no answer (no interval, a level below the plan's, or an estimate outside its interval) is regressed, the conservative direction, so an absent or weakened earlier interval cannot let a rerun-until-pass through; it adds no finding of its own except `interval_malformed`. A baseline rule's baseline value remains the earlier runs' point estimates. `method` is never read. Each slice decided on an interval carries in `decisions`, additively, `intervalBound` (`lower` or `upper`), `intervalBoundValue` and `intervalLevel`; other slices carry none, and the verdict document for a plan with no `interval_level` is byte-identical to the one before this criterion. | Test |
| FR-108-AC-12 | Under a plan declaring `interval_level`, a candidate with no `interval` is `inconclusive` with `interval_unstated`; one whose `interval` does not satisfy FR-044-AC-10 is `reject` with `interval_malformed`, in any run; one whose `level` is below the plan's is `reject` with `interval_level_short`; the rule is never judged on the point estimate instead. An estimate outside its interval (engineering-assurance's `EstimateOutsideInterval`) is `inconclusive` with `rule_not_evaluable`; it is reached when a recomputed `proportion` or `count` estimate falls outside the stored interval, which FR-108-AC-4's rounding tolerance permits (stored `value` 0.67 inside an interval with `lower` 0.67, recomputed 2/3). The interval requirement does not depend on resolving the baseline: a first candidate under a baseline rule with no interval lists both `interval_unstated` and `no_prior`. A plan whose rule states `interval_level` with an `eq` comparator, at 0 or 1 or outside them, non-numeric or null, refuses the plan load as `QM-PLAN-INVALID` under FR-108-AC-1, naming the member. | Test |
| FR-108-AC-14 | A plan whose `decision_rule` states `margin_mode: relative` (engineering-assurance FR-021-AC-12) is decided with the relative reference in verify, so a relative margin changes the verdict a same-value absolute margin would give; a plan stating `absolute` or no `margin_mode` decides with the margin applied as an absolute offset. | Test |
| FR-108-AC-15 | `quoin_jev::graded_corpus_collection` takes a `GradedCorpus` (plan id, definition version, governed metric, collection id, subject, scope, tool identity and version, timestamp, environment, `verificationStack` capabilities and artifacts, `repetitions`, and the graded items as `ConstantPredictorItem`) and returns a candidate `MeasurementCollection` document with every member the intake contract requires filled from the input: `schemaVersion`, `collectionId`, `subject`, `scope`, `toolIdentity`, `toolVersion`, `timestamp`, `environment`, `verificationStack` exactly as the caller states it, `buildProfile` and every other member included, never defaulted or invented, non-empty `observations` and `rawEvidence` (an empty object: FR-108-CON-3, nothing is duplicated into it). It performs no I/O and reads no clock, environment or random source. | Test |
| FR-108-AC-16 | The collection's first observation is the governed aggregate: metric `metric`, state `measured`, shape `ratio`, unit `fraction`, no `dimensions`, and a `population` of `examined` = the item count, `matched` = the items whose `actual` equals their `expected` or is a member of their `contested`, `complete` = `true` and the input's `repetitions`; its `value` is `matched / examined`. | Test |
| FR-108-AC-17 | Every other observation is one item row, exactly the rows `item_observations` (FR-108-AC-10) builds for the same items and metric, in `item_id` order (FR-108-AC-19), so the item rows number exactly the aggregate's `population.examined` and `quoin measurement verify` on a plan with `baseline: constant-predictor` decides against a baseline recomputed from the stored rows after the collection passes intake, and never returns `constant_predictor_rows_absent`, `_malformed` or `_mismatch` for it. The families in the item rows are exactly the `family` values the caller's items carry; the function takes no family list and invents none. | Test |
| FR-108-AC-18 | `graded_corpus_collection` refuses with `QM-COLLECTION-INVALID` (`MeasurementErrorCode::CollectionInvalid`), naming the field, when `plan_id`, `definition_version`, `metric`, `collection_id`, `subject`, `tool_identity`, `tool_version` or `timestamp` is empty; when `verificationStack` capabilities or artifacts are empty; when `repetitions` is `0`; when there are no items; and for every refusal `item_observations` states (an empty `item_id`, `family` or `expected`, or a repeated `item_id`). It returns no partial collection. | Test |
| FR-108-AC-19 | The result is deterministic: the same input yields the same document and the same canonical JSON bytes on repeated calls, and permuting the order of the input items yields the same document, because item rows follow the aggregate ordered by `item_id` ascending (bytewise). | Test |
| FR-108-AC-20 | The item's `actual` is the answer class the caller grades on (for the criterion-strength harness the `Graded` row's `actual_class`, the rounded level, not the raw `actual` score). An item the graded tool did not agree with, including an unanswered or unrecognized row (whatever non-empty class the caller maps it to) or one whose `actual` is empty, is counted in `examined` and not in `matched`; the function cannot drop an item. A transport failure that aborts a pass is the caller's, and never produces a collection (MP-228's scoring rule). | Test |
| FR-108-AC-21 | The criterion-strength live harness (`tests/live_criterion_strength.rs`, the `live-api` feature, ignored by default) builds its items with the offline mapping of FR-108-AC-23, builds the collection with `graded_corpus_collection` and, when the environment names a measurement repository, writes it through `write_measurement_collection`; with none named it writes nothing. The write comes before every assertion of the test, including the unanswered and unrecognized checks and the bars, so a failing pass still leaves its evidence stored. The write needs intake's preconditions: the named repository holds the governing plan under `spec/assurance`; that plan's `protected_apparatus` entries resolve in the repository and are all listed in the collection's `verificationStack.artifacts` (FR-110); and the plan is `active` with the `definition_version` and `repetitions` the collection states. End to end, the stored collection is decided by `quoin measurement verify` with a recomputed constant-predictor baseline only against a plan whose `statistical_design` declares `baseline: constant-predictor` with an `estimator: proportion` and a `decision_rule` (FR-108-AC-1). The harness stores the pass under MP-244, a single-pass plan that declares exactly that and the `weakness_kind` and `coverage` answer families. MP-228 is not the target: its typed design is the proportion of clearing passes (`ge 0.6` over five repetitions), a different population from the items of one pass, so storing an agreement rate under it would be decided by the wrong rule. | Test |
| FR-108-AC-22 | `graded_corpus_collection` states no `buildProfile`: it takes `verificationStack` from the caller and never invents it. Intake admits any stated `verificationStack.buildProfile` (FR-044-AC-14), so the harness states the profile it actually ran under and a debug build never states `release` falsely; a harness that skips the live write for a debug build says so on stderr with the reason. | Test |
| FR-108-AC-23 | The mapping from the harness's graded rows to `ConstantPredictorItem` (`item_id` the fixture id, `family` the answer space the fixture is graded in, `expected`, `contested`, `actual` the answer class) is a function separate from the live call, tested offline on the default path over the committed fixture corpus: it yields 15 items in two families, `weakness_kind` 11 and `coverage` 4 (FR-108's worked example), and `graded_corpus_collection` over them passes `quoin_measurement::validate` against a plan fixture. The live write itself is the only part gated by the environment and the `live-api` feature. | Test |
| FR-108-AC-24 | A `constant-predictor` baseline holds the run's item rows' distinct `family` values against the plan's `statistical_design.answer_families`, compared as a set (engineering-assurance FR-021), before computing anything: a plan stating none is `inconclusive` with `constant_predictor_families_unstated`; an item row whose `family` is not declared (two declared families merged under a new name, or one split) is `constant_predictor_family_unexpected`; a declared family with no item row (including two declared families merged under one of the declared names) is `constant_predictor_family_missing`. No baseline is computed in any of these cases, the same families in any order are accepted, and an undeclared family is reported before a missing one. | Test |
| FR-108-AC-25 | A plan whose `statistical_design.decision_rule.baseline` is `constant-predictor` and which states no `statistical_design.answer_families` is refused at load as `QM-PLAN-INVALID` naming `statistical_design.answer_families`; a plan that states the list loads. Only the current plan is held to this: an earlier revision without the list still parses when read as history, so the definition-change tamper findings between such revisions are kept. | Test |

> **CR note (2026-09-29, agent-ix/quoin#654):** FR-108-AC-13 is withdrawn. It
> required the engineering-assurance crate `version` to move to a stated value
> in several files together, which is a version bump, not behaviour. The
> capability it stood for needs no criterion of its own: `quoin-measurement`
> calls `DecisionRule::holds_on_interval` in its own code, so a build resolving
> a revision without it does not compile, and FR-108-AC-11 covers
> what it decides. TC-1959 is withdrawn with it. The id is not reused.

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

## Constant-predictor baseline: producer shape and computation (PLAT-1016)

MP-222/PLAT-932's constant-predictor baseline — the size-weighted mean of the
best-constant agreement per answer family, computed from the corpus rather
than hard-coded — needs a producer to retain each graded item's own answer,
grouped by family. PLAT-985 investigated this and found no producer did;
`quoin-jev`'s `tests/support/grading.rs::trivial_baseline()` already
implemented the formula, but only as an in-process test assertion over one
crate's own fixture corpus, writing no `MeasurementCollection`.

**Producer output shape.** A producer that grades a corpus against recorded
labels writes one retained observation per graded item, in the same
`MeasurementCollection` as the metric's own aggregate observation, under the
metric name `{metric}.constant-predictor-item`
(`quoin_measurement::constant_predictor_item_metric`) — never under `metric`
itself, so an item observation is never read as another slice of the governed
measurement (the checker's run-building filter matches a plan's `metric`
exactly). Each item observation's `dimensions` (`quoin_measurement::
constant_predictor_dims`) carries:

| dimension | type | meaning |
| --- | --- | --- |
| `item_id` | string | the item's identity, unique among this metric's item observations in the collection |
| `family` | string | the answer-space family this item belongs to — the plan's own Population grouping, not invented by the checker or the producer |
| `expected` | string | the primary recorded (ground-truth) label |
| `contested` | string[] | every reading the corpus recorded as defensible, `expected` included |
| `actual` | string | what the graded tool returned, for traceability; not read by the baseline formula |

`quoin-jev`'s `constant_predictor::item_observations` (PLAT-1016) is the
reference producer: given a plan id, definition version, governed metric, and
a slice of graded items, it builds exactly this shape. It performs no I/O —
merging the observations into a collection and writing it is a caller's
concern, matching every other producer path in this crate.

**Why `dimensions`, not `rawEvidence` or a new population member.**
FR-108-CON-3 already states `rawEvidence` is not read — no schema says what
its members mean — and giving this one baseline a private schema inside it
would mean narrowing that constraint for a single feature. `population`
(`examined`/`matched`/`complete`/`identity`) describes what was examined in
aggregate; there is nothing aggregate about one item's own label. `dimensions`
already exists to distinguish one slice of a metric from another, and an
item's own row is exactly one more slice — of a metric the plan does not
itself read.

**Intake and comparison.** Intake admits an item observation under its
governed metric's own plan — the same active-plan, plan-id and
definition-version checks as any observation — but never holds one item to
the plan's `minimum_population` or `repetitions`, which bind the aggregate.
The suffix is reserved: a plan whose own `metric` ends in
`.constant-predictor-item` is refused at load, because it would read another
plan's item rows as its own run. `compare` leaves item observations out; they
are one graded item's labels, not a slice of a measurement.

**Checker computation.** `quoin-measurement::verify`'s `Baseline::
ConstantPredictor` arm (FR-108-AC-10) reads the run being decided's own item
observations for the governed metric, groups them by `family`, and computes
each family's best-constant hit count: the maximum, over every label any item
in the family recorded as `expected` or a member of `contested`, of the count
of items agreeing with that label (by the same `expected`-or-`contested`
rule). The baseline is the sum of every family's best-constant hits over the
total item count across every family.

It fails closed rather than computing a baseline from a sample it cannot
vouch for. No item observations is `constant_predictor_rows_absent`. Any
malformed one — no non-empty string `item_id`, `family` or `expected`, a
`contested` that is not an array of strings, or a repeated `item_id` — is
`constant_predictor_rows_malformed`: dropping it would shrink the sample the
baseline is computed from, silently, and in whichever direction the dropped
items happened to lean. An item count other than the governed observation's
own `population.examined` is `constant_predictor_rows_mismatch`: the item rows
are not the population the rate was measured over. That link is what makes
the plan's `minimum_population` bind the baseline too — the floor already
holds on `examined`, and the item rows must number exactly `examined`. It also
means a sliced governed metric (observations with `dimensions`) is
`constant_predictor_rows_mismatch` for each slice whose `examined` is smaller
than the population-wide item rows; per-slice constant-predictor baselines are
not supported.

## Known limits

- **Prior-collection laundering.** `prior-collection` compares the candidate
  with the latest earlier usable run and nothing else. A producer can lower
  that bar by recording a deliberately poor run under a *different* apparatus
  (a different tool or corpus is enough) just before the candidate: it is not a
  rerun of the candidate's apparatus, so `rerun_until_pass` does not fire,
  and it becomes the prior. The regression is still listed in
  `regressedRuns` and counted, so it is visible; it is not refused. A plan
  that must not be gamed this way uses `best-seen` or a threshold.
- **A rounded aggregate is judged at its own precision.** A stored `1` states
  no decimals, so it is consistent with any quotient from 0.5 to 1.5. This is
  harmless to the verdict — the rule sees the recomputed estimate — but it
  means `value_disagrees_with_rows` catches only a stored value that is wrong
  at the precision it claims.
- **A constant-predictor baseline trusts the producer's own `family`
  grouping.** The checker groups by whatever string a `family` dimension
  states; MP-222's rule that grouping "not be invented here" binds the
  producer, and the plan states its families only in prose, so the checker
  has nothing to hold the grouping against. The dangerous direction is
  *collapsing* families: grouping two answer spaces into one understates the
  baseline the same way a single global constant did in MP-222's worked
  example (9/15 instead of 12/15), and lets a tool clear a bar it never beat.
  Splitting too finely only overstates it (a one-item family always scores
  1.0), which makes the bar harder, not easier. No structural heuristic
  separates a collapsed family from an honest one: in the worked example's
  own `weakness_kind` family, `weakness` never co-occurs with `sound` in any
  item's `contested`, so "disjoint label clusters" would flag the correct
  grouping too. The plan therefore declares its families in
  `statistical_design.answer_families` (engineering-assurance FR-021) and the
  checker requires the item rows' family set to equal it
  (`constant_predictor_family_unexpected`, `constant_predictor_family_missing`;
  FR-108-AC-24). The list is the plan author's statement: the checker cannot
  tell whether a declared family is itself a sound answer space.
- **The tamper facts attest the history this clone presents.** A history
  rewritten and force-pushed so the tampering commits never existed leaves
  nothing for any reader of it to find; protection of the store's branch
  against force-push is what closes that, not this checker. Likewise a plan
  document replaced under a different path with too little similarity for
  git to follow as a rename starts a new history.

## Graded-corpus collection producer (PLAT-1021)

PLAT-1016 built the per-item rows but no production path called
`item_observations`: `quoin-jev` is a leaf library with no binary, and
`quoin measurement record` takes a finished collection, so no stored
collection carried item rows and every real `constant-predictor` plan stayed
`constant_predictor_rows_absent`. The owner chose to close this inside the
library: a pure function builds the whole collection, and the live Jev
harness, the only code that grades a real corpus, writes it. No new binary,
subcommand or crate dependency is added (`quoin-jev` already depends on
`quoin-measurement`; neither `quoin-cli` nor `quoin-core` gains `quoin-jev`).

**Function.** `quoin_jev::graded_corpus_collection(corpus: &GradedCorpus) ->
Result<JsonValue, MeasurementError>` (module `graded_corpus`, re-exported
from the crate root). The result is the candidate document
`write_measurement_collection` and `quoin measurement record` accept. The
caller supplies identity and time (`collection_id`, `timestamp`,
`environment`, `scope`, `verificationStack`), so the function reads no clock
or environment and stays deterministic. Intake still owns its own checks and
merges (active plan, definition version, minimum population, repetitions,
protected apparatus, `unverifiedArtifacts`); the function does not restate
them.

**Fields filled.** The collection's own members come from the input
(FR-108-AC-15). Its observations are the governed aggregate (FR-108-AC-16)
followed by the item rows (FR-108-AC-17). The aggregate and the item rows are
computed from one pass over the same items with one agreement rule, so the
item count equals `population.examined` by construction, which is what
`constant_predictor_rows_mismatch` checks. `item_observations` stays the one
builder of item rows; the collection function calls it.

**Where the harness calls it.** After the graded pass of
`the_lens_beats_the_do_nothing_baseline` in `live_criterion_strength.rs`,
before the bars are asserted, so a failing bar still leaves the evidence of
the run. The target repository comes from an environment variable read by the
harness (not by the library); unset means no write. The plan id, metric and
definition version are the governing plan's (MP-228), and a mismatch is
refused by intake, not guarded in the harness. Other live harnesses (EARS,
gap analysis) are not changed here; each would map its own graded rows the
same way under its own plan.

**Forward note for PLAT-1020 (data shape only).** PLAT-1020 intends a
plan-declared `statistical_design.answer_families` list that the checker
compares with the producer's item-row families. The shape this producer
guarantees for that comparison is: the set of distinct `dimensions.family`
values over a collection's item rows is exactly the set of families of the
items the caller passed, with no family added, defaulted or merged by the
function, and every item row has a non-empty `family`. The function neither
reads the plan nor takes a families list. Whether the checker compares sets or
sizes, and the reason codes for a difference, are PLAT-1020's to specify.

## Constraints

- **FR-108-CON-1**: The checker performs no I/O and reads no clock; the
  command's I/O is the store read and the `git` calls for the intake order
  and the tamper facts (PLAT-985).
- **FR-108-CON-2**: The decision rule's comparator and margin semantics are
  engineering-assurance's; the checker calls `DecisionRule::holds`, or
  `DecisionRule::holds_on_interval` when the rule states `interval_level`, and
  states no comparison of its own.
- **FR-108-CON-3**: `rawEvidence` is not read: no schema says what its
  members mean.

## Dependencies

- FR-044 defines the plan-governed store the checker reads.
- engineering-assurance FR-020 and FR-021 own the `objective`, `estimator`
  and `decision_rule` types and the rule's evaluation, and FR-021's
  `definition_change_without_version_bump` is what
  `definition_changed_without_version_bump` (PLAT-985) applies over the
  plan's git history.
- FR-110 owns `protected_apparatus` resolution at intake and the git
  comparison of recorded protected sets. `apparatus_forged` and
  `apparatus_unverifiable` are withdrawn: nothing is recorded over file
  content, and no revision is compared.
- `quoin-jev`'s `constant_predictor` module (PLAT-1016) is the reference
  producer for `constant-predictor` baseline item observations; it depends on
  this crate's `MeasurementObservation` type and the `constant_predictor_item_metric`/
  `constant_predictor_dims` naming convention rather than restating them.
- `quoin-jev`'s `graded_corpus` module (PLAT-1021) is the reference producer
  of the whole graded-corpus collection, and calls `item_observations`.
