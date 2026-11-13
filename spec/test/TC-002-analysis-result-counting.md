---
id: TC-002
title: "An analysis result is linked, bound and counted only as a qualified passing record"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-152"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "verifies"
  - target: "ix://agent-ix/quoin/NFR-033"
    type: "verifies"
---

# TC-002: An analysis result is linked, bound and counted only as a qualified passing record

## Description

Verify that a `quire.differential-report/v2` report becomes evidence for an obligation only through an explicit link,
and that each disposition is counted, or not counted, as FR-153 states, through the FR-078 `qualified-record` path.

## Test Procedure

1. With a fixture repository holding obligations `FR-001-AC-1` and `FR-001-AC-2` (declared method Analysis), an
   empty store and a registered suite, record each captured report of TC-001 (agreement satisfied, agreement
   refuted, disagreement, inconclusive, unavailable) with `--link FR-001-AC-1`, each into a fresh store. Read the
   run record, the binding graph and the command's human and JSON output.
2. Record the agreement-satisfied report with no `--link`; with `--link FR-001-AC-1 --link FR-001-AC-2`; with a link
   naming no obligation beside a valid one; with `--adapter junit --link FR-001-AC-1`; and a copy whose
   `analysisKind` and `requestDigest` are set so that the symbol equals `FR-001-AC-1`.
3. Over the store of step 1 for the satisfied report, run the qualifier and bridge with an approved producer
   definition and every other premise admitted, then again with no approved producer definition, then run
   `quoin evidence audit`. Read the FR-078 report, trace (forward and inverse) and strict decision.
4. Over the other four stores of step 1, run the bridge with the same admitted premises and read the report.
5. Into one suite record the satisfied report and then the refuted report (same request); then a second request's
   report; then the same sequence into two suites. Supply the run records in both orders.

## Expected Results

Step 1: the satisfied report stores one entry with `traceIds` `["FR-001-AC-1"]` and one binding to the symbol
`<analysisKind>::<requestDigest>`; the refuted, inconclusive and unavailable reports store the entry with the link
and no binding; the disagreement report stores a run with zero entries, no binding and prints the link as unbound
with state `disagreement`.

Step 2: no link stores an entry with no `traceIds` and leaves the binding graph unchanged; two links bind both
obligations in supplied order; the unmatched link is listed and the valid one binds; the `junit` invocation is
refused naming the adapter, writing no run; the crafted symbol with no link binds nothing.

Step 3: with every premise admitted the obligation is `qualified-record` with record, run and entry locators and
leaves `unbacked_rows`; without an approved producer definition it is `unevaluated` naming that premise and strict
mode rejects; a healthy audit with no qualifier decision is not counted; forward and inverse trace return the same
record claim and a source-symbol query for the symbol returns none.

Step 4: the refuted store reports `failed`, inconclusive and unavailable report `errored`, disagreement reports
`absent` with no invented locator; none adds a claim and strict mode rejects.

Step 5: a later fail, error or missing symbol in the same suite leaves the obligation uncounted in either supply
order; a second request in the same suite replaces the first; in two suites both are bound and both must pass.

## Notes

Steps 3 to 5 need the FR-116 qualifier and the `quoin-quire` bridge, which are planned. Until they exist the
qualified assertions are not run and this test case does not pass; steps 1 and 2 run on the existing store.
