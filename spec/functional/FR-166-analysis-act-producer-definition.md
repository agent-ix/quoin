---
id: FR-166
title: "Draft FR-116 producer definition for the analysis act (owner approval required)"
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

# FR-166: Draft FR-116 producer definition for the analysis act (owner approval required)

## Description

[FR-116](./FR-116-owner-qualified-quire-evidence.md) requires a Quoin-owner-approved producer definition before an
act can qualify, and [FR-153](./FR-153-analysis-result-counting-by-disposition.md) names the content of the
definition for the analysis act as a decision it does not take. This requirement is the **proposed content** of that
definition: what runs, what the host supplies, what a positive decision means and what it does not mean. It is a
draft. It takes effect only on the approval stated in the Owner approval section; until then the default below
holds and FR-153 is unchanged.

The definition has the identity `analysis-act` and the version `1`, to be bound to the approval when given. It
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

The act is run by CI or manually. The definition proposes that the host supply the association FR-116 requires: for
CI, the exact system, project, run, attempt, job and invocation of the `run-package` step; for a manual run, the
accountable per-target attestation and the explicit reason CI is not applicable. Whether manual is permitted is an
owner point (below).

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
   selected HEAD, tool and configuration, execution association, and freshness.

The decision is use-specific (FR-116): it says that this act, run under this definition, completed with both solvers
agreeing the analysed clauses are consistent, for this exact target and method. A decision **does not assert**:

- that the analysis request is relevant to the obligation's statement, that it is the right analysis, or that it
  proves the obligation. The `--discharges` id is the recorder's assertion. Quoin does not judge it (FR-153); the
  actor-authority premise of FR-116 evaluates only whether the asserting actor may make it;
- that a solver is sound, or that any property beyond the one consistency request the runner built holds;
- the outcome of any other obligation bound to the same entry, or of a sibling criterion (FR-116 per-target rule);
- anything about a `disagreement`, which is stored as a zero-entry run and counts as `absent` (FR-153).

A `refuted` agreement is `failed`; an `inconclusive` or `unavailable` report is `errored`; neither can qualify.

### Default until approved

Until the owner approves this definition (or a changed one), no producer definition for the analysis act is
approved. Every analysis-backed obligation SHALL be `unevaluated`, naming the producer-definition premise, and none
SHALL be counted (FR-153). Nothing in this draft changes that.

## Owner approval

Approval of this definition is a decision of the FR-116 owner (Peter). The agent that drafted it, a ticket, a review
or a comment cannot give it. **This pull request is a DRAFT and SHALL NOT merge before that approval.** After approval,
the approval is recorded in this requirement by removing the draft marking, and FR-153's "no producer definition is
approved" statement is then amended in the same change.

Open points the owner must rule on. Each is a fact this draft cannot ground in the cited specs:

1. **Source revision.** The report carries no source revision, so the performed revision FR-116 compares with the
   selected HEAD is not supplied by it. Which host observation (for example the CI checkout SHA of the step that ran
   `run-package`) supplies it, and is it acceptable that without one the result is `unevaluated`?
2. **CI or manual.** Is CI the only permitted association, or is a manual run with per-target attestation allowed? If
   CI, which system and which workflow step is the required invocation?
3. **Package to obligation.** Nothing ties the contract package, or the clauses it holds, to the obligation's
   statement beyond the recorder's `--discharges`. Is that assertion plus actor authority sufficient, or must the
   definition require a further link (which would be a change to FR-152)?
4. **Tool and configuration pins.** Which solver versions, solver limits and `run-package` version are required?
   quire-analyze FR-005 states the report has no executable digest or expected-version check, and this contract adds
   no digest. The definition may require only that the observed `identity.version` and `producer.version` fall in an
   owner-named set; which set?
5. **Actor authority.** Who may be the asserting actor for an Analysis obligation, and what is the host's
   authority-admission reference?
6. **Bounded reliance.** Is there an FR-093 assessment of `quire-analyze` and the solvers for this positive use, and
   who is its accountable owner?
7. **Freshness.** Is any time validity rule required beyond revision and supersession? FR-116 infers no built-in age.
8. **Scope of one act.** Does one report qualify only the one obligation stated, or each of several obligations
   stated with the same report? (FR-116 requires a separate per-target decision either way.)
9. **Waiver.** How an owner waiver of an analysis result is represented stays with the policy owner (quire-rs
   FR-078); this draft does not decide it.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-166-CON-1 | The definition SHALL add no field to the report, the run entry or the store, no compatibility reader and no digest, ledger or pin of a file or version (FR-116-CON-2). | Architecture | Inspection |
| FR-166-CON-2 | Qualification SHALL run no solver and read only supplied observations (FR-116-CON-1; FR-153-CON-2). | Architecture | Inspection |

## Acceptance Criteria

All criteria describe the proposed definition; none claims an implemented qualifier.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-166-AC-1 | With no approved definition, every analysis-backed obligation is `unevaluated` naming the producer-definition premise and none is counted. | Test |
| FR-166-AC-2 | Under an approved definition, a `satisfied` agreement bound by `--discharges` to an Analysis obligation, latest in its suite and with every other FR-116 premise admitted, yields a positive decision for that target only. | Test |
| FR-166-AC-3 | Under an approved definition, a `refuted`, `inconclusive`, `unavailable` or `disagreement` report, a superseded run, a Test-method obligation, or a missing execution association yields no positive decision, each with its FR-153 or FR-116 category. | Test |
| FR-166-AC-4 | The definition names `quire-analyze run-package` with Z3 and cvc5 started only through `producer-execute`, and the inputs and outputs above, each citing its owning requirement. | Inspection |
| FR-166-AC-5 | The text states that a positive decision asserts no request relevance, and that approval is the FR-116 owner's decision with the open points listed. | Inspection |

## Dependencies

- **Upstream**: [FR-116](./FR-116-owner-qualified-quire-evidence.md), [FR-151](./FR-151-differential-report-v2-adapter.md),
  [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md), [FR-153](./FR-153-analysis-result-counting-by-disposition.md),
  [NFR-033](../non-functional/NFR-033-analysis-counting-never-infers-a-binding.md), [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-analyze FR-005 and FR-009; engineering-assurance FR-019 and FR-029; quire-rs FR-078.
