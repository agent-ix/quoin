---
id: FR-175
title: "quoin anomaly check reports a fixed anomaly whose verifying test has a failing or missing result"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/spec-artifacts-process/US-003"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-120"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-015"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/NFR-035"
    type: "references"
---

# FR-175: quoin anomaly check reports a fixed anomaly whose verifying test has a failing or missing result

## Description

Tracked by Linear PLAT-1583. An `Anomaly` resolved or closed as `fixed` carries a `## Fix Verification` table
(`agent-ix/spec-artifacts-process` FR-015) whose rows name, by `Kind` `Criterion` or `Test`, what demonstrates the fix.
`quire validate` checks that the table exists and that each `Target` resolves to the right kind of artifact. It cannot
say whether the named test ever ran and passed. `quoin anomaly check` (FR-136) reads the evidence store, which records
the results of runs (FR-030), and reports a fixed anomaly whose named test has a failing or missing result there.

This requirement adds no evidence mechanism. A result is a run entry of the evidence store, carrying the trace ids its
run emitted (FR-120). The check states what the store holds against what the anomaly recorded; it does not accept the
fix (NFR-035).

> **CR note (PLAT-1583):** the record reader of FR-136 (`quoin-anomaly`, `record.rs`) reads no `## Fix Verification`
> rows today, and the reverse-reference resolution (`refs.rs`) does not index `TestMatrix` test ids. This requirement
> widens both: the reader gains the verification rows (`Kind`, `Target`, `Rationale`), and the resolution gains the test
> case ids of the corpus. A verification table with a wrong header, a wrong row arity or no row is a mapping error under
> the existing FR-136 rule, so the document yields `anomaly.no-record`; an out-of-vocabulary `Kind` is `quire validate`'s
> and still yields the record.

## Behavior

### What is evaluated

- The check SHALL evaluate each anomaly whose record has `status` `resolved` or `closed` and `resolution` `fixed`, and no other.
- The check SHALL evaluate the rows of its `## Fix Verification` table in document order.
- The check SHALL skip an anomaly that has no record (`anomaly.no-record`, FR-136).
- The check SHALL skip a row whose `Target` resolves to nothing in the corpus.
- Quoin SHALL NOT report a dangling reference again, because `quire validate` reports it.
- An anomaly whose only rows are skipped rows SHALL yield no finding from this check.
- The check SHALL evaluate every `Test` row.
- The check SHALL evaluate a `Criterion` row whose `Target` is an acceptance-criterion id.
- The check SHALL NOT evaluate a `Criterion` row whose `Target` names an `FR`, `NFR` or hazard document, which FR-015 allows, because no run entry carries a document id as a result.
- The check SHALL read the evidence store under `spec/evidence/` of the scope and SHALL write nothing to it or to any document.

### What counts as a result

The result of a row is derived from the newest run of each suite in the store (FR-030, newest by timestamp), over the
entries that match the row's `Target`.

- An entry matches a `Test` row if its trace ids include the `Target`.
- An entry matches a `Criterion` row if its trace ids include the `Target`, or include the id of a test case that declares that criterion, as FR-030-AC-15 defines.
- An entry's outcome is its recorded `pass`, `fail`, `error` or `skip` (FR-120).
- The result of a row SHALL be `failing` if any matching entry in any suite's newest run has outcome `fail` or `error`.
- The result of a row SHALL be `passing` if it is not `failing` and any matching entry has outcome `pass`.
- The result of a row SHALL be `missing` otherwise: no matching entry, only `skip` entries, or no store (an absent store reads as empty, FR-030).
- If a suite's newest run file is unreadable, then that suite SHALL contribute no entry, and an older run of it SHALL NOT stand in; FR-030-AC-13 still reports the file.
- An older run, a binding in `bindings.json` and a trace tag in source SHALL NOT make a result `passing`.

### Findings

The findings are diagnostics of `quoin anomaly check`, in the format of FR-136, at severity `warning` (a new check ships
advisory-first). FR-136 sets a non-zero exit only for severity `error`, so these findings leave the exit status as it
would be without them.

| Code | Severity | Reason | Condition |
|---|---|---|---|
| `anomaly.fix-result-failing` | warning | `failing-result` | An evaluated row of either `Kind` has a `failing` result. |
| `anomaly.fix-test-missing-result` | warning | `missing-result` | A `Test` row has a `missing` result. |

- The check SHALL emit `anomaly.fix-result-failing` for each evaluated row whose result is `failing`, naming the `Target`.
- The check SHALL emit `anomaly.fix-test-missing-result` for each `Test` row whose result is `missing`, naming the `Target`.
- The check SHALL emit no finding for a `Criterion` row whose result is `missing` or `passing`, because the ticket requires a linked test and not a passing criterion.
- A fixed anomaly with only `Criterion` rows SHALL yield no finding unless a matched result is `failing`.
- A row whose result is `passing` SHALL yield no finding.
- Each finding SHALL carry the 1-based line of the document's `status` field, as every FR-136 diagnostic does.
- Each finding SHALL name the `Target` it concerns.
- The check SHALL NOT state that a fix is verified, accepted or acceptable.
- The check SHALL NOT judge whether a test is adequate for the fix.
- Two runs over the same working tree and store SHALL give byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-175-AC-1 | With the fixture `valid/closed-fixed-test-only.md` (`Test` row `TC-001`) and a store whose newest run holds an entry with trace id `TC-001` and outcome `pass`, `quoin anomaly check` emits neither code. | Test |
| FR-175-AC-2 | With the same fixture and a newest run whose `TC-001` entry has outcome `fail` (or `error`), the check emits `anomaly.fix-result-failing` naming `TC-001`, at severity `warning`, and the exit status is the one it would be without the finding. | Test |
| FR-175-AC-3 | With the same fixture and a store holding no entry that carries `TC-001`, only a `skip` entry for it, or no store at all, the check emits `anomaly.fix-test-missing-result` naming `TC-001`. | Test |
| FR-175-AC-4 | With an older run that passes `TC-001` and a newest run of that suite that fails it, the result is `failing`; with an older failing run and a newest passing run, it is `passing`; a `bindings.json` binding for `TC-001` with no run entry does not make it `passing`. | Test |
| FR-175-AC-5 | With two suites, one whose newest run passes `TC-001` and one whose newest run fails it, the result is `failing`. | Test |
| FR-175-AC-6 | With a suite whose newest run file is unreadable and whose older run passes `TC-001`, the result is `missing`, and the older run does not stand in. | Test |
| FR-175-AC-7 | A fixture whose table holds only `Criterion` rows (`valid/closed-fixed-criterion-only.md`) yields no finding when the criterion id has no entry, no finding when it has a `pass` entry, and `anomaly.fix-result-failing` naming the criterion id when it has a `fail` entry. | Test |
| FR-175-AC-8 | For a `Criterion` row targeting `FR-001-AC-1`, an entry carrying the test case `TC-001` that declares `FR-001-AC-1` (corpus `tests.md`) is matched as FR-030-AC-15 defines: a `fail` entry for `TC-001` makes the result `failing`, a `pass` entry makes it `passing`. | Test |
| FR-175-AC-9 | With a `Criterion` row and a `Test` row (`valid/closed-fixed.md`), a `passing` `Test` row with a `missing` or `passing` `Criterion` row yields no finding; a `failing` `Criterion` row yields `anomaly.fix-result-failing` naming the criterion id; a `missing` `Test` row with a `passing` `Criterion` row still yields `anomaly.fix-test-missing-result`. | Test |
| FR-175-AC-10 | A `Criterion` row whose `Target` names an `FR`, `NFR` or hazard document is not evaluated and yields no finding, whatever the store holds. | Test |
| FR-175-AC-11 | A `resolved` anomaly with `resolution` `fixed` (`valid/resolved-fixed.md`) is evaluated like a `closed` one; anomalies with `resolution` `not-a-defect`, `duplicate` or `wont-fix`, and an anomaly at `assessed`, yield neither code. | Test |
| FR-175-AC-12 | An anomaly whose only row is a `Test` row with a dangling target (`invalid/verification-dangling-test.md`) yields neither code and no dangling-reference diagnostic from `quoin anomaly check`; a document with a verification table that has a wrong header or no row yields `anomaly.no-record` and neither code. | Test |
| FR-175-AC-13 | Every finding of AC-2 and AC-3 carries its code, severity `warning`, its reason, the line of the document's `status` field and its `Target`. | Test |
| FR-175-AC-14 | Every file of the scope and the store has identical bytes before and after a run, two runs give identical output, and the output contains no sentence stating that a fix is verified, accepted or acceptable. | Test |
| FR-175-AC-15 | The anomaly documents and corpus are those shipped by `agent-ix/spec-artifacts-process`, read in place from its sibling checkout. Because the store is read from the scope's own `spec/evidence/`, each test builds a temporary scope at test time that holds the fixture documents and the store, which is never committed; the quoin repository contains no copy of either. | Inspection |

## Dependencies

- **Upstream**: [FR-136](./FR-136-anomaly-record-mapping.md) (widened by the CR note above), [FR-030](./FR-030-evidence-store.md), [FR-120](./FR-120-trace-marks-in-test-result-output.md); `agent-ix/spec-artifacts-process` [FR-015](ix://agent-ix/spec-artifacts-process/FR-015).
- **Constrained by**: [NFR-035](../non-functional/NFR-035-anomaly-report-states-and-never-decides.md): the check states and does not accept.
