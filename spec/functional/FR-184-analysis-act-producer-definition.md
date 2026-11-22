---
id: FR-184
title: "FR-116 producer definition for the analysis act"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-116"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-152"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-151"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-033"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-093"
    type: "references"
  - target: "ix://agent-ix/filament-analyze/FR-009"
    type: "references"
  - target: "ix://agent-ix/filament-analyze/FR-005"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-078"
    type: "references"
---

# FR-184: FR-116 producer definition for the analysis act

## Description

[FR-116](./FR-116-owner-qualified-quire-evidence.md) requires a Quoin-owner-approved producer definition before an
act can qualify, and [FR-153](./FR-153-analysis-result-counting-by-disposition.md) names the content of the
definition for the analysis act as a decision it does not take. This requirement is that definition: what runs, what
the host supplies, what a positive decision means and what it does not mean. The FR-116 owner approved it; the
approval is recorded in the Owner approval section.

The definition has the identity `analysis-act` and the version `2`. It
cites the owners of the report, the runner and the executor and does not restate them.

### What runs

The act is one invocation of `filament-analyze run-package` ([filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009))
with `--package`, `--z3`, `--cvc5`, `--producer-execute` and `--output`, the arguments and their absolute-path rules
as that requirement states them. The runner builds one consistency request over all analysable clauses of the
package. It starts the Z3 and cvc5 solvers only through `producer-execute`, which is the
`engineering-assurance producer-execute` command ([engineering-assurance FR-019 and FR-029](ix://agent-ix/engineering-assurance/FR-029)),
one invocation for each engine's version and one for its query. No other way of starting a solver is part of this
act, and a report from a solver started any other way is not an observation of it.

## Inputs

| Input | Source and meaning |
|---|---|
| Contract package | The `filament.contract-package/v1` document given as `--package` (filament-analyze FR-009 / `ContractPackageWireV1`). It names no analysis request; the runner fixes the request. |
| Solver executables | `--z3` and `--cvc5`, absolute paths with no symbolic-link component (filament-analyze FR-009). |
| Executor | `--producer-execute`, the absolute path of the `engineering-assurance` executable, never searched for on `PATH`. |
| Declared method | The selected obligation's declared method, which must be Analysis (FR-116 method/act pairing; FR-152; FR-153). |
| Obligation statement | The `--discharges` id the recorder gave to `quoin evidence record` (FR-152). It is the only binding source. |
| Run purpose | The host's explicit run kind, `analysis` for an ordinary analysis report or `formal_rule` for a formal-rule judgement. Existing ordinary projections without this member are treated as `analysis`. |
| Formal-rule claim mapping | For `run_kind: formal_rule`, the host supplies the mapping retained as `formal_rule_selection` by FR-151. It maps every selected claim id and source span to its QSL symbol and the `approved` or `draft` status returned by FR-188 when the run was prepared. |

## Outputs

- One `filament.differential-report/v2` document and the runner's exit status, whose classes are those of
  [filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009). The report is read by
  the `differential-report` adapter, which accepts it only when `filament_analyze::validate_report_document` does
  (FR-151).
- When recorded, the source report is validated and reduced to the FR-151
  allowlisted projection in the FR-030
  `analysis-reports/<suite>/<timestamp>.json` companion file. It is linked to
  exactly one run by the shared canonical suite/timestamp key and is covered
  together with that run by the host's signature-verification of the commit
  that adds the pair. The projection adds no member to the source report schema,
  `RunRecord`, `RunEntry` or `--run-observation` object.
- The source report is validated against the closed `filament.differential-report/v2` schema. The FR-151 projection
  retains only `producer.name`, `producer.version`, each engine's `configuredExecutable`, observed `identity.path`
  and `identity.version` when present, each engine's `status` and `outcome`, the `disposition`, and the `agreedStatus`,
  plus the run kind and the formal-rule claim mapping when `run_kind` is `formal_rule`, as FR-151 defines. The projection adds no member to the source report,
  `RunRecord` or `RunEntry`. The exact suite/timestamp key
  pairs it with one run; qualification requires the run cardinality and outcome mapped from the disposition and
  agreed status (one entry for agreement, inconclusive or unavailable; zero entries for disagreement). It does not
  retain query, model, stdout, stderr or raw solver output. These projected values are the transcribed observations
  the performed-act premise reads.
- The report supplies no CI system, run, attempt or job, no actor, no performed-at time and no source revision. The
  adapter's entry carries no `evidenceKind` and no `traceIds` (FR-151). Those premises come from the host (FR-116) or
  are open points below.

### Execution association

The act SHALL run only in the Quoin repository's GitHub Actions workflow through the `workflow_dispatch` event. This
is a manually started CI workflow execution, not a local/manual act; no `push`, `pull_request`, `schedule`, or other
automatic trigger is admitted by this definition. The required invocation is exactly one `filament-analyze
run-package` step in that run. The host SHALL authenticate the association against GitHub's API using credentials
available only to the workflow host and SHALL derive, rather than accept as caller-authored report fields, the
repository owner/name, workflow identity, event name, run id, run attempt, job id, invocation identity, actor, and
checkout SHA. The authenticated workflow run and job SHALL agree on repository, workflow, event, run id, attempt,
job, and checkout SHA; the checkout SHA SHALL be the commit actually checked out for the invocation. A missing,
failed, ambiguous, or mismatched lookup leaves the association or performed revision unevaluated. A supplied run URL,
environment variable, report field, or green status by itself is not authenticated association evidence.

The workflow SHALL publish the exact report bytes produced by that invocation as an artifact of the authenticated
run. The host SHALL retrieve that artifact through the authenticated run, verify the artifact's service-provided
digest, and compute and retain the SHA-256 of the exact `filament.differential-report/v2` bytes selected from it. The
report SHALL validate under FR-151; the artifact SHALL contain exactly one such report for this invocation. The
host's validation-evidence reference SHALL bind the authenticated run/attempt/job/invocation tuple, checkout SHA,
artifact identity and digest, report byte digest, and the exact report projection supplied to qualification. No
association or output identity is inferred from a filename. This definition adds no member to the closed report
schema or the FR-151 projection. A workflow change is required to implement this definition and is outside this
specification change.

### Source revision

The qualifier SHALL take the performed revision FR-116 compares with the selected HEAD from the CI checkout SHA of
the `run-package` step, supplied by the host. The report carries none. Without one, the result is `unevaluated`,
naming the performed-revision premise.

### Formal-rule claim qualification

When a formal-rule judgement is qualified, the host SHALL supply expected
`run_kind: formal_rule`, and the FR-151 projection SHALL carry that same
discriminator. The host SHALL provide the `formal_rule_selection` mapping from
the selection used for `run-package` and the FR-188 status report used to build
it. The mapping SHALL contain exactly one entry for each id in the report's
`selection.selected` array. Each entry SHALL name that claim id, the matching
report source span, its QSL symbol, and the status reported for that symbol
when the run was prepared. The qualifier SHALL confirm that the mapping's ids
equal `selection.selected` exactly, that every source span matches the report,
and that each selected symbol has state `approved`. If expected `run_kind` is
`formal_rule` and the projection's discriminator is missing or different, the
source report has no `selection`, the mapping is missing or malformed, the ids
or spans differ, or any selected rule has state `draft`, the qualifier SHALL
return `qualification-rejected` with reason `draft_rule_in_judgement`. It SHALL
fail closed and SHALL NOT infer a missing mapping or status from the current
checkout. Ordinary analysis qualification does not require a
`formal_rule_selection` mapping. This mapping records no digest, checkout
revision, version pin, or additional repository file path.

### Tool and configuration

The supported execution environment is the GitHub-hosted `ubuntu-24.04` x86_64 runner, with Rust toolchain `1.98.1`,
the workflow's checked-out Quoin commit, and a `filament-analyze` executable built from the
`agent-ix/filament-analyze` source revision declared by the workflow. The executable SHALL run `run-package` with
the exact FR-009 contract-package input, absolute
`--z3`, `--cvc5`, `--producer-execute`, and `--output` paths, and the bounded reader limits and refusal behavior of
filament-analyze FR-009. The contract-package input SHALL contain exactly one selected `analysis_claim`; its
canonical bytes, selected claim id, and source identity SHALL be retained as validation evidence and SHALL match the
report's package, assertion, and clause identities. The report SHALL contain exactly that one selected assertion.

The `producer-execute` executable SHALL be the Engineering Assurance `v0.10.1` distribution and SHALL implement the
FR-019/FR-029 producer-execution contract used by FR-009; the workflow SHALL record the resolved executable version
and distribution identity. Engineering Assurance 0.8.1 is unsupported because it does not provide the required
`signature-verify` command. Z3 and cvc5 SHALL be real executables on this runner, and their observed
`identity.path`/`identity.version` values and configured paths SHALL be retained by the report and compared with the
accepted FR-093 context. This definition adds no solver-version allowlist: a missing identity or any observed-context
mismatch prevents reliance rather than being treated as a match. The workflow SHALL record the exact invocation
arguments and non-secret effective configuration needed to reproduce the run; secrets SHALL NOT be copied into the
report, artifact metadata, or FR-093 decision.

### Validation corpus and positive reliance boundary

The validation corpus SHALL be a versioned, owner-approved `filament-analyze` TC-006 artifact at an exact source
revision. The owning ticket [VER-17](https://linear.app/agent-ix/issue/VER-17/analyze24-qa-retain-the-complete-seeded-real-engine-corpus-and-direct)
tracks the missing retained corpus. The artifact must enumerate exact case identities and input bytes, expected
classifications, real-engine versus controlled-failure cases, tool/configuration identities, and the reports or
non-report outcomes that each case is expected to produce. A category list or test name without those identities is
not a validation corpus. The validation-evidence reference SHALL identify the approved corpus artifact, source
revision, each case identity and expected outcome, and the corresponding observed outcome. Partial or stub-only
evidence does not satisfy this definition.

The two cases in `official_z3_cvc5_differential_corpus_agrees` (`real-sat` and `real-unsat`) are only the current
real-engine SAT/UNSAT controls. They SHALL NOT be described as the complete TC-006 corpus or as a `run-package`
end-to-end result. The current filament-analyze test matrix states that the complete retained corpus (#24) remains
open, and VER-17 does not yet provide exact case identities. Therefore the exact `validationCorpus` identity remains
unresolved: no FR-093 assessment based on this definition can be accepted and no analysis obligation can qualify
until the owning team defines and retains it. This requirement does not claim that the corpus, workflow, or any live
run exists, and does not create a live run.

### Actor authority

The qualifier SHALL take the asserting actor to be the signer of the record, mapped to a person through the
signed-records feature ([PLAT-1433](https://linear.app/agent-ix/issue/PLAT-1433)). `--discharges` together with that authority is sufficient
to bind the obligation only when the complete signature result targets commit C's exact object identity, C contains
the exact selected run and companion projection bytes, and both canonical paths were absent from C's first-parent
tree (or the empty tree for a root commit) before C adds them together. A signature result for a later commit whose
tree merely retains the pair, for a commit adding only one member, for another commit with matching tree contents,
or for mismatching bytes does not admit actor authority. No commit id is persisted in the store.

### What a positive qualifier decision means

The bridge SHALL NOT emit a record claim for an obligation unless **all** of these hold, and each is a premise the
qualifier evaluates and retains (FR-116):

1. the report's `disposition` is `agreement` and `agreedStatus` is `satisfied`, so both engines, Z3 and cvc5, are
   conclusive with the same status (FR-151: entry outcome `pass`);
2. the entry is bound to the obligation by `--discharges` and the binding survives the FR-032 audit (FR-152);
3. the obligation's declared method is Analysis;
4. the entry is in the latest run of its suite and no later run of that suite fails, errors or lacks its symbol
   (FR-153; the run is not superseded);
5. every other FR-116 premise holds: bounded reliance (FR-093), actor authority, performed revision against the
   selected HEAD, tool and configuration, execution association, and freshness. Freshness is revision and
   supersession only; no age rule applies;
6. the report is an observation of a solver started through `producer-execute` (see Producer-execute evidence below).

Freshness SHALL be decided by the performed revision and supersession only; the qualifier SHALL apply no age rule.

The decision is use-specific (FR-116): it says that this act, run under this definition, completed with both solvers
agreeing the analysed clauses are consistent, for this exact target and method. A report whose `--discharges` states more than
one obligation qualifies none of them: each stays `unevaluated`, and only a report stating a single obligation can
qualify (the reading of owner answer 8, flagged in Open points). The credit is **consistency evidence only**: it says the analysed clauses are consistent and
nothing about the obligation's meaning. The requirement-to-formalization link
([Linear VER-75](https://linear.app/agent-ix/issue/VER-75)) will tighten it. A decision **does not assert**:

- that the analysis request is relevant to the obligation's statement, that it is the right analysis, or that it
  proves the obligation. The `--discharges` id is the recorder's assertion. Quoin does not judge it (FR-153); the
  actor-authority premise of FR-116 evaluates only whether the asserting actor may make it;
- that a solver is sound, or that any property beyond the one consistency request the runner built holds;
- the outcome of any other obligation bound to the same entry, or of a sibling criterion (FR-116 per-target rule);
- anything about a `disagreement`, which is stored as a zero-entry run and counts as `absent` (FR-153).

### Producer-execute evidence

A report is an observation of the act only when its solvers were started through `producer-execute`
(filament-analyze FR-009 starts them no other way). The qualifier SHALL read only the FR-151 projection's fields to decide
this, as FR-184-CON-2 requires: `producer.name` and `producer.version` of the runner, and for each of the Z3 and cvc5
engines a `configuredExecutable` that is an absolute path, an observed `identity.path` and `identity.version` (the
version invocation that `producer-execute` ran first, FR-009), and a `status` and `outcome` from the closed set
(FR-005-AC-6; FR-009 result-state table). If any required value is absent for either engine, the projection does not
establish an observation of the act; the result is `unevaluated` naming the tool and configuration premise, and no
positive decision is made.

A `refuted` agreement is `failed`; an `inconclusive` or `unavailable` report is `errored`; neither can qualify.

### Default `unevaluated`

The definition is approved, and two premises keep the result `unevaluated`, each named in the result:

- **Premise: no FR-093 assessment.** Bounded reliance on `filament-analyze` and the solvers for this positive use
  ([FR-093](./FR-093-evidence-producer-trust.md)) is open: no assessment exists. Every analysis-backed obligation
  SHALL stay `unevaluated` and uncounted until an assessment is written.
- **Premise: no authenticated workflow association.** A result without the authenticated workflow association and
  report-artifact identity specified above SHALL be `unevaluated`.
- **Premise: unresolved validation corpus.** A result whose validation evidence does not identify the owner-approved
  TC-006 corpus artifact and its exact cases SHALL remain `unevaluated` naming bounded reliance.

### Open points

- **Report stating several obligations.** Rule stated by the lead from answer 8, owner may amend: a report whose
  `--discharges` states more than one obligation qualifies none of them.
- **Producer-execute marker.** The report carries no field that names `producer-execute` directly; the qualifier
  reads the engine fields listed above. A dedicated marker would be a filament-analyze change and is not required here.

## Owner approval

Owner approval for version 1 was recorded 2026-11-13 via the planner relay, confirmed by the owner in session. The
FR-116 owner (Peter) approved that definition with these answers:

1. **Source revision.** The CI checkout SHA of the `run-package` step; without one the result is `unevaluated`.
2. **CI only.** No local/manual acts. The workflow step needs the owner's clearance to edit, so this requirement
   originally named the step without editing a workflow file.
3. **Package to obligation.** The recorder's `--discharges` plus actor authority is sufficient; FR-152 is unchanged.
   The credit is consistency evidence only, and the requirement-to-formalization link (Linear VER-75) will tighten it.
4. **Tool and configuration.** The version 1 decision named no solver version set; solver identities are observed. Version 2 names Engineering Assurance `v0.10.1` as the supported distribution because the qualification path requires its `signature-verify` command.
5. **Actor authority.** The signer mapped to a person through the signed-records feature (PLAT-1433).
6. **Bounded reliance (FR-093).** OPEN. No assessment exists. It is a premise that keeps every analysis-backed
   obligation `unevaluated` until an assessment is written.
7. **Freshness.** Nothing beyond revision and supersession.
8. **Scope of one act.** One report qualifies only the one obligation stated.
9. **Waiver.** Ruled (FR-116 owner, relayed by the planner 2026-11-13); the quire-rs side is pending. Option A: a
   waiver is a named person's decision with an approval reference, signed through the signed-records feature
   ([PLAT-1433](https://linear.app/agent-ix/issue/PLAT-1433)), and reported as its own waiver outcome and never as a
   pass. Strict mode still fails by default; a policy setting must opt in before approved waivers pass. It reuses the
   person-owned disposition pattern of the advisory join (PLAT-1516) and the anomaly release rationale (PLAT-1584).
   The quire-rs FR-078 side still needs its own spec change from the IR and quire-rs owners; until it lands nothing is
   waivable, and this requirement edits no quire-rs requirement.

On 2026-11-22, the owner accepted recording bounded FR-093 reliance and authorized a `workflow_dispatch`-only
workflow job, with no live run during this effort. That acceptance does not assert a completed assessment or real
validation evidence. Version 2 specifies the environment and evidence needed for a future assessment; no actual run
or completed TC-006 corpus is claimed here. Until the corpus and accepted FR-093 assessment exist, nothing counts as
covered.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-184-CON-1 | The definition SHALL add no field to the closed v2 report schema, `RunRecord` or `RunEntry` (FR-116-CON-2). | Compatibility | Inspection |
| FR-184-CON-2 | Qualification SHALL run no solver and read only supplied observations (FR-116-CON-1; FR-153-CON-2). | Architecture | Inspection |
| FR-184-CON-3 | The store SHALL retain only the FR-151 allowlisted projection, never the source report or its query/model/solver output, and never copy it into the run record; an older run with no companion remains unevaluated and is not migrated. | Compatibility | Inspection |
| FR-184-CON-4 | The workflow evidence SHALL add no field to the closed `filament.differential-report/v2` schema, `RunRecord`, `RunEntry` or FR-151 projection. Authenticated workflow, artifact and output identities are supplied to the host's FR-116/FR-093 observation boundary, not copied into those closed records (FR-116-CON-2). | Architecture | Inspection |

## Acceptance Criteria

The criteria describe the approved definition; none claims an implemented qualifier.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-184-AC-1 | With no FR-093 assessment of `filament-analyze` and the solvers, every analysis-backed obligation is `unevaluated` naming the bounded-reliance premise and none is counted. | Test |
| FR-184-AC-2 | A `satisfied` agreement bound by `--discharges` to an Analysis obligation, latest in its suite, with an authenticated `workflow_dispatch` association and checkout SHA and every other FR-116 premise admitted, yields a positive decision for that target only. | Test |
| FR-184-AC-3 | A `refuted` agreement report bound to an Analysis obligation yields `failed` and no positive decision. | Test |
| FR-184-AC-4 | An `inconclusive` report yields `errored` and no positive decision. | Test |
| FR-184-AC-5 | An `unavailable` report yields `errored` and no positive decision. | Test |
| FR-184-AC-6 | A `disagreement` report yields `absent` (zero-entry run) and no positive decision. | Test |
| FR-184-AC-7 | A passing run followed by a later run of the suite that fails or errors on the bound symbol yields `stale` and no positive decision. | Test |
| FR-184-AC-8 | A passing run followed by a later run of the suite that lacks the bound symbol yields `absent` and no positive decision. | Test |
| FR-184-AC-9 | A passing report stated for an obligation whose declared method is Test yields `qualification-rejected` naming the method and act pairing premise. | Test |
| FR-184-AC-10 | A passing report whose supplied execution association is a local/manual act, an automatic workflow trigger, or an unauthenticated or mismatched GitHub workflow/job association yields no positive decision and names the execution-association premise (FR-116-AC-9; quire-rs FR-078-AC-7). A `workflow_dispatch` GitHub Actions execution is CI and is admitted only when its authenticated run, attempt, job, invocation and checkout SHA agree. | Test |
| FR-184-AC-11 | A passing report with no supplied execution association yields `unevaluated` naming the execution association premise (FR-078-AC-7). | Test |
| FR-184-AC-12 | A passing report whose engine record lacks the observed `identity.version`, `configuredExecutable` or a `status` and `outcome` (so it is not an observation of a solver started through `producer-execute`) yields `unevaluated` naming the tool and configuration premise and no positive decision. | Test |
| FR-184-AC-13 | A satisfied agreement for an Analysis obligation yields a decision whose retained premises and result name the credit as consistency evidence only, and do not assert that the analysis request is relevant to the obligation's statement; the bridge adds no relevance claim. | Test |
| FR-184-AC-14 | A satisfied agreement with no CI checkout SHA for the `run-package` step yields `unevaluated` naming the performed-revision premise. | Test |
| FR-184-AC-15 | One report with `--discharges` for one obligation yields a decision for that obligation only; no sibling obligation is qualified by it. | Test |
| FR-184-AC-16 | One satisfied report recorded with `--discharges FR-001-AC-1,FR-001-AC-2` qualifies neither obligation: each is `unevaluated` and uncounted. | Test |
| FR-184-AC-17 | A report qualified with expected `run_kind: formal_rule` whose projection has the same run kind and one matching `formal_rule_selection` entry with state `approved` for each selected claim may qualify; the qualifier checks the selected set. | Test |
| FR-184-AC-18 | A formal-rule qualification whose projection lacks `run_kind: formal_rule`, whose report has no selection, whose mapping is missing, incomplete, extra, duplicate, or source-mismatched, or whose selected claim maps to state `draft` yields `qualification-rejected` with reason `draft_rule_in_judgement` and no positive decision. | Test |
| FR-184-AC-19 | An ordinary analysis report with no `formal_rule_selection` mapping remains eligible for qualification under the existing FR-116 premises and is not rejected as `draft_rule_in_judgement`. | Test |
| FR-184-AC-20 | A valid `workflow_dispatch` association is derived from authenticated GitHub run and job observations; a caller-supplied URL, context field, mismatched repository/workflow/event/run/attempt/job/SHA, or missing lookup cannot produce a positive decision. The exact checkout SHA is the performed revision. | Test |
| FR-184-AC-21 | A report artifact that is absent, contains zero or multiple reports for the invocation, fails artifact digest verification, fails FR-151 validation, or whose exact report-byte SHA-256 and projection are not bound to the authenticated run evidence cannot produce a positive decision. | Test |
| FR-184-AC-22 | An FR-093 assessment using the two `real-sat`/`real-unsat` controls alone, a category list without exact corpus identity, any stub-only corpus, or incomplete TC-006 validation evidence remains `unevaluated`; the owner-approved retained TC-006 corpus and its source revision, case identities, expected outcomes and observed results are required before analysis can qualify. | Test |
| FR-184-AC-23 | A run on an environment other than GitHub-hosted `ubuntu-24.04` x86_64, using Engineering Assurance other than `v0.10.1`, without real Z3 and cvc5 identity observations, or with a missing/mismatched observed context cannot produce a positive decision. | Test |

## Dependencies

- **Upstream**: [FR-116](./FR-116-owner-qualified-quire-evidence.md), [FR-151](./FR-151-differential-report-v2-adapter.md),
  [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md), [FR-153](./FR-153-analysis-result-counting-by-disposition.md),
  [NFR-033](../non-functional/NFR-033-analysis-counting-never-infers-a-binding.md), [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: filament-analyze FR-005 and FR-009; engineering-assurance FR-019 and FR-029; quire-rs FR-078.
