---
id: FR-175
title: "quoin anomaly check reports a fixed anomaly whose verifying test has no passing result"
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

# FR-175: quoin anomaly check reports a fixed anomaly whose verifying test has no passing result

## Description

Tracked by Linear PLAT-1583. An `Anomaly` resolved or closed as `fixed` carries a `## Fix Verification` table
(`agent-ix/spec-artifacts-process` FR-015) whose rows name, by `Kind` `Criterion` or `Test`, what demonstrates the fix.
`quire validate` checks that the table exists and that each `Target` resolves to the right kind of artifact. It cannot
say whether the named test ever ran and passed. `quoin anomaly check` (FR-136) reads the evidence store, which records
the results of runs (FR-030), and reports the fixed anomalies whose named test has no passing result there.

This requirement adds no evidence mechanism. A result is a run entry of the evidence store, carrying the trace ids its
run emitted (FR-120), and a test case is matched by its `TestMatrix` `Test ID` or an acceptance-criterion id as that
entry carries it (FR-030, "A tool reports the id it knows"). The check states what the store holds against what the
anomaly recorded; it does not accept the fix (NFR-035).

## Behavior

### What is evaluated

- The check SHALL evaluate each anomaly whose record has `status` `resolved` or `closed` and `resolution` `fixed`,
  and no other.
- The check SHALL evaluate the rows of its `## Fix Verification` table in document order.
- The check SHALL skip an anomaly that has no record (`anomaly.no-record`, FR-136) and SHALL skip a row whose `Target`
  resolves to nothing in the corpus, because `quire validate` reports a dangling reference and Quoin SHALL NOT report it
  again.
- The check SHALL read the evidence store under `spec/evidence/` of the scope's repository and SHALL write nothing to it
  or to any document.

### What counts as a result

The result of a row is derived from the newest run of each suite in the store (FR-030, newest by timestamp), over
the entries whose trace ids include the row's `Target`.

- An entry's outcome is its recorded `pass`, `fail`, `error` or `skip` (FR-120).
- The result of a `Target` SHALL be `failing` if any such entry in any suite's newest run has outcome `fail` or `error`.
- The result of a `Target` SHALL be `passing` if it is not `failing` and any such entry has outcome `pass`.
- The result of a `Target` SHALL be `missing` otherwise: no entry carries the id, the entries are all `skip`, or the
  store is absent (an absent store reads as empty, FR-030).
- An older run, a binding in `bindings.json` and a trace tag in source SHALL NOT make a result `passing`.

### Findings

The findings are diagnostics of `quoin anomaly check`, in the format and with the severity rules of FR-136.

| Code | Severity | Reason | Condition |
|---|---|---|---|
| `anomaly.fix-test-failing` | error | `failing-result` | A row of either `Kind` has a `failing` result. |
| `anomaly.fix-test-missing-result` | error | `missing-result` | A `Test` row has a `missing` result. |
| `anomaly.fix-no-passing-link` | error | `no-passing-link` | The table has no `Test` row and no `Criterion` row with a `passing` result. |

- The check SHALL emit `anomaly.fix-test-failing` for each row whose result is `failing`, naming the `Target`.
- The check SHALL emit `anomaly.fix-test-missing-result` for each `Test` row whose result is `missing`, naming the
  `Target`.
- The check SHALL NOT emit a finding for a `Criterion` row whose result is `missing` while a `Test` row exists or another
  `Criterion` row is `passing`: a criterion may be demonstrated by inspection and its rationale says so.
- If the table has no `Test` row and no `Criterion` row is `passing`, then the check SHALL emit one
  `anomaly.fix-no-passing-link` for the document. A `Criterion` row with a `passing` result is thus accepted as the
  link, because a unit test carries its criterion's id (FR-030).
- A row whose result is `passing` SHALL yield no finding, and a document in which every `Test` row is `passing` SHALL
  yield none of the three.
- Each finding SHALL carry the 1-based line of the document's `status` field, as every FR-136 diagnostic does, and the
  `Target` it concerns where it has one.
- The exit status SHALL be non-zero if any of these findings is present, as FR-136 sets for severity `error`.
- The check SHALL NOT state that a fix is verified, accepted or acceptable, and SHALL NOT judge whether a test is
  adequate for the fix.
- Two runs over the same working tree and store SHALL give byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-175-AC-1 | Over the corpus of `agent-ix/spec-artifacts-process` `tests/fixtures/anomaly/corpus`, the fixture `valid/closed-fixed-test-only.md` (`Test` row `TC-001`) and a store in the test's temporary directory whose newest run holds an entry with trace id `TC-001` and outcome `pass`, `quoin anomaly check` emits none of the three codes and exits zero. | Test |
| FR-175-AC-2 | With the same fixture and a newest run whose `TC-001` entry has outcome `fail`, the check emits `anomaly.fix-test-failing` naming `TC-001` and exits non-zero; an outcome of `error` gives the same. | Test |
| FR-175-AC-3 | With the same fixture and a store holding no entry that carries `TC-001`, only a `skip` entry for it, or no store at all, the check emits `anomaly.fix-test-missing-result` naming `TC-001` and exits non-zero. | Test |
| FR-175-AC-4 | With a store whose older run passes `TC-001` and whose newest run of that suite fails it, the result is `failing`; with an older failing run and a newest passing run, the result is `passing`; a `bindings.json` binding for `TC-001` with no entry in a run does not make it `passing`. | Test |
| FR-175-AC-5 | A fixture whose `## Fix Verification` holds only `Criterion` rows (`valid/closed-fixed-criterion-only.md`) and a store with no entry for the criterion id emits one `anomaly.fix-no-passing-link` and no `anomaly.fix-test-missing-result`, and exits non-zero; the same fixture with a `pass` entry carrying the criterion id emits none. | Test |
| FR-175-AC-6 | A fixture with a `Criterion` row and a `Test` row (`valid/closed-fixed.md`), where the `Test` row is `passing` and the `Criterion` row is `missing`, emits no finding; where the `Criterion` row is `failing`, it emits `anomaly.fix-test-failing` naming the criterion id. | Test |
| FR-175-AC-7 | A `resolved` anomaly with `resolution` `fixed` (`valid/resolved-fixed.md`) is evaluated like a `closed` one; anomalies with `resolution` `not-a-defect`, `duplicate` or `wont-fix`, and an anomaly at `assessed`, are never evaluated and yield none of the three codes. | Test |
| FR-175-AC-8 | An anomaly whose `Test` row targets an id that resolves to nothing (`invalid/verification-dangling-test.md`) yields none of the three codes and no dangling-reference diagnostic from `quoin anomaly check`; a document with a mapping error yields only `anomaly.no-record`. | Test |
| FR-175-AC-9 | Every finding of AC-2, AC-3 and AC-5 carries its code, severity `error`, its reason and the line of the document's `status` field. | Test |
| FR-175-AC-10 | Every file in the corpus and the store has identical bytes before and after a run; two runs over the same inputs give identical output; and the output contains no sentence stating that a fix is verified, accepted or acceptable. | Test |
| FR-175-AC-11 | The anomaly documents and corpus the tests use are those shipped by `agent-ix/spec-artifacts-process`, read in place from the sibling checkout; the evidence stores are built in each test's temporary directory; the quoin repository contains no copy of either. | Inspection |

## Dependencies

- **Upstream**: [FR-136](./FR-136-anomaly-record-mapping.md), [FR-030](./FR-030-evidence-store.md), [FR-120](./FR-120-trace-marks-in-test-result-output.md); `agent-ix/spec-artifacts-process` [FR-015](ix://agent-ix/spec-artifacts-process/FR-015).
- **Constrained by**: [NFR-035](../non-functional/NFR-035-anomaly-report-states-and-never-decides.md) in spirit (the check states and does not accept).
