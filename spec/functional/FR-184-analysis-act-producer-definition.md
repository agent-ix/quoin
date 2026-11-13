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

- One `quire.differential-report/v2` document and the runner's exit status (`0` satisfied, `1` refuted, `2` usage or
  input error, `3` non-conclusive, `4` publication or executor failure; quire-analyze FR-009). The report is read by
  the `differential-report` adapter, which accepts it only when `quire_analyze::validate_report_document` does
  (FR-151).
- The report supplies to the FR-116 tool and configuration context only what quire-analyze FR-005-AC-6 requires it to
  retain: `producer.name` and `producer.version`; for each engine its name, `configuredExecutable`, observed
  `identity.path` and `identity.version` (when available), `limits`, `status` and `outcome`; the `disposition` and
  `agreedStatus`; the `requestDigest`. These are the transcribed observations the performed-act premise reads.
- The report supplies no CI system, run, attempt or job, no actor, no performed-at time and no source revision. The
  adapter's entry carries no `evidenceKind` and no `traceIds` (FR-151). Those premises come from the host (FR-116) or
  are open points below.

### Execution association

The act is run by CI only; a manual run is not an association this definition admits. The required invocation is the
`run-package` step of the CI workflow that runs the analysis act. The host supplies the association FR-116 requires:
the exact system, project, run, attempt, job and invocation of that step. This requirement names the step and edits no
workflow file; changing the step needs the owner's clearance.

### Source revision

The performed revision FR-116 compares with the selected HEAD is the CI checkout SHA of the `run-package` step,
supplied by the host. The report carries none. Without one, the result is `unevaluated`, naming the performed-revision
premise.

### Tool and configuration

No named version set and no pin is required. The definition records the observed solver `identity.version` and the
`producer.version` the report supplies, and requires no particular value of either (FR-184-CON-1 below).

### Actor authority

The asserting actor is the signer of the record, mapped to a person through the signed-records feature
([PLAT-1433](https://linear.app/agent-ix/issue/PLAT-1433)). `--discharges` together with that authority is sufficient
to bind the obligation; no further link is required and FR-152 is unchanged.

### What a positive qualifier decision means

The bridge MAY emit a record claim for an obligation only when **all** of these hold, and each is a premise the
qualifier evaluates and retains (FR-116):

1. the report's `disposition` is `agreement` and `agreedStatus` is `satisfied`, so both engines, Z3 and cvc5, are
   conclusive with the same status (FR-151: entry outcome `pass`);
2. the entry is bound to the obligation by `--discharges` and the binding survives the FR-032 audit (FR-152);
3. the obligation's declared method is Analysis;
4. the entry is in the latest run of its suite and no later run of that suite fails, errors or lacks its symbol
   (FR-153; the run is not superseded);
5. every other FR-116 premise holds: bounded reliance (FR-093), actor authority, performed revision against the
   selected HEAD, tool and configuration, execution association, and freshness. Freshness is revision and
   supersession only; no age rule applies.

The decision is use-specific (FR-116): it says that this act, run under this definition, completed with both solvers
agreeing the analysed clauses are consistent, for this exact target and method. One report qualifies only the one
obligation stated with it. The credit is **consistency evidence only**: it says the analysed clauses are consistent and
nothing about the obligation's meaning. The requirement-to-formalization link
([Linear VER-75](https://linear.app/agent-ix/issue/VER-75)) will tighten it. A decision **does not assert**:

- that the analysis request is relevant to the obligation's statement, that it is the right analysis, or that it
  proves the obligation. The `--discharges` id is the recorder's assertion. Quoin does not judge it (FR-153); the
  actor-authority premise of FR-116 evaluates only whether the asserting actor may make it;
- that a solver is sound, or that any property beyond the one consistency request the runner built holds;
- the outcome of any other obligation bound to the same entry, or of a sibling criterion (FR-116 per-target rule);
- anything about a `disagreement`, which is stored as a zero-entry run and counts as `absent` (FR-153).

A `refuted` agreement is `failed`; an `inconclusive` or `unavailable` report is `errored`; neither can qualify.

### Default `unevaluated`

The definition is approved, and two premises keep the result `unevaluated`, each named in the result:

- **Premise: no FR-093 assessment.** Bounded reliance on `quire-analyze` and the solvers for this positive use
  ([FR-093](./FR-093-evidence-producer-trust.md)) is open: no assessment exists. Every analysis-backed obligation
  SHALL stay `unevaluated` and uncounted until an assessment is written.
- **Premise: no CI checkout SHA.** A result with no CI checkout SHA of the `run-package` step SHALL be `unevaluated`.

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
9. **Waiver.** OPEN. It stays with quire-rs FR-078, as a separate ruling.

Items 6 and 9 are unresolved.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-184-CON-1 | The definition SHALL add no field to the report, the run entry or the store, no compatibility reader and no digest, ledger or pin of a file or version (FR-116-CON-2). | Architecture | Inspection |
| FR-184-CON-2 | Qualification SHALL run no solver and read only supplied observations (FR-116-CON-1; FR-153-CON-2). | Architecture | Inspection |

## Acceptance Criteria

The criteria describe the approved definition; none claims an implemented qualifier.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-184-AC-1 | With no FR-093 assessment of `quire-analyze` and the solvers, every analysis-backed obligation is `unevaluated` naming the bounded-reliance premise and none is counted. | Test |
| FR-184-AC-2 | A `satisfied` agreement bound by `--discharges` to an Analysis obligation, latest in its suite, with a CI checkout SHA and every other FR-116 premise admitted, yields a positive decision for that target only. | Test |
| FR-184-AC-3 | A `refuted`, `inconclusive`, `unavailable` or `disagreement` report, a superseded run, a Test-method obligation, or a missing or non-CI execution association yields no positive decision, each with its FR-153 or FR-116 category. | Test |
| FR-184-AC-4 | The definition names `quire-analyze run-package` with Z3 and cvc5 started only through `producer-execute`, and the inputs and outputs above, each citing its owning requirement. | Inspection |
| FR-184-AC-5 | The text states that a positive decision asserts no request relevance and is consistency evidence only, and records the owner approval with items 6 and 9 explicitly open. | Inspection |
| FR-184-AC-6 | A satisfied agreement with no CI checkout SHA for the `run-package` step yields `unevaluated` naming the performed-revision premise. | Test |
| FR-184-AC-7 | One report with `--discharges` for one obligation yields a decision for that obligation only; no sibling obligation is qualified by it. | Test |

## Dependencies

- **Upstream**: [FR-116](./FR-116-owner-qualified-quire-evidence.md), [FR-151](./FR-151-differential-report-v2-adapter.md),
  [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md), [FR-153](./FR-153-analysis-result-counting-by-disposition.md),
  [NFR-033](../non-functional/NFR-033-analysis-counting-never-infers-a-binding.md), [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-analyze FR-005 and FR-009; engineering-assurance FR-019 and FR-029; quire-rs FR-078.
