---
id: TC-001
title: "Differential report v2 adapter transcribes every disposition and refuses malformed reports"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-150"
    type: "verifies"
  - target: "ix://agent-ix/quoin/NFR-032"
    type: "verifies"
---

# TC-001: Differential report v2 adapter transcribes every disposition and refuses malformed reports

## Description

Verify that the `differential-report` adapter maps each `quire.differential-report/v2` disposition to the run
entry or `unrepresented` result of FR-150, and refuses reports that are the wrong version, malformed,
self-contradictory or oversized, writing no run in any refusal.

## Test Procedure

1. With an empty evidence store and a suite registered, run `quoin evidence record --adapter differential-report` over
   one report captured from `quire-analyze run-package` for each disposition: agreement satisfied, agreement
   refuted, disagreement, inconclusive, and unavailable (once with a missing executable and once with an
   unsupported platform). Read the store and the command's human and JSON output after each.
2. Run the same command, each time against a fresh store, over: a `quire.differential-report/v1` document; a
   `tl-mltl.differential-summary/v1` document; a v2 document with a required member removed, an undeclared member
   added at the top level and inside an engine, a wrongly typed member, and an out-of-set disposition, engine status
   and engine outcome; empty, truncated, non-UTF-8, array and scalar inputs; an `agreement` report with a `null`
   `agreedStatus`, a `disagreement` report with a non-null one, an `agreement` report whose engine status differs
   from `agreedStatus`, and a report naming one engine twice.
3. Run the command over inputs of 16,777,217 bytes and exactly 16,777,216 bytes (the second otherwise valid), and
   over one valid report twice from different working directories with its object members reordered the second time.
4. Run `quoin evidence record --tool quire-analyze`, `--tool differential-report`, `--tool tl-mltl`, `--tool r2u2` and
   `--tool c2po` over a valid report, and list the adapters.

## Expected Results

Step 1: agreement satisfied records one `pass` entry and agreement refuted one `fail` entry, both under symbol
`<analysisKind>::<requestDigest>`; disagreement and inconclusive each record one `error` entry; each unavailable
report records no entry and prints one `unrepresented` result of state `unavailable` naming both engines with status
and outcome verbatim. No entry carries `traceIds`.

Step 2: every input is refused with a non-zero exit and an error naming the schema value or member found, and the
store holds no run.

Step 3: the 16,777,217-byte input is refused naming the size limit without being parsed, the 16,777,216-byte input is
accepted, and the two valid runs return identical results.

Step 4: `quire-analyze` and `differential-report` select the adapter, `tl-mltl`, `r2u2` and `c2po` do not, and the
listing names `quire.differential-report/v2` and no `differential-summary`.
