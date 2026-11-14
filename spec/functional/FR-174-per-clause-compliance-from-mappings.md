---
id: FR-174
title: "quoin discharge --rmm reports per-clause compliance for facts recorded from mappings"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-173"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-135"
    type: "depends_on"
---

# FR-174: quoin discharge --rmm reports per-clause compliance for facts recorded from mappings

## Description

The path from a typed control to a compliance row is `facts-from-mappings` (FR-173), then `record-discharge` (FR-133), then
`quoin discharge --repo --rmm` (FR-135). This requirement fixes the end-to-end result and the stale-fact case. FR-135's rules are
unchanged.

## Rationale

The store keeps a fact until it is replaced or expires, and a fact for a clause is replaced only when a new fact for that clause
is recorded. A control reassessed as not effective emits no new fact, so its earlier fact stays. The report says what is
stored, and FR-173 names the stale fact so a person can withdraw it.

## Behavior

- The report SHALL show a clause answered by an `effective` control and attested as `Compliant`, with the control ids as its Evidence.
- The report SHALL show a clause with a `scope_exclusion` and a recorded `delegated`, `accepted_risk` or `temporary_exception` disposition under that exact disposition, never `Compliant`.
- The report SHALL treat a recorded `temporary_exception` as current only while `asOf` is earlier than its persisted `decisionExpiresAt` and its attestation is current.
- At or after `decisionExpiresAt`, the report SHALL show the clause as `Not compliant - temporary exception expired`, even when the attestation remains current.
- The report SHALL show a clause whose controls are not `effective` and that has no stored fact as `Not compliant - undischarged`.
- Re-running the chain SHALL replace the stored fact of each clause it emits (FR-133).
- A stored `direct` fact SHALL stay in force, and its clause `Compliant`, until its attestation expires or its file is removed from the store, even after the control is reassessed.
- A person withdraws a fact by removing its file from the evidence store (FR-133's path), which the `stale` line of FR-173 names.
- The chain SHALL carry no clause text and no score.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-174-AC-1 | Over a synthetic set with binding clauses recorded under each supported disposition and one answered by an effective control, `quoin discharge --repo --rmm` shows the three exact dispositions and `Compliant` with the control id respectively; no disposition is rendered as `Compliant`. | Test |
| FR-174-AC-2 | After the expiry instant given as `--as-of`, the first clause renders `Not compliant - attestation not current`. | Test |
| FR-174-AC-3 | The rendered matrix and the chain's intermediate files contain none of the set's clause-text sentinel and no percentage or count of compliant rows. | Test |
| FR-174-AC-4 | The same chain against a synthetic pack installed in a temp `IX_HOME` renders the same states by clause id. | Test |
| FR-174-AC-5 | After the control of a recorded `Compliant` clause is changed to `ineffective` and the chain is re-run, the clause still renders `Compliant` and FR-173 reports `stale` for it; after its fact file is removed it renders `Not compliant - undischarged`. | Test |
| FR-174-AC-6 | For a stored `temporary_exception` with `decisionExpiresAt` earlier than `attestation.expiresAt`, `--as-of` immediately before `decisionExpiresAt` renders that disposition; at `decisionExpiresAt` and after it renders `Not compliant - temporary exception expired`. | Test (TC-004) |

## Dependencies

- **Upstream**: [FR-173](./FR-173-facts-from-mappings.md), [FR-135](./FR-135-requirements-mapping-matrix-view.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md)
- **Downstream**: none
