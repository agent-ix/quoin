---
id: SR-211
title: "Failure-domain review of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; PR #924 @ ab633975: FR-188, FR-190, FR-192; context rust/crates/quoin-assurance/src/discharge/rmm.rs, quire-rs FR-086"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-192"
    type: references
  - target: "ix://agent-ix/quoin/FR-188"
    type: references
  - target: "ix://agent-ix/quoin/FR-190"
    type: references
---

## Summary

Ticket: PLAT-1436. Looked for unstated failure modes, identity confusion,
partial-state outcomes and edge cases in the command and document rules.

## Verdict

FAIL. FND-001 would produce wrong gaps on ordinary RMM input. The rest are
unstated failure paths the implementation tickets would otherwise have to
decide on their own.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `clause_open` fires when "a row has no compliance or evidence". `compliance` is a required enum on every `RmmRow`, so "no compliance" never holds. That leaves "no evidence", which is true by construction for every `tailored` and `not_applicable` row: their `evidenceRefs` are empty (see the rmm.rs tests). Compliant-by-disposition and not-applicable clauses would be reported open, and which compliance values count as open (`undischarged`, `attestation_not_current`, `temporary_exception_expired`, `undetermined`) is unstated. | spec/functional/FR-192-dossier-rmm-content.md:42; spec/functional/FR-192-dossier-rmm-content.md:56; rust/crates/quoin-assurance/src/discharge/rmm.rs:101; rust/crates/quoin-assurance/src/discharge/rmm.rs:457 |
| FND-002 | medium | FR-190 maps the FR-086 `uncovered` state and `dangling` references to gaps, but says nothing about FR-086's `kind_mismatch` entries (a wrong-kind design element under an `outgoing` or `either` relation). Under FR-188's "never silently dropped" rule they have no gap kind and no rendering rule. | spec/functional/FR-190-dossier-architecture-content.md:37; spec/functional/FR-190-dossier-architecture-content.md:38; spec/functional/FR-188-dossier-command.md:62 |
| FND-003 | medium | When one source fails (exit 3), FR-188 does not say whether `generate` writes the other selected documents, stops before writing, or leaves earlier files in place. FR-188-AC-9 checks only the exit code. A partial run can leave a mix of fresh and stale dossier files that a person then commits. | spec/functional/FR-188-dossier-command.md:71; spec/functional/FR-188-dossier-command.md:86 |
| FND-004 | medium | `generate` accepts `--out <DIR>`, but `export` has no `--out`. Export looks for `<DIR>/<document-id>.md` with no way to name `<DIR>`, so a dossier generated with `--out other` cannot be exported. | spec/functional/FR-188-dossier-command.md:24; spec/functional/FR-188-dossier-command.md:26; spec/functional/FR-188-dossier-command.md:30 |
| FND-005 | medium | `export` checks only that files are committed, not that they still match the records at HEAD. A dossier committed before a later requirement change exports without notice. That defeats US-024's goal that the reviewer reads "documents that match the repository at one commit". A `check`-before-export rule, or an explicit statement that staleness is accepted, is missing. | spec/functional/FR-188-dossier-command.md:40; spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md:16 |
| FND-006 | low | `--as-of` is "required whenever a binding or pack is given", but no exit status and no AC covers a binding or pack given without `--as-of`. | spec/functional/FR-192-dossier-rmm-content.md:26 |
