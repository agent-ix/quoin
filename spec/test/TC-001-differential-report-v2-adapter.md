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
   one report captured from `quire-analyze run-package` for each disposition the runner can produce: agreement
   satisfied, agreement refuted, disagreement, inconclusive, and unavailable (missing executable); and over a
   captured or hand-constructed v2 document, accepted by the owner's validator, with an engine `unsupported-platform`
   (a Linux runner reports `cleanup-failed` instead, so this one is not a runner capture). Read the store and the
   command's human and JSON output after each.
2. Run the same command, each time against a fresh store, over the refusal table of FR-150-AC-14, one row per
   refusal class, each derived from a capture by one stated change: wrong, absent or non-string `schema`; a missing,
   extra (top level and engine), wrongly typed or out-of-set member; empty, truncated, non-UTF-8, array, scalar and
   non-JSON input; the four self-contradiction reports; a reordered copy and a pretty-printed copy.
3. Run the command over an input of `quire_analyze::MAX_REPORT_BYTES` + 1 bytes and one of exactly `MAX_REPORT_BYTES`
   bytes, and over one valid report twice from different working directories.
4. Run `quoin evidence record --tool quire-analyze`, `--tool differential-report`, `--tool tl-mltl`, `--tool r2u2` and
   `--tool c2po` over a valid report, and list the adapters.

## Expected Results

Step 1: agreement satisfied records one `pass` entry and agreement refuted one `fail` entry, both under symbol
`<analysisKind>::<requestDigest>`; inconclusive and each unavailable report record one `error` entry; disagreement
records a run with zero entries and prints one `unrepresented` result of state `disagreement` naming both engines
with status and outcome verbatim. No entry carries `traceIds`.

Step 2: every input is refused with a non-zero exit and an error naming the schema value or member found (the
owner's validator error for the rest), and the store holds no run.

Step 3: the `MAX_REPORT_BYTES` + 1 input is refused naming the size limit without being parsed, the input of exactly
`MAX_REPORT_BYTES` bytes is not refused for size, and the two valid runs return identical results.

Step 4: `quire-analyze` and `differential-report` select the adapter, `tl-mltl`, `r2u2` and `c2po` do not, and the
listing names `quire.differential-report/v2` and no `differential-summary`.
