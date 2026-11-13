---
id: TC-002
title: "An analysis result is discharged, bound and counted only as a qualified passing record"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-152"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "verifies"
  - target: "ix://agent-ix/quoin/NFR-033"
    type: "verifies"
---

# TC-002: An analysis result is discharged, bound and counted only as a qualified passing record

## Description

Verify that a `quire.differential-report/v2` report becomes evidence for an obligation only through the ids the recorder states,
and that each disposition is counted, or not counted, as FR-153 states, through the FR-078 `qualified-record` path.

## Test Procedure

1. With a fixture repository holding obligations `FR-001-AC-1` and `FR-001-AC-2` (declared method Analysis), an
   empty store and a registered suite, record each captured report of TC-001 (agreement satisfied, agreement
   refuted, disagreement, inconclusive, unavailable) with `--discharges FR-001-AC-1`, each into a fresh store. Read the
   run record, the binding graph and the command's human and JSON output.
2. Record the agreement-satisfied report with no `--discharges`; with `--discharges FR-001-AC-1,FR-001-AC-2`; with an id
   naming no obligation beside a valid one; and with `--adapter junit --discharges FR-001-AC-1`. Then record the
   unedited satisfied report with no `--discharges` into a repository where an obligation id differs from the symbol,
   the `analysisKind` and the `requestDigest`.
3. Over the store of step 1 for the satisfied report, run the qualifier and bridge with an approved producer
   definition and every other premise admitted, then again with no FR-093 assessment, then again with no CI checkout
   SHA for the run-package step, then run `quoin evidence audit`. Read the FR-078 report, trace (forward and inverse) and strict decision.
4. Over the other four stores of step 1, run the bridge with the same admitted premises and read the report.
5. Into one suite record the satisfied report and then the refuted report with a later `--timestamp`; then a second
   request's report; then the same sequence into two suites; then the two with equal timestamps. The satisfied and
   refuted pair with one `requestDigest` is hand-constructed and accepted by the validator, as FR-151-AC-5 does for
   `unsupported-platform`, because a deterministic capture cannot produce both. Supply the run records in both orders.
6. Record the satisfied report with `--discharges` naming an obligation whose declared method is Test and read the
   bridge result.

## Expected Results

Step 1: the satisfied report stores one entry with `traceIds` `["FR-001-AC-1"]` and one binding to the symbol
`<analysisKind>::<requestDigest>`; the refuted, inconclusive and unavailable reports store the entry with the stated id
and no binding; the disagreement report stores a run with zero entries, no binding and prints the
`unrepresented` result.

Step 2: no `--discharges` stores an entry with no `traceIds` and adds no binding entry; two ids bind both
obligations with `traceIds` sorted; the unmatched id is listed, not stored on the entry, and the valid one binds; the
`junit` invocation is refused naming the adapter, writing no run; the unedited report with no `--discharges` binds
nothing.

Step 3: with every premise admitted the obligation is `qualified-record` with record, run and entry locators and
leaves `unbacked_rows`; with no FR-093 assessment it is `unevaluated` naming the bounded-reliance premise, and with no CI checkout SHA it is
`unevaluated` naming the performed-revision premise; each stays in `unbacked_rows` and strict mode rejects; a healthy audit with no qualifier decision is not counted; forward and inverse trace return the same
record claim and a source-symbol query for the symbol returns none.

Step 4: the refuted store reports `failed`, inconclusive and unavailable report `errored`, disagreement reports
`absent` with no invented locator; none adds a claim and strict mode rejects.

Step 5: a later fail or error in the same suite makes the obligation `stale` with the later `failed` or `errored`
outcome visible, and a later run lacking the symbol makes it `absent`, in either supply order and with equal timestamps;
a second request in the same suite replaces the first; in two suites both are bound and both must pass.

Step 6: the result is `qualification-rejected` naming the method and act pairing premise; it is not counted.

## Notes

Steps 3 to 6 need the FR-116 qualifier and the `quoin-quire` bridge, which are planned. Until they exist the
qualified assertions are not run and this test case does not pass; steps 1 and 2 run on the existing store.
