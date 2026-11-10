---
id: FR-116
title: "Positive owner qualification for Quire retained evidence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-093"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-078"
    type: "references"
---

# FR-116: Positive owner qualification for Quire retained evidence

## Description

For the opt-in retained-evidence path of quire-rs
[FR-078](ix://agent-ix/quire-rs/FR-078), Quoin SHALL decide whether one requested
verification act supplies positive evidence for one exact selected obligation
and its declared method. Inspection, Analysis, an executed gate and an executed
doctest can qualify without being source-symbol declarations.

This is a **draft owner contract, with implementation planned**. It supplies the
missing positive qualification premises identified by PLAT-1196; it does not
claim an implemented API, a published qualification capability, a performed
verification act or a discharged downstream criterion. PLAT-1162's apparatus
intake is a separate concern and does not establish these premises.

The contract uses [FR-030](./FR-030-evidence-store.md)'s canonical store,
[FR-032](./FR-032-evidence-auditor.md)'s existing audit and
[FR-093](./FR-093-evidence-producer-trust.md)'s bounded reliance decision. An
accepted reliance decision or `AuditReport.healthy` alone is not a positive
qualification result. Neither recording a run nor declaring `evidenceKind`
establishes that an act occurred.

### Scope and existing record boundary

The planned qualification boundary is the Quoin-owned check consumed by the
planned `quoin_quire::evidence::qualify` bridge in FR-078. Its premises below are
a typed owner input contract, **not new fields asserted to exist in
`RunRecord`, a new retained-record wire schema, or a second evidence store**.
The bridge consumes the same owner decision rather than implementing another
method/provenance checker in Quire.

FR-030's records currently carry no source commit, and FR-032 no longer compares
a persisted run commit with HEAD. This contract SHALL preserve that legacy
record and auditor behavior. Source revision and freshness for this additional
path SHALL instead be checked using correlated observations supplied at
qualification time. A store record alone cannot answer those questions.

Waiver authority and the final strict disposition of an owner-recognized waiver
remain separate policy decisions under FR-078. This contract decides positive
verification only; it does not decide that pending policy.

## Inputs

The Quoin qualification boundary SHALL receive these original owner inputs for
each requested act. The table specifies semantic members of the planned typed
input, not an existing JSON envelope or permission to place arbitrary keys in
`RunEntry.config`.

| Input | Required meaning |
|---|---|
| Selected target | The owner selection and original criterion identity supplied by FR-078, including its source/document/origin identity, exact requested target and declared method. A flat string ID alone is insufficient when the selected inventory distinguishes rows with the same rendered ID. |
| Original audit premises | The original `AuditInput`, canonical `RunRecord` and `Binding` observations, method catalog and original record/run/entry locators from the owner read boundary. The bridge invokes the shared audit itself as FR-078 specifies; a supplied `healthy` flag or precomputed pass list is insufficient. |
| Producer definition | A Quoin-owner-approved definition identity/version describing the allowed declared-method/act-kind pair, required verification acts, required result semantics, permitted target scope, actual tool/configuration premises and required CI or manual association. A suite or command name alone is not this definition. |
| Bounded reliance | The original FR-093 decision and current observation for the exact intended producer use, including accountable owner, permitted downstream decisions, validation evidence and accepted limitations. The requested positive use must be permitted, not merely another advisory use of the producer. |
| Actor authority | The actual performer or accountable attester identity, its owner-authorized scope for the requested obligation/method, and the host's authority-admission reference. A name copied into a record does not establish authority. |
| Performed-act observation | The actual completed act, its original result, performed-at time, material/scope examined, required invocation identities and their results, correlated with the selected canonical record and entry. A transcribed claim that an act passed is not an observation of that act. |
| Source context | The repository/selection context, the source revision actually examined or executed, and the actual source HEAD selected for this qualification call, supplied by the owner host. A current HEAD observation is not a substitute for the performed revision. |
| Tool and configuration context | The actual producer and exact tool version, adapter identity when used, and effective non-secret configuration facts required by the producer definition: configuration reference, arguments, feature selections and relevant environment facts. A repeated configuration label does not establish equal effective configuration. Manual acts identify the actual inspection/analysis procedure and the recording tool/version. |
| Execution association | Either the exact CI system/project/run/attempt/job and workflow/step or invocation association, or an explicit manual association permitted by the producer definition, with an accountable per-target attestation and the reason CI is not applicable. |
| Current observation | The caller-supplied observation time, current owner decision/context and the newest selected store observations needed to detect invalidation, supersession, absence or conflicting reads. Any owner-required time validity rule is explicit in the producer definition; no built-in age allowance is inferred. |

The host owns the authentication and admission of actual CI/manual observations
and actor authority. The qualifier SHALL distinguish those admitted observations
from transcribed fields and unobserved premises. It SHALL retain their owner
references so a consumer can identify the authority and observation on which a
decision relies. Quoin's comparison of supplied data does not authenticate a
person, prove an external CI service truthful, or make an untrusted host
trustworthy. That residual reliance is explicit and bounded by FR-093; shape
validation and matching strings alone cannot establish it.

## Outputs

For each exact requested target and method, Quoin SHALL return either a positive
qualification decision with its original record/entry locators and evaluated
premises, or a non-qualified result with all applicable causes. A positive
decision SHALL retain the producer-definition identity, bounded-decision and
authority references, actor, performed/current source revisions, actual
tool/configuration context, execution association and observation/freshness
basis. It is a use-specific decision, not a producer certificate.

The owner results SHALL preserve the following distinctions for FR-078's
consumer mapping. Cause details name the unanswered or mismatched premise and
the available target/record/invocation locus. An absent record has no invented
record locator or producer outcome.

| Result/category | Meaning |
|---|---|
| Qualified passing act | Every required owner premise is positively evaluated and the original required act results passed. |
| `failed` | A required performed act reported failure. |
| `errored` | A required act or its producer reported an error. |
| `skipped` | A required act was skipped; it is not a waiver. |
| `absent` | A requested retained record or required act result is absent. |
| `unevaluated` | A required premise or authoritative observation is unavailable or was not evaluated; an unknown owner contract is not an accepted default. |
| `wrong-revision` | The actually performed source revision does not match the actual selected source HEAD/context required for this act. |
| `selection-mismatch` | The audit, record association or owner context belongs to a different selected corpus. |
| `stale` | Owner revalidation/time validity or newer selected evidence invalidates the earlier reliance. Original failed/error outcomes remain visible as well. |
| `invalid-target` / `ambiguous-target` | The requested target is not in the owner selection, or distinct owner targets cannot be resolved unambiguously. |
| `record-conflict` | Required owner observations contradict each other or cannot be correlated to one consistent selected record/read. |
| `qualification-rejected` | An evaluated premise refuses the method/act pairing, actor authority, bounded use, producer definition/version, tool/configuration or required execution association. |

The qualifier SHALL order results by the owner selection's target order, then
original record/entry locus and cause category, with stable premise identity
within a category. It SHALL retain all applicable causes rather than replace a
failed, stale or unavailable act with one generic unbacked result. Existing
FR-032 finding precedence remains unchanged; this result also carries the
additional owner-premise causes.

## Behavior

### Exact correlation and bounded authority

The qualifier SHALL correlate the producer definition, actor authority,
performed-act observation, source context, tool/configuration facts and
execution association with the **same selected canonical record and entry** and
the **same exact obligation and declared method**. Matching only a suite name,
timestamp, `evidenceKind`, command string or current HEAD is insufficient. A
passing unrelated CI job cannot make an older record current. Missing
correlation is unevaluated; contradictory correlations are record-conflict.

The qualifier SHALL preserve the authored method. The performed act kind and
the declared verification method are distinct: an Analysis obligation can
require an executed gate, and a catalog-declared method can require doctests.
The producer definition specifies the compatible pair. Quoin SHALL NOT relabel
an Analysis or Inspection obligation as Test to obtain a convenient binding.

When the original audit does not establish healthy evidence, the qualifier
SHALL retain the audit's original failure/unevaluated details without granting
positive qualification. When the catalog or a required owner premise is absent,
skipping that check in the legacy audit SHALL NOT constitute a positive answer
on this opt-in path.

An accepted or accepted-with-limitations FR-093 assessment is necessary but not
sufficient. The qualifier SHALL require a currently observed bounded use that
permits the requested downstream decision. A limitation that excludes a
required act or requested target cannot be treated as an accepted exception.
Unobserved, invalidated and not-accepted reliance cannot produce a positive
decision. The additional method, source, actor and execution premises in this
contract remain mandatory even if an FR-093 optional trigger did not ask them.

One performed act may supply evidence for several explicitly examined targets,
but the qualifier SHALL issue a separate per-target decision with the
corresponding observation for each. It SHALL NOT extend an accepted decision to
every Inspection row, a sibling criterion or a shared test-case reference.
FR-030's legacy indirect binding rules remain unchanged; they do not by
themselves establish these positive per-target observations.

### Evidence required for each act kind

| Act kind | Positive observation required |
|---|---|
| Inspection | An actual completed inspection report identifies the exact target, material at the performed revision, performer/attester, applicable inspection questions and their per-target passing findings. Manual association requires owner-authorized scope and an explicit reason CI is not applicable. A schema-valid inspections table or a transcribed `pass` alone is insufficient. |
| Analysis | An actual completed analysis identifies the target, method/procedure, inputs, result and every act required by its producer definition. A gate-shaped Analysis such as IR FR-038-AC-195 requires observations of **both** warnings-denied Clippy lanes, their actual arguments/features and results, and their association with the required CI `make lint` execution. A one-lane run or a command merely present in a Makefile cannot qualify the whole act. A permitted manual analysis uses the same per-target authority/attestation rule as manual Inspection. |
| Executed gate | Actual invocation observations identify the gate procedure at the performed revision, each required sub-invocation, effective configuration, exit/result semantics and correlated CI run/attempt/job. A zero exit that masks a failed required sub-invocation does not establish a passing gate. |
| Executed doctest | Actual harness output identifies the selected documentation example/target, execution mode, required cases and passing results under the observed tool/configuration. Discovery, compilation-only/`no-run`, ignored or skipped examples do not establish execution. A failed or errored example keeps its original outcome. |

For a definition requiring CI, the qualifier SHALL require the actual run,
attempt, job and invocation association; an arbitrary URL or unrelated green
job is insufficient. For a definition permitting a manual act, the qualifier
SHALL require the accountable per-target attestation and the explicit
not-applicable reason instead of inventing a CI run. A manual reason cannot
excuse a definition's required CI execution.

### Freshness at qualification time

The qualifier SHALL compare the performed revision from the correlated act
observation with the actual selected source HEAD supplied by the host. It SHALL
report a mismatch as wrong-revision and a missing/unanswered performed revision
as unevaluated. Supplying the current HEAD next to an unrelated old record
cannot satisfy this check. These are read-time premises of this opt-in use, not
restored commit fields, tool-revision pins or digest requirements in FR-030.

The qualifier SHALL evaluate the producer definition's current observation and
owner-required validity conditions using the caller-supplied time and facts.
It SHALL reject expired or invalidated reliance as stale, preserve a missing
observation as unevaluated and reject an internally contradictory time/context
as record-conflict. A recent timestamp alone does not establish freshness.

The qualifier SHALL use the newest selected canonical observations under
FR-030/FR-032. A newer failed or errored act SHALL prevent reuse of an older
passing act for that same required evidence; its original outcome and the
applicable stale cause remain visible. A changed or contradictory record/read
association cannot be resolved by choosing whichever positive record was
visited first. Qualification SHALL NOT write or repair the store to obtain a
pass.

### Consumer and waiver boundary

Only Quoin's evaluated owner result supplies these premises to the planned
FR-078 bridge; a Quire-local configuration map, source tag or reconstructed
pass list cannot stand in for it. The bridge's owner-neutral projection retains
the original selected target and locators without manufacturing a
`SymbolGraph` node or source binder. A positive decision supplies no blanket
strict-coverage pass: the consumer still evaluates inventory completeness,
other requested acts and its inherited strict premises.

The qualifier SHALL NOT interpret a waiver as a passing verification act.
Unknown waiver-shaped input remains unevaluated with the pending-policy detail
specified by FR-078. An eventual owner-recognized waiver remains a separate
owner-policy outcome; this contract SHALL NOT map it silently to Pass, Skip,
Fail or absence, or determine its eventual strict exit. Positive qualification
can be reviewed independently of that unresolved waiver policy.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-116-CON-1 | The qualification computation SHALL be a pure reader of supplied owner inputs, with no clock read, filesystem/network lookup, producer execution or store mutation inside it. The host assembles observations. | Architecture | Inspection |
| FR-116-CON-2 | The new opt-in path SHALL preserve FR-030 record formats, FR-032 legacy audit behavior and FR-115 legacy matrix semantics without adding a second store, persisted commit/digest gate or global producer badge. | Architecture | Inspection |
| FR-116-CON-3 | The owner result SHALL expose its conditional reliance on admitted host observations and bounded authority, identifying field presence, transcription and matching identity strings as insufficient to establish external authenticity. | Design | Inspection |

## Acceptance Criteria

All criteria below describe planned implementation obligations. This draft
claims no executable acceptance evidence or current downstream discharge.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-116-AC-1 | A passing act with the complete admitted owner context qualifies only its exact requested selected target/method, retaining original record/entry locators and every evaluated authority, source, tool/configuration and execution premise. | Test |
| FR-116-AC-2 | A completed manual Inspection with authorized per-target findings and an allowed explicit CI-not-applicable reason can qualify; a valid inspections table, recorded actor name, transcribed pass or missing attestation cannot. | Test |
| FR-116-AC-3 | The Analysis producer definition for IR FR-038-AC-195 requires actual observations of both warning-denied Clippy lanes and their required CI gate association; one lane, a missing feature/argument, an unexecuted command or a failed lane does not qualify it, and its method remains Analysis. | Test |
| FR-116-AC-4 | A gate qualifies only when every required sub-invocation and result is observed under its effective configuration; a masked failing sub-invocation is not accepted from the aggregate zero exit. | Test |
| FR-116-AC-5 | A doctest qualifies from actual passing execution of the required selected examples; discovery-only, no-run, ignored, skipped, failed and errored controls retain their distinct non-qualified outcomes. | Test |
| FR-116-AC-6 | Missing authority is unevaluated; evaluated authority excluding the target/method/use is qualification-rejected. An accepted advisory use or limitation excluding the required act grants no positive verification decision. | Test |
| FR-116-AC-7 | A mismatched performed revision is wrong-revision, an unavailable performed revision is unevaluated, and a current HEAD paired with an unrelated historical record cannot make it current. No persisted run commit is introduced. | Test |
| FR-116-AC-8 | A record, source/tool observation or execution witness belonging to a different selection, record/entry, obligation or method cannot qualify the requested act; missing correlation is unevaluated, a different selection is selection-mismatch, and contradictory correlations are record-conflict. | Test |
| FR-116-AC-9 | A definition requiring CI refuses a mismatched run/attempt/job/workflow/invocation association; a manual exception cannot replace required CI, while an allowed manual act retains its real attester and explicit not-applicable reason without an invented run. | Test |
| FR-116-AC-10 | The actual tool/version, adapter when required and effective configuration must match the bounded producer definition; a reused configuration name with different required arguments/features is rejected, and missing observations remain unevaluated. | Test |
| FR-116-AC-11 | The declared method/act-kind pair is checked through its original catalog and owner definition; absent catalog/premises do not become acceptance, unknown or incompatible evaluated methods are rejected, and `evidenceKind` alone grants nothing. | Test |
| FR-116-AC-12 | Invalidated/expired owner context is stale; absent observation is unevaluated; a newer failed/error canonical observation prevents an older passing act from qualifying while retaining both original outcome and owner freshness detail. | Test |
| FR-116-AC-13 | Failed, errored, skipped, absent, unevaluated, wrong-revision, selection-mismatch, stale, invalid-target, ambiguous-target, record-conflict and qualification-rejected causes retain their original target/available locator and stable deterministic ordering; applicable causes are not collapsed to one generic gap. | Test |
| FR-116-AC-14 | Two owner targets with a shared rendered ID are not collapsed; unknown or ambiguous targets cannot qualify, and a shared suite/test-case reference does not grant a sibling target a decision without its own observed act. | Test |
| FR-116-AC-15 | A genuine healthy audit with no positive owner premises, or a matching FR-093 reliance assessment alone, does not qualify an act; the bridge consumes the owner result rather than a precomputed healthy flag or a second checker. | Test |
| FR-116-AC-16 | Accepted retained claims create no source symbols/binders or copied store records; legacy no-evidence audit/matrix behavior and the original canonical store formats remain unchanged. | Test |
| FR-116-AC-17 | Waiver-shaped input produces no positive qualification and remains distinct under FR-078's pending owner policy; no fixture or report claims the final recognized-waiver strict policy settled by this contract. | Test |
| FR-116-AC-18 | The result exposes the host-admission and bounded authority basis for every positive decision; an unadmitted self-asserted context cannot obtain a positive decision merely by copying the expected field values. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) for original canonical records and locators; [FR-032](./FR-032-evidence-auditor.md) for the existing shared audit; [FR-093](./FR-093-evidence-producer-trust.md) for bounded owner reliance and invalidation; the original method catalog and the caller's admitted CI/manual/source observations.
- **Downstream**: quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078) / PLAT-1160's planned bridge and neutral backing projection; IR-1006 method-consumer qualification; the affected IR/RT Inspection and Analysis rows. [FR-115](./FR-115-evidence-backed-test-matrix.md)'s existing join is not itself this opt-in owner qualification.
- **Separate**: PLAT-1162's evidence-gate apparatus intake does not supply this contract; the owner-recognized waiver representation and final strict policy remain with the policy owner identified by quire-rs FR-078.
