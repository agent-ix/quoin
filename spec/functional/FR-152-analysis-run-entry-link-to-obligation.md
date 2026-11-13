---
id: FR-152
title: "An analysis run entry binds to an obligation only through an explicit link"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-151"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "references"
  - target: "ix://agent-ix/quoin/ADR-100"
    type: "references"
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-078"
    type: "references"
---

# FR-152: An analysis run entry binds to an obligation only through an explicit link

## Description

The `differential-report` adapter (FR-151) transcribes one `quire.differential-report/v2` report into one run entry
with the symbol `<analysisKind>::<requestDigest>`, no `traceIds` and no `evidenceKind`. A report names no obligation
or criterion, so the entry binds nothing by itself. FR-151 leaves "retained-evidence counting over these entries" to
a separate specification; this requirement and FR-153 are that specification.

### What already exists, referenced and not restated

- The store, its run record and its binding graph are FR-030. `record_run` writes the run, then binds an
  obligation to the suite only through a trace id on a **passing** entry, where the id is an obligation id or a
  target id the obligation's method cell declares (FR-033, quire-rs FR-053). A failing, erroring or skipped entry
  binds nothing. A newer `(obligation, suite)` binding replaces the symbols of the older one.
- Counting a retained record toward coverage and trace is quire-rs FR-078 with the owner qualification contract of
  Quoin FR-116. The engine side of FR-078 (the owner-neutral backing input, coverage reconciliation, trace and the
  strict decision) is implemented in quire-rs. The Quoin qualifier and the `quoin-quire` bridge are not.

### The gap this requirement closes

Nothing in the existing chain supplies a trace id for an analysis entry, so the join above never fires and every
analysis entry stays unbound. FR-033 and ADR-100 forbid the remedy a matcher would offer: a component that infers a
binding from names sits inside the evidence chain and can change what a result is bound to without anyone noticing.

The binding source is therefore an **explicit link stated by the person or job that records the run**: one or more
obligation or target ids given to `quoin evidence record`. The link is an assertion that this analysis request is
verification evidence for that obligation. Quoin does not judge that assertion; the actor-authority premise of Quoin
FR-116 is where the asserting actor's authority for the obligation is evaluated, at qualification time.

### Which repository implements each step

| Step | Implemented in |
| --- | --- |
| Adapter output: one entry or one `unrepresented` result per report | quoin (FR-151) |
| Attach the stated link to the entry and bind it by the existing join | quoin (this requirement) |
| Retained evidence record: run record and binding in the store | quoin (FR-030, unchanged) |
| Qualification of the bound act | quoin (FR-116 qualifier and `quoin-quire` bridge; FR-153) |
| Coverage reconciliation, `qualified-record` status, trace and strict decision | quire-rs (FR-078, unchanged) |

quire-rs needs no change for analysis results: an analysis obligation is an ordinary selected obligation whose
declared method is Analysis, and FR-078 already counts it by exact selected target.

## Behavior

- `quoin evidence record` SHALL accept zero or more explicit links, each an obligation id or a declared target id, as
  a repeatable `--link <id>` option.
- When a link is supplied with an accepted `differential-report` report that yields an entry, `quoin evidence
  record` SHALL set the entry's `traceIds` to the supplied ids, verbatim and in supplied order, in the stored run
  record.
- When a link is supplied, `quoin evidence record` SHALL bind the entry through the existing join of FR-030 and
  change neither the join nor the binding record.
- If no link is supplied, then `quoin evidence record` SHALL store the entry with no `traceIds`.
- If no link is supplied, then `quoin evidence record` SHALL create no binding.
- If a link names an id that is neither an obligation id nor a declared target id of any obligation, then `quoin
  evidence record` SHALL report it among the unmatched ids.
- If a link names such an id, then `quoin evidence record` SHALL create no binding for it.
- If the report is a `disagreement`, then `quoin evidence record` SHALL write the zero-entry run of FR-151, create no
  binding, and report each supplied link as unbound with the state `disagreement`.
- If a report is refused by the adapter, then `quoin evidence record` SHALL write no run and no binding, whatever
  links were supplied.
- The adapter SHALL continue to leave `traceIds` unset; the link is attached by the command, after the adapter has
  returned.
- Quoin SHALL NOT derive a link from the symbol, the `analysisKind`, the `requestDigest`, the report's content, a
  title, a path or a statement.
- Quoin SHALL NOT bind an entry whose outcome is not `pass`.
- `quoin evidence record` SHALL NOT accept `--link` for any adapter other than `differential-report`.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-152-CON-1 | The link SHALL add no record family, no field to the run record and no store file; it travels as the entry's existing `traceIds`. | Compatibility | Test |
| FR-152-CON-2 | Recording SHALL stay a transcription that runs no solver, reads no file other than its inputs and opens no network connection (FR-069-CON-1). | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-152-AC-1 | An `agreement`/`satisfied` report recorded with `--link FR-001-AC-1`, where that obligation exists and declares Analysis, stores one entry whose `traceIds` are exactly `["FR-001-AC-1"]` and writes one binding from that obligation to the suite whose symbols are exactly `<analysisKind>::<requestDigest>`; the outcome reports the obligation as bound. | Test |
| FR-152-AC-2 | The same report recorded with no `--link` stores one entry with no `traceIds` and leaves the binding graph byte-identical to before the run; `quoin evidence audit` reports the obligation as undischarged. | Test |
| FR-152-AC-3 | A report whose `analysisKind` and `requestDigest` are chosen so that `<analysisKind>::<requestDigest>`, `analysisKind` or `requestDigest` equals an existing obligation id or declared target id, recorded with no `--link`, creates no binding. | Test |
| FR-152-AC-4 | An `agreement`/`refuted` report and an `inconclusive` report, each recorded with `--link FR-001-AC-1`, store an entry whose `traceIds` are the linked id and create no binding; an `unavailable` report does the same. | Test |
| FR-152-AC-5 | A `disagreement` report recorded with `--link FR-001-AC-1` writes a run with zero entries, creates no binding, and prints the link as unbound with state `disagreement` in human and JSON output. | Test |
| FR-152-AC-6 | `--link` given twice with two obligation ids stores `traceIds` in the supplied order and binds both obligations to the same symbol; a link naming no obligation or declared target is listed among the unmatched ids and binds nothing, while a valid link given beside it still binds. | Test |
| FR-152-AC-7 | A refused report (wrong schema, non-canonical form, oversize) recorded with `--link` writes no run and no binding and exits non-zero as FR-151 states. | Test |
| FR-152-AC-8 | `--link` with `--adapter junit` is refused before any run is written, naming the adapter. | Test |
| FR-152-AC-9 | `git diff` of the change that implements this requirement shows no edit to the `RunRecord`, `RunEntry` or `Binding` types, or to `record_run`'s join; the stored run record of AC-1 has no member the store did not already define. | Inspection |

## Dependencies

- **Upstream**: [FR-151](./FR-151-differential-report-v2-adapter.md) (the entry and the `unrepresented` result),
  [FR-030](./FR-030-evidence-store.md) (store and binding graph), [FR-033](./FR-033-evidence-format-adapters.md),
  [ADR-100](../adrs/ADR-100-execution-evidence-is-emitted-by-the-run.md).
- **Downstream**: [FR-153](./FR-153-analysis-result-counting-by-disposition.md) counts the bound entry.
- **Cross-repository**: quire-rs [FR-078](ix://agent-ix/quire-rs/spec/functional/FR-078), unchanged.
