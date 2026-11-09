---
id: SR-187
title: code-review review of PLAT-1196 owner qualification
type: SpecReview
analysis: code-review
scope: agent-ix/quoin; spec/functional/FR-116-owner-qualified-quire-evidence.md; spec/functional/index.md;
  Ticket PLAT-1196
review_set: all
---

## Summary

Ticket: PLAT-1196. Independent frozen-head review of quoin#728 at 6cb037f563e6ad80fa8e1e05262af3e19e41cd7e.

Spec-only diff integrity, duplication and contract faithfulness reviewed. No production code, Rust, executable schema, manifests, locks or tests change. FR-116 owns the missing positive qualification decision; references preserve canonical ownership rather than copying the store/auditor or Quire reconciliation. Implementation/test discharge is explicitly unclaimed.

## Verdict

**PASS** — no defects found in the scoped draft contract. Owner review remains required; keep the PR draft.

## Assurance Context

AP-201 (spec/assurance/AP-201-finding-quality.md) governs benchmark/measurement collection, scoring, persistence, comparison and rendered measurement views; this spec-only positive evidence contract changes none of those, so it does not apply. AP-202 (spec/assurance/AP-202-jev-lens-evaluation.md) governs live Jev lens GO/NO-GO evaluation; no such run/change occurs, so it does not apply. No applicable enforced-review profile was found. This is direct independent draft review, not human owner acceptance.

## Validation and limits

Measured tools: quoin 0.29.1; released quire 0.38.1 (engine 0.51.0). Scoped FR and index validation passed; FR grammar summary: 1/1 clean, zero findings. Ambient catalog diagnostics report duplicate archetypes/inverse-edge labels and inline data-schema guidance; none identifies a defect in this diff. Advisor output is retained beside these artifacts. Review covers planned contract semantics only, with no claim of implemented APIs, performed verification, downstream discharge or final waiver policy approval.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
clean: true
scope:
- id: FR-116-CON-1
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The qualification computation SHALL be a pure reader of supplied owner inputs, with no clock read, filesystem/network
    lookup, producer execution or store mutation inside it. The host assembles observations.
- id: FR-116-CON-2
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The new opt-in path SHALL preserve FR-030 record formats, FR-032 legacy audit behavior and FR-115 legacy
    matrix semantics without adding a second store, persisted commit/digest gate or global producer badge.
- id: FR-116-CON-3
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The owner result SHALL expose its conditional reliance on admitted host observations and bounded authority,
    identifying field presence, transcription and matching identity strings as insufficient to establish external
    authenticity.
- id: FR-116-AC-1
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A passing act with the complete admitted owner context qualifies only its exact requested selected target/method,
    retaining original record/entry locators and every evaluated authority, source, tool/configuration and execution
    premise.
- id: FR-116-AC-2
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A completed manual Inspection with authorized per-target findings and an allowed explicit CI-not-applicable
    reason can qualify; a valid inspections table, recorded actor name, transcribed pass or missing attestation
    cannot.
- id: FR-116-AC-3
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The Analysis producer definition for IR FR-038-AC-195 requires actual observations of both warning-denied
    Clippy lanes and their required CI gate association; one lane, a missing feature/argument, an unexecuted command
    or a failed lane does not qualify it, and its method remains Analysis.
- id: FR-116-AC-4
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A gate qualifies only when every required sub-invocation and result is observed under its effective configuration;
    a masked failing sub-invocation is not accepted from the aggregate zero exit.
- id: FR-116-AC-5
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A doctest qualifies from actual passing execution of the required selected examples; discovery-only,
    no-run, ignored, skipped, failed and errored controls retain their distinct non-qualified outcomes.
- id: FR-116-AC-6
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Missing authority is unevaluated; evaluated authority excluding the target/method/use is qualification-rejected.
    An accepted advisory use or limitation excluding the required act grants no positive verification decision.
- id: FR-116-AC-7
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A mismatched performed revision is wrong-revision, an unavailable performed revision is unevaluated,
    and a current HEAD paired with an unrelated historical record cannot make it current. No persisted run commit
    is introduced.
- id: FR-116-AC-8
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A record, source/tool observation or execution witness belonging to a different selection, record/entry,
    obligation or method cannot qualify the requested act; missing correlation is unevaluated, a different selection
    is selection-mismatch, and contradictory correlations are record-conflict.
- id: FR-116-AC-9
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A definition requiring CI refuses a mismatched run/attempt/job/workflow/invocation association; a manual
    exception cannot replace required CI, while an allowed manual act retains its real attester and explicit not-applicable
    reason without an invented run.
- id: FR-116-AC-10
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The actual tool/version, adapter when required and effective configuration must match the bounded producer
    definition; a reused configuration name with different required arguments/features is rejected, and missing
    observations remain unevaluated.
- id: FR-116-AC-11
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The declared method/act-kind pair is checked through its original catalog and owner definition; absent
    catalog/premises do not become acceptance, unknown or incompatible evaluated methods are rejected, and `evidenceKind`
    alone grants nothing.
- id: FR-116-AC-12
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Invalidated/expired owner context is stale; absent observation is unevaluated; a newer failed/error canonical
    observation prevents an older passing act from qualifying while retaining both original outcome and owner freshness
    detail.
- id: FR-116-AC-13
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Failed, errored, skipped, absent, unevaluated, wrong-revision, selection-mismatch, stale, invalid-target,
    ambiguous-target, record-conflict and qualification-rejected causes retain their original target/available locator
    and stable deterministic ordering; applicable causes are not collapsed to one generic gap.
- id: FR-116-AC-14
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Two owner targets with a shared rendered ID are not collapsed; unknown or ambiguous targets cannot qualify,
    and a shared suite/test-case reference does not grant a sibling target a decision without its own observed act.
- id: FR-116-AC-15
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: A genuine healthy audit with no positive owner premises, or a matching FR-093 reliance assessment alone,
    does not qualify an act; the bridge consumes the owner result rather than a precomputed healthy flag or a second
    checker.
- id: FR-116-AC-16
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Accepted retained claims create no source symbols/binders or copied store records; legacy no-evidence
    audit/matrix behavior and the original canonical store formats remain unchanged.
- id: FR-116-AC-17
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: Waiver-shaped input produces no positive qualification and remains distinct under FR-078's pending owner
    policy; no fixture or report claims the final recognized-waiver strict policy settled by this contract.
- id: FR-116-AC-18
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: The result exposes the host-admission and bounded authority basis for every positive decision; an unadmitted
    self-asserted context cannot obtain a positive decision merely by copying the expected field values.
- id: FR-116-prose-4-351
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'For the opt-in retained-evidence path of quire-rs

    [FR-078](ix://agent-ix/quire-rs/FR-078), Quoin SHALL decide whether one requested

    verification act supplies positive evidence for one exact selected obligation

    and its declared method. Inspection, Analysis, an executed gate and an executed

    doctest can qualify without being source-symbol declarations.'
- id: FR-116-prose-5-378
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'This is a **draft owner contract, with implementation planned**. It supplies the

    missing positive qualification premises identified by PLAT-1196; it does not

    claim an implemented API, a published qualification capability, a performed

    verification act or a discharged downstream criterion. PLAT-1162''s apparatus

    intake is a separate concern and does not establish these premises.'
- id: FR-116-prose-6-396
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The contract uses [FR-030](./FR-030-evidence-store.md)''s canonical store,

    [FR-032](./FR-032-evidence-auditor.md)''s existing audit and

    [FR-093](./FR-093-evidence-producer-trust.md)''s bounded reliance decision. An

    accepted reliance decision or `AuditReport.healthy` alone is not a positive

    qualification result. Neither recording a run nor declaring `evidenceKind`

    establishes that an act occurred.'
- id: FR-116-prose-8-416
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The planned qualification boundary is the Quoin-owned check consumed by the

    planned `quoin_quire::evidence::qualify` bridge in FR-078. Its premises below are

    a typed owner input contract, **not new fields asserted to exist in

    `RunRecord`, a new retained-record wire schema, or a second evidence store**.

    The bridge consumes the same owner decision rather than implementing another

    method/provenance checker in Quire.'
- id: FR-116-prose-9-378
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'FR-030''s records currently carry no source commit, and FR-032 no longer compares

    a persisted run commit with HEAD. This contract SHALL preserve that legacy

    record and auditor behavior. Source revision and freshness for this additional

    path SHALL instead be checked using correlated observations supplied at

    qualification time. A store record alone cannot answer those questions.'
- id: FR-116-prose-10-216
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'Waiver authority and the final strict disposition of an owner-recognized waiver

    remain separate policy decisions under FR-078. This contract decides positive

    verification only; it does not decide that pending policy.'
- id: FR-116-prose-12-253
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The Quoin qualification boundary SHALL receive these original owner inputs for

    each requested act. The table specifies semantic members of the planned typed

    input, not an existing JSON envelope or permission to place arbitrary keys in

    `RunEntry.config`.'
- id: FR-116-prose-14-602
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The host owns the authentication and admission of actual CI/manual observations

    and actor authority. The qualifier SHALL distinguish those admitted observations

    from transcribed fields and unobserved premises. It SHALL retain their owner

    references so a consumer can identify the authority and observation on which a

    decision relies. Quoin''s comparison of supplied data does not authenticate a

    person, prove an external CI service truthful, or make an untrusted host

    trustworthy. That residual reliance is explicit and bounded by FR-093; shape

    validation and matching strings alone cannot establish'
- id: FR-116-prose-14-3
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: it.
- id: FR-116-prose-16-523
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'For each exact requested target and method, Quoin SHALL return either a positive

    qualification decision with its original record/entry locators and evaluated

    premises, or a non-qualified result with all applicable causes. A positive

    decision SHALL retain the producer-definition identity, bounded-decision and

    authority references, actor, performed/current source revisions, actual

    tool/configuration context, execution association and observation/freshness

    basis. It is a use-specific decision, not a producer certificate.'
- id: FR-116-prose-17-265
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The owner results SHALL preserve the following distinctions for FR-078''s

    consumer mapping. Cause details name the unanswered or mismatched premise and

    the available target/record/invocation locus. An absent record has no invented

    record locator or producer outcome.'
- id: FR-116-prose-19-416
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL order results by the owner selection''s target order, then

    original record/entry locus and cause category, with stable premise identity

    within a category. It SHALL retain all applicable causes rather than replace a

    failed, stale or unavailable act with one generic unbacked result. Existing

    FR-032 finding precedence remains unchanged; this result also carries the

    additional owner-premise causes.'
- id: FR-116-prose-22-525
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL correlate the producer definition, actor authority,

    performed-act observation, source context, tool/configuration facts and

    execution association with the **same selected canonical record and entry** and

    the **same exact obligation and declared method**. Matching only a suite name,

    timestamp, `evidenceKind`, command string or current HEAD is insufficient. A

    passing unrelated CI job cannot make an older record current. Missing

    correlation is unevaluated; contradictory correlations are record-conflict.'
- id: FR-116-prose-23-384
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL preserve the authored method. The performed act kind and

    the declared verification method are distinct: an Analysis obligation can

    require an executed gate, and a catalog-declared method can require doctests.

    The producer definition specifies the compatible pair. Quoin SHALL NOT relabel

    an Analysis or Inspection obligation as Test to obtain a convenient binding.'
- id: FR-116-prose-24-333
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'When the original audit does not establish healthy evidence, the qualifier

    SHALL retain the audit''s original failure/unevaluated details without granting

    positive qualification. When the catalog or a required owner premise is absent,

    skipping that check in the legacy audit SHALL NOT constitute a positive answer

    on this opt-in path.'
- id: FR-116-prose-25-539
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'An accepted or accepted-with-limitations FR-093 assessment is necessary but not

    sufficient. The qualifier SHALL require a currently observed bounded use that

    permits the requested downstream decision. A limitation that excludes a

    required act or requested target cannot be treated as an accepted exception.

    Unobserved, invalidated and not-accepted reliance cannot produce a positive

    decision. The additional method, source, actor and execution premises in this

    contract remain mandatory even if an FR-093 optional trigger did not ask them.'
- id: FR-116-prose-26-436
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'One performed act may supply evidence for several explicitly examined targets,

    but the qualifier SHALL issue a separate per-target decision with the

    corresponding observation for each. It SHALL NOT extend an accepted decision to

    every Inspection row, a sibling criterion or a shared test-case reference.

    FR-030''s legacy indirect binding rules remain unchanged; they do not by

    themselves establish these positive per-target observations.'
- id: FR-116-prose-29-419
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'For a definition requiring CI, the qualifier SHALL require the actual run,

    attempt, job and invocation association; an arbitrary URL or unrelated green

    job is insufficient. For a definition permitting a manual act, the qualifier

    SHALL require the accountable per-target attestation and the explicit

    not-applicable reason instead of inventing a CI run. A manual reason cannot

    excuse a definition''s required CI execution.'
- id: FR-116-prose-31-466
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL compare the performed revision from the correlated act

    observation with the actual selected source HEAD supplied by the host. It SHALL

    report a mismatch as wrong-revision and a missing/unanswered performed revision

    as unevaluated. Supplying the current HEAD next to an unrelated old record

    cannot satisfy this check. These are read-time premises of this opt-in use, not

    restored commit fields, tool-revision pins or digest requirements in FR-030.'
- id: FR-116-prose-32-386
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL evaluate the producer definition''s current observation and

    owner-required validity conditions using the caller-supplied time and facts.

    It SHALL reject expired or invalidated reliance as stale, preserve a missing

    observation as unevaluated and reject an internally contradictory time/context

    as record-conflict. A recent timestamp alone does not establish freshness.'
- id: FR-116-prose-33-457
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL use the newest selected canonical observations under

    FR-030/FR-032. A newer failed or errored act SHALL prevent reuse of an older

    passing act for that same required evidence; its original outcome and the

    applicable stale cause remain visible. A changed or contradictory record/read

    association cannot be resolved by choosing whichever positive record was

    visited first. Qualification SHALL NOT write or repair the store to obtain a

    pass.'
- id: FR-116-prose-35-504
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'Only Quoin''s evaluated owner result supplies these premises to the planned

    FR-078 bridge; a Quire-local configuration map, source tag or reconstructed

    pass list cannot stand in for it. The bridge''s owner-neutral projection retains

    the original selected target and locators without manufacturing a

    `SymbolGraph` node or source binder. A positive decision supplies no blanket

    strict-coverage pass: the consumer still evaluates inventory completeness,

    other requested acts and its inherited strict premises.'
- id: FR-116-prose-36-448
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'The qualifier SHALL NOT interpret a waiver as a passing verification act.

    Unknown waiver-shaped input remains unevaluated with the pending-policy detail

    specified by FR-078. An eventual owner-recognized waiver remains a separate

    owner-policy outcome; this contract SHALL NOT map it silently to Pass, Skip,

    Fail or absence, or determine its eventual strict exit. Positive qualification

    can be reviewed independently of that unresolved waiver policy.'
- id: FR-116-prose-40-148
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: 'All criteria below describe planned implementation obligations. This draft

    claims no executable acceptance evidence or current downstream discharge.'
- id: FR-116-prose-43-880
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '- **Upstream**: [FR-030](./FR-030-evidence-store.md) for original canonical records and locators; [FR-032](./FR-032-evidence-auditor.md)
    for the existing shared audit; [FR-093](./FR-093-evidence-producer-trust.md) for bounded owner reliance and
    invalidation; the original method catalog and the caller''s admitted CI/manual/source observations.

    - **Downstream**: quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078) / PLAT-1160''s planned bridge and neutral
    backing projection; IR-1006 method-consumer qualification; the affected IR/RT Inspection and Analysis rows.'
- id: FR-116-prose-43-320
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '[FR-115](./FR-115-evidence-backed-test-matrix.md)''s existing join is not itself this opt-in owner qualification.

    - **Separate**: PLAT-1162''s evidence-gate apparatus intake does not supply this contract; the owner-recognized
    waiver representation and final strict policy remain with the policy owner identified by FR-078.'
- id: FR-116-table-47
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Selected target | The owner selection and original criterion identity supplied by FR-078, including
    its source/document/origin identity, exact requested target and declared method. A flat string ID alone is insufficient
    when the selected inventory distinguishes rows with the same rendered ID. |'
- id: FR-116-table-48
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Original audit premises | The original `AuditInput`, canonical `RunRecord` and `Binding` observations,
    method catalog and original record/run/entry locators from the owner read boundary. The bridge invokes the shared
    audit itself as FR-078 specifies; a supplied `healthy` flag or precomputed pass list is insufficient. |'
- id: FR-116-table-49
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Producer definition | A Quoin-owner-approved definition identity/version describing the allowed declared-method/act-kind
    pair, required verification acts, required result semantics, permitted target scope, actual tool/configuration
    premises and required CI or manual association. A suite or command name alone is not this definition. |'
- id: FR-116-table-50
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Bounded reliance | The original FR-093 decision and current observation for the exact intended producer
    use, including accountable owner, permitted downstream decisions, validation evidence and accepted limitations.
    The requested positive use must be permitted, not merely another advisory use of the producer. |'
- id: FR-116-table-51
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Actor authority | The actual performer or accountable attester identity, its owner-authorized scope
    for the requested obligation/method, and the host''s authority-admission reference. A name copied into a record
    does not establish authority. |'
- id: FR-116-table-52
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Performed-act observation | The actual completed act, its original result, performed-at time, material/scope
    examined, required invocation identities and their results, correlated with the selected canonical record and
    entry. A transcribed claim that an act passed is not an observation of that act. |'
- id: FR-116-table-53
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Source context | The repository/selection context, the source revision actually examined or executed,
    and the actual source HEAD selected for this qualification call, supplied by the owner host. A current HEAD
    observation is not a substitute for the performed revision. |'
- id: FR-116-table-54
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Tool and configuration context | The actual producer and exact tool version, adapter identity when
    used, and effective non-secret configuration facts required by the producer definition: configuration reference,
    arguments, feature selections and relevant environment facts. A repeated configuration label does not establish
    equal effective configuration. Manual acts identify the actual inspection/analysis procedure and the recording
    tool/version. |'
- id: FR-116-table-55
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Execution association | Either the exact CI system/project/run/attempt/job and workflow/step or invocation
    association, or an explicit manual association permitted by the producer definition, with an accountable per-target
    attestation and the reason CI is not applicable. |'
- id: FR-116-table-56
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Current observation | The caller-supplied observation time, current owner decision/context and the
    newest selected store observations needed to detect invalidation, supersession, absence or conflicting reads.
    Any owner-required time validity rule is explicit in the producer definition; no built-in age allowance is inferred.
    |'
- id: FR-116-table-57
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Qualified passing act | Every required owner premise is positively evaluated and the original required
    act results passed. |'
- id: FR-116-table-58
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `failed` | A required performed act reported failure. |'
- id: FR-116-table-59
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `errored` | A required act or its producer reported an error. |'
- id: FR-116-table-60
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `skipped` | A required act was skipped; it is not a waiver. |'
- id: FR-116-table-61
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `absent` | A requested retained record or required act result is absent. |'
- id: FR-116-table-62
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `unevaluated` | A required premise or authoritative observation is unavailable or was not evaluated;
    an unknown owner contract is not an accepted default. |'
- id: FR-116-table-63
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `wrong-revision` | The actually performed source revision does not match the actual selected source
    HEAD/context required for this act. |'
- id: FR-116-table-64
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `selection-mismatch` | The audit, record association or owner context belongs to a different selected
    corpus. |'
- id: FR-116-table-65
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `stale` | Owner revalidation/time validity or newer selected evidence invalidates the earlier reliance.
    Original failed/error outcomes remain visible as well. |'
- id: FR-116-table-66
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `invalid-target` / `ambiguous-target` | The requested target is not in the owner selection, or distinct
    owner targets cannot be resolved unambiguously. |'
- id: FR-116-table-67
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `record-conflict` | Required owner observations contradict each other or cannot be correlated to one
    consistent selected record/read. |'
- id: FR-116-table-68
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| `qualification-rejected` | An evaluated premise refuses the method/act pairing, actor authority, bounded
    use, producer definition/version, tool/configuration or required execution association. |'
- id: FR-116-table-69
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Inspection | An actual completed inspection report identifies the exact target, material at the performed
    revision, performer/attester, applicable inspection questions and their per-target passing findings. Manual
    association requires owner-authorized scope and an explicit reason CI is not applicable. A schema-valid inspections
    table or a transcribed `pass` alone is insufficient. |'
- id: FR-116-table-70
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Analysis | An actual completed analysis identifies the target, method/procedure, inputs, result and
    every act required by its producer definition. A gate-shaped Analysis such as IR FR-038-AC-195 requires observations
    of **both** warnings-denied Clippy lanes, their actual arguments/features and results, and their association
    with the required CI `make lint` execution. A one-lane run or a command merely present in a Makefile cannot
    qualify the whole act. A permitted manual analysis uses the same per-target authority/attestation rule as manual
    Inspection. |'
- id: FR-116-table-71
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Executed gate | Actual invocation observations identify the gate procedure at the performed revision,
    each required sub-invocation, effective configuration, exit/result semantics and correlated CI run/attempt/job.
    A zero exit that masks a failed required sub-invocation does not establish a passing gate. |'
- id: FR-116-table-72
  path: spec/functional/FR-116-owner-qualified-quire-evidence.md
  role: examined
  excerpt: '| Executed doctest | Actual harness output identifies the selected documentation example/target, execution
    mode, required cases and passing results under the observed tool/configuration. Discovery, compilation-only/`no-run`,
    ignored or skipped examples do not establish execution. A failed or errored example keeps its original outcome.
    |'
- id: functional-index-FR-116
  path: spec/functional/index.md
  role: examined
  excerpt: "- [FR-116: Positive owner qualification for Quire retained evidence](./FR-116-owner-qualified-quire-evidence.md)\
    \ \u2014 draft contract; implementation planned (PLAT-1196)"
- id: FR-030
  path: spec/functional/FR-030-evidence-store.md
  role: context_only
  excerpt: "`quoin` SHALL persist what was verified, against which statement, by which run \u2014\nin a file store\
    \ under `spec/evidence/` \u2014 and SHALL expose verbs to transcribe a\nrun and collect unreferenced records."
- id: FR-032
  path: spec/functional/FR-032-evidence-auditor.md
  role: context_only
  excerpt: '`quoin` SHALL audit the evidence store against the obligations of the day and

    report the ways evidence has rotted, without running anything.'
- id: FR-093
  path: spec/functional/FR-093-evidence-producer-trust.md
  role: context_only
  excerpt: 'Quoin records why one bounded decision may rely on one evidence-producer use, then makes

    changes to the accepted context visible without awarding the producer a global badge.'
- id: FR-115
  path: spec/functional/FR-115-evidence-backed-test-matrix.md
  role: context_only
  excerpt: '`quoin-core` SHALL expose `matrix.build`, an operation beside

    `assurance.build_case`, that joins the quire-rs static coverage matrix

    ([FR-050](ix://agent-ix/quire-rs/FR-050)-AC-47..51''s `coverage_matrix`) with

    the `quoin-evidence` binding graph ([FR-030](./FR-030-evidence-store.md)) and

    the `quoin-auditor` `AuditReport` ([FR-032](./FR-032-evidence-auditor.md)) to

    answer the question the static matrix cannot: was a criterion discharged by a

    **passing run**, or only tagged?'
- id: quire-rs/FR-078
  path: /home/peter/dev/worktrees/IR-1006/spec/functional/FR-078-qualified-evidence-binding.md
  role: context_only
  excerpt: 'When a caller supplies retained verification evidence, Quire shall qualify its

    binding to the selected declaration before granting coverage credit. A performed

    Inspection, Analysis, shell or Makefile gate, or Rust doctest can carry evidence

    without being a source-symbol declaration. A citation, method name, suite

    registration, or schema-valid Inspections table is not a passing execution or a

    performed inspection.'
findings: []
bindings: []
```
