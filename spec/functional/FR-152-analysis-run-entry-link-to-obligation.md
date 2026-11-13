---
id: FR-152
title: "An analysis run entry binds to an obligation only through the obligation ids the recorder states"
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
  - target: "ix://agent-ix/quoin/FR-041"
    type: "references"
  - target: "ix://agent-ix/quoin/ADR-100"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-078"
    type: "references"
---

# FR-152: An analysis run entry binds to an obligation only through the obligation ids the recorder states

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

A `differential-report` entry carries no trace id, so the join above never fires and the entry stays unbound.
FR-033 and ADR-100 forbid the remedy a matcher would offer: a component that infers a binding from names sits inside
the evidence chain and can change what a result is bound to without anyone noticing.

The binding source already exists. `quoin evidence record --discharges <obligation-id>` (FR-041) is the recorder
stating which obligation a run discharges: each stated id that an obligation states is added to every entry as a
trace id, the entry then binds through the same `record_run` join, only a passing entry binds, and an id no obligation
states is reported `unmatched`. FR-041 refuses `--discharges` on every run adapter except the sbom adapter and
finding-shaped recordings, so `differential-report` is refused today. This requirement admits `differential-report`
to `--discharges` and adds nothing else: no new option, no new field, no new join.

What `--discharges` does, and this requirement relies on: the stated ids are trimmed, sorted and deduplicated, so
`traceIds` are in sorted order and not in the order given; only ids of obligations that exist are attached, so a
declared target id is not accepted and an unknown id is reported `unmatched` and not stored on the entry. What is
stored is the run record with the entry's `traceIds` and the binding from the obligation to the suite; nothing else.

The stated id is an assertion by the recorder that this analysis request is verification evidence for that obligation.
Quoin does not judge it here; the actor-authority premise of Quoin FR-116 evaluates the asserting actor's authority
for the obligation at qualification time.

### Which repository implements each step

| Step | Implemented in |
| --- | --- |
| Adapter output: one entry or one `unrepresented` result per report | quoin (FR-151) |
| Attach the stated obligation ids to the entry and bind them by the existing join (`--discharges`, FR-041) | quoin (this requirement admits the adapter) |
| Retained evidence record: run record and binding in the store | quoin (FR-030, unchanged) |
| Qualification of the bound act | quoin (FR-116 qualifier and `quoin-quire` bridge; FR-153) |
| Coverage reconciliation, `qualified-record` status, trace and strict decision | quire-rs (FR-078, unchanged) |

quire-rs needs no change for analysis results: an analysis obligation is an ordinary selected obligation whose
declared method is Analysis, and FR-078 already counts it by exact selected target.

## Behavior

- `quoin evidence record` SHALL accept `--discharges` with `--adapter differential-report`, with the meaning FR-041
  gives it, cited and not restated.
- When `--discharges` is supplied with an accepted `differential-report` report that yields an entry, `quoin evidence
  record` SHALL add the stated obligation ids that exist to the entry's `traceIds` and bind them through the existing
  join of FR-030, changing neither the join nor the binding record.
- If `--discharges` is not supplied with a `differential-report` report, then `quoin evidence record` SHALL store the
  entry with no `traceIds`.
- If `--discharges` is not supplied with a `differential-report` report, then `quoin evidence record` SHALL add no
  binding.
- If a stated id is not the id of any obligation, then `quoin evidence record` SHALL report it among the unmatched
  ids and SHALL NOT store it on the entry.
- If the report is a `disagreement`, then `quoin evidence record` SHALL write the zero-entry run of FR-151, add no
  binding and print the `unrepresented` result of FR-151.
- If a report is refused by the adapter, then `quoin evidence record` SHALL write no run and no binding, whatever ids
  were stated.
- The adapter SHALL continue to leave `traceIds` unset; the stated ids are attached by the command, after the adapter
  has returned.
- Quoin SHALL NOT derive an obligation id from the symbol, the `analysisKind`, the `requestDigest`, the report's
  content, a title, a path or a statement.
- Quoin SHALL NOT bind an entry whose outcome is not `pass`.
- Quoin SHALL NOT create or accept a binding option other than `--discharges` for an analysis entry.

### Declared method of a discharged obligation

`quoin evidence record` has no qualification premise and, like FR-041 for an inventory, does not check the declared
method of the obligation a recorder states. The legacy `quoin evidence audit` cannot enforce it either: its
`method_conformance` compares the run's `evidenceKind` with the catalog and says nothing when the run declares none,
and a recorder can set `--kind` to anything. Whether a bound analysis entry is allowed to back an obligation whose
declared method is not Analysis is therefore decided only at qualification: the planned bridge (FR-153) SHALL report
such an obligation as `qualification-rejected`, naming the method and act pairing premise of FR-116, and SHALL NOT
count it.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-152-CON-1 | Stating obligations SHALL add no record family, no field to the run record and no store file; it travels as the entry's existing `traceIds`. | Compatibility | Test |
| FR-152-CON-2 | Recording SHALL stay a transcription that runs no solver, reads no file other than its inputs and opens no network connection (FR-069-CON-1). | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-152-AC-1 | An `agreement`/`satisfied` report recorded with `--discharges FR-001-AC-1`, where that obligation exists, stores one entry whose `traceIds` are exactly `["FR-001-AC-1"]` and writes one binding from that obligation to the suite whose symbols are exactly `<analysisKind>::<requestDigest>`; the outcome reports the obligation as bound. | Test |
| FR-152-AC-2 | The same report recorded without `--discharges` stores one entry with no `traceIds` and adds no binding entry for any obligation; `quoin evidence audit` reports the obligation as undischarged. | Test |
| FR-152-AC-3 | A valid report recorded without `--discharges`, in a fixture whose obligation ids differ from the symbol, the `analysisKind` and the `requestDigest`, adds no binding. The report is not edited: it is a report the validator accepts. | Test |
| FR-152-AC-4 | An `agreement`/`refuted` report and an `inconclusive` report, each recorded with `--discharges FR-001-AC-1`, store an entry whose `traceIds` are the stated id and add no binding; an `unavailable` report does the same. | Test |
| FR-152-AC-5 | A `disagreement` report recorded with `--discharges FR-001-AC-1` writes a run with zero entries, adds no binding and prints the `unrepresented` result in human and JSON output. | Test |
| FR-152-AC-6 | `--discharges` given twice with two obligation ids stores `traceIds` sorted and deduplicated and binds both obligations to the same symbol; an id naming no obligation is listed among the unmatched ids, is not stored on the entry and binds nothing, while a valid id given beside it still binds. | Test |
| FR-152-AC-7 | A refused report (wrong schema, non-canonical form, oversize) recorded with `--discharges` writes no run and no binding and exits non-zero as FR-151 states. | Test |
| FR-152-AC-8 | `--discharges` with `--adapter junit` is still refused before any run is written, naming the adapter, as FR-041 states. | Test |

## Dependencies

- **Upstream**: [FR-151](./FR-151-differential-report-v2-adapter.md) (the entry and the `unrepresented` result),
  [FR-030](./FR-030-evidence-store.md) (store and binding graph), [FR-033](./FR-033-evidence-format-adapters.md),
  [FR-041](./FR-041-sbom-inventory-evidence.md) (`--discharges`),
  [ADR-100](../adrs/ADR-100-execution-evidence-is-emitted-by-the-run.md).
- **Downstream**: [FR-153](./FR-153-analysis-result-counting-by-disposition.md) counts the bound entry.
- **Cross-repository**: quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078), unchanged.
