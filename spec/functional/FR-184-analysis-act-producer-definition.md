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
  - target: "ix://agent-ix/quire-analyze/FR-009"
    type: "references"
  - target: "ix://agent-ix/quire-analyze/FR-005"
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

The definition has the identity `analysis-act` and the version `1`. It
cites the owners of the report, the runner and the executor and does not restate them.

### What runs

The act is one invocation of `quire-analyze run-package` ([quire-analyze FR-009](ix://agent-ix/quire-analyze/FR-009))
with `--package`, `--z3`, `--cvc5`, `--producer-execute` and `--output`, the arguments and their absolute-path rules
as that requirement states them. The runner builds one consistency request over all analysable clauses of the
package. It starts the Z3 and cvc5 solvers only through `producer-execute`, which is the
`engineering-assurance producer-execute` command ([engineering-assurance FR-019 and FR-029](ix://agent-ix/engineering-assurance/FR-029)),
one invocation for each engine's version and one for its query. No other way of starting a solver is part of this
act, and a report from a solver started any other way is not an observation of it.

## Inputs

| Input | Source and meaning |
|---|---|
| Contract package | The `quire.contract-package/v1` document given as `--package` (quire-analyze FR-009). It names no analysis request; the runner fixes the request. |
| Solver executables | `--z3` and `--cvc5`, absolute paths with no symbolic-link component (quire-analyze FR-009). |
| Executor | `--producer-execute`, the absolute path of the `engineering-assurance` executable, never searched for on `PATH`. |
| Declared method | The selected obligation's declared method, which must be Analysis (FR-116 method/act pairing; FR-152; FR-153). |
| Obligation statement | The `--discharges` id the recorder gave to `quoin evidence record` (FR-152). It is the only binding source. |

## Outputs

- One `quire.differential-report/v2` document and the runner's exit status, whose classes are those of
  [quire-analyze FR-009](ix://agent-ix/quire-analyze/FR-009). The report is read by
  the `differential-report` adapter, which accepts it only when `quire_analyze::validate_report_document` does
  (FR-151).
- When recorded, the source report is validated and reduced to the FR-151
  allowlisted projection in the FR-030
  `analysis-reports/<suite>/<timestamp>.json` companion file. It is linked to
  exactly one run by the shared canonical suite/timestamp key and is covered
  together with that run by the host's signature-verification of the commit
  that adds the pair. The projection adds no member to the source report schema,
  `RunRecord`, `RunEntry` or `--run-observation` object.
- The source report is validated against the closed `quire.differential-report/v2` schema. The FR-151 projection
  retains only `producer.name`, `producer.version`, each engine's `configuredExecutable`, observed `identity.path`
  and `identity.version` when present, each engine's `status` and `outcome`, the `disposition`, and the `agreedStatus`.
  Its `symbol`, when present, is copied from the paired run entry; it is absent for a disagreement, which has no
  run entry. The projection does not retain the source report's `requestDigest` separately, query, model, stdout,
  stderr or raw solver output. These projected values are the transcribed observations the performed-act premise reads.
- The report supplies no CI system, run, attempt or job, no actor, no performed-at time and no source revision. The
  adapter's entry carries no `evidenceKind` and no `traceIds` (FR-151). Those premises come from the host (FR-116) or
  are open points below.

### Execution association

The qualifier SHALL admit an execution association only from CI; a manual run is not an association this definition
admits. The required invocation is the
`run-package` step of the CI workflow that runs the analysis act. The host supplies the association FR-116 requires:
the exact system, project, run, attempt, job and invocation of that step. This requirement names the step and edits no
workflow file; changing the step needs the owner's clearance.

### Source revision

The qualifier SHALL take the performed revision FR-116 compares with the selected HEAD from the CI checkout SHA of
the `run-package` step, supplied by the host. The report carries none. Without one, the result is `unevaluated`,
naming the performed-revision premise.

### Tool and configuration

No named version set and no pin is required. The qualifier reads the observed solver `identity.version` and the
`producer.version` in the FR-151 projection, and requires no particular value of either; the definition records no
allowed version set (FR-184-CON-4 below).

### Actor authority

The qualifier SHALL take the asserting actor to be the signer of the record, mapped to a person through the
signed-records feature ([PLAT-1433](https://linear.app/agent-ix/issue/PLAT-1433)). `--discharges` together with that authority is sufficient
to bind the obligation only when the same verified signed commit contains the
exact selected run and its exact companion projection, and both selected files
match their bytes in that commit. A signature result for the run alone, a
projection-only commit, a later replaced projection, or a run/projection
mismatch does not admit actor authority. No commit id is persisted in the
store.

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
(quire-analyze FR-009 starts them no other way). The qualifier SHALL read only the FR-151 projection's fields to decide
this, as FR-184-CON-2 requires: `producer.name` and `producer.version` of the runner, and for each of the Z3 and cvc5
engines a `configuredExecutable` that is an absolute path, an observed `identity.path` and `identity.version` (the
version invocation that `producer-execute` ran first, FR-009), and a `status` and `outcome` from the closed set
(FR-005-AC-6; FR-009 result-state table). If any required value is absent for either engine, the projection does not
establish an observation of the act; the result is `unevaluated` naming the tool and configuration premise, and no
positive decision is made.

A `refuted` agreement is `failed`; an `inconclusive` or `unavailable` report is `errored`; neither can qualify.

### Default `unevaluated`

The definition is approved, and two premises keep the result `unevaluated`, each named in the result:

- **Premise: no FR-093 assessment.** Bounded reliance on `quire-analyze` and the solvers for this positive use
  ([FR-093](./FR-093-evidence-producer-trust.md)) is open: no assessment exists. Every analysis-backed obligation
  SHALL stay `unevaluated` and uncounted until an assessment is written.
- **Premise: no CI checkout SHA.** A result with no CI checkout SHA of the `run-package` step SHALL be `unevaluated`.

### Open points

- **Report stating several obligations.** Rule stated by the lead from answer 8, owner may amend: a report whose
  `--discharges` states more than one obligation qualifies none of them.
- **Producer-execute marker.** The report carries no field that names `producer-execute` directly; the qualifier
  reads the engine fields listed above. A dedicated marker would be a quire-analyze change and is not required here.

## Owner approval

Owner approval recorded 2026-11-13 via the planner relay, confirmed by the owner in session. The FR-116 owner (Peter)
approved this definition with these answers:

1. **Source revision.** The CI checkout SHA of the `run-package` step; without one the result is `unevaluated`.
2. **CI only.** No manual runs. The workflow step needs the owner's clearance to edit, so this requirement names the
   step and no workflow file is edited.
3. **Package to obligation.** The recorder's `--discharges` plus actor authority is sufficient; FR-152 is unchanged.
   The credit is consistency evidence only, and the requirement-to-formalization link (Linear VER-75) will tighten it.
4. **Tool and configuration.** No named version set; only the observed solver and runner versions are recorded; no pins.
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

Item 6 is unresolved: nothing counts as covered until the FR-093 assessment exists.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-184-CON-1 | The definition SHALL add no field to the closed v2 report schema, `RunRecord` or `RunEntry` (FR-116-CON-2). | Compatibility | Inspection |
| FR-184-CON-2 | Qualification SHALL run no solver and read only supplied observations (FR-116-CON-1; FR-153-CON-2). | Architecture | Inspection |
| FR-184-CON-3 | The store SHALL retain only the FR-151 allowlisted projection, never the source report or its query/model/solver output, and never copy it into the run record; an older run with no companion remains unevaluated and is not migrated. | Compatibility | Inspection |
| FR-184-CON-4 | The definition SHALL persist no checkout SHA, CI association, signer principal, commit id, content digest, ledger or version pin (FR-116-CON-2). A producer-authored request digest appears only inside an existing run-entry symbol when one exists; it is not separately retained in the projection, and quoin computes no digest. | Architecture | Inspection |

## Acceptance Criteria

The criteria describe the approved definition; none claims an implemented qualifier.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-184-AC-1 | With no FR-093 assessment of `quire-analyze` and the solvers, every analysis-backed obligation is `unevaluated` naming the bounded-reliance premise and none is counted. | Test |
| FR-184-AC-2 | A `satisfied` agreement bound by `--discharges` to an Analysis obligation, latest in its suite, with a CI checkout SHA and every other FR-116 premise admitted, yields a positive decision for that target only. | Test |
| FR-184-AC-3 | A `refuted` agreement report bound to an Analysis obligation yields `failed` and no positive decision. | Test |
| FR-184-AC-4 | An `inconclusive` report yields `errored` and no positive decision. | Test |
| FR-184-AC-5 | An `unavailable` report yields `errored` and no positive decision. | Test |
| FR-184-AC-6 | A `disagreement` report yields `absent` (zero-entry run) and no positive decision. | Test |
| FR-184-AC-7 | A passing run followed by a later run of the suite that fails or errors on the bound symbol yields `stale` and no positive decision. | Test |
| FR-184-AC-8 | A passing run followed by a later run of the suite that lacks the bound symbol yields `absent` and no positive decision. | Test |
| FR-184-AC-9 | A passing report stated for an obligation whose declared method is Test yields `qualification-rejected` naming the method and act pairing premise. | Test |
| FR-184-AC-10 | A passing report whose supplied execution association is not CI (for example a manual association) yields `qualification-rejected` naming the execution association premise (FR-116-AC-9; quire-rs FR-078-AC-7). | Test |
| FR-184-AC-11 | A passing report with no supplied execution association yields `unevaluated` naming the execution association premise (FR-078-AC-7). | Test |
| FR-184-AC-12 | A passing report whose engine record lacks the observed `identity.version`, `configuredExecutable` or a `status` and `outcome` (so it is not an observation of a solver started through `producer-execute`) yields `unevaluated` naming the tool and configuration premise and no positive decision. | Test |
| FR-184-AC-13 | A satisfied agreement for an Analysis obligation yields a decision whose retained premises and result name the credit as consistency evidence only, and do not assert that the analysis request is relevant to the obligation's statement; the bridge adds no relevance claim. | Test |
| FR-184-AC-14 | A satisfied agreement with no CI checkout SHA for the `run-package` step yields `unevaluated` naming the performed-revision premise. | Test |
| FR-184-AC-15 | One report with `--discharges` for one obligation yields a decision for that obligation only; no sibling obligation is qualified by it. | Test |
| FR-184-AC-16 | One satisfied report recorded with `--discharges FR-001-AC-1,FR-001-AC-2` qualifies neither obligation: each is `unevaluated` and uncounted. | Test |

## Dependencies

- **Upstream**: [FR-116](./FR-116-owner-qualified-quire-evidence.md), [FR-151](./FR-151-differential-report-v2-adapter.md),
  [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md), [FR-153](./FR-153-analysis-result-counting-by-disposition.md),
  [NFR-033](../non-functional/NFR-033-analysis-counting-never-infers-a-binding.md), [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-analyze FR-005 and FR-009; engineering-assurance FR-019 and FR-029; quire-rs FR-078.
