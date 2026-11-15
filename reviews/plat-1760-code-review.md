---
id: SR-20261115183517
title: "Code review — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md; PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. Review of quoin PR #931 at `a0c40bd7`, a spec-only diff to
FR-115 (+19/-2): an optional typed `cause` (`missing-run` | `failed-run`) on
`stale-evidence` findings, carried into `evidence_detail.findings`, and the new
criteria FR-115-AC-17..19. The spec text was read against FR-032 and the
current auditor producers (`rust/crates/quoin-auditor/src/audit/stale.rs`,
`ladder.rs`) and the matrix join (`rust/crates/quoin-assurance/src/matrix.rs`).
No Rust or React changed, so neither lane ran. No build or test was run.

## Verdict

**CONDITIONAL**: two medium findings and one low. The design is sound. The
cause adds information and leaves status, precedence and summary unchanged,
it matches the two producers that exist (`unrecorded_evidence`, `failed_run`),
and it is read through CON-2 without being re-derived. No duplication,
vendoring or ceremony was added. The findings are about an AC that is false
in a reachable fixture and an input rule that is not stated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-115-AC-18's second sentence, "A passing bound symbol remains `bound` with no `stale-evidence` finding", is false whenever no mock inspection is recorded for the suite. FR-032-AC-16 then emits a `not-evaluated` entry, and FR-115-AC-4 maps that to `undischarged`, which is why FR-115-AC-6 has to name the mock inspection explicitly. A passing symbol can also be `suspect`. The sentence needs the same precondition AC-6 states. | spec/functional/FR-115-evidence-backed-test-matrix.md:317 |
| FND-002 | medium | FR-115 does not say what `matrix.build` does with a supplied `audit` whose finding has a `cause` on a non-`stale-evidence` kind. The Outputs bullet says `cause` is "copied verbatim" into `evidence_detail.findings`. FR-115-AC-19 says a non-stale finding "serializes with no `cause` key in either output". `matrix.build` takes the audit on stdin, and `Finding` accepts unknown members, so this input can reach it. Copy, drop or refuse: two implementers would pick differently. | spec/functional/FR-115-evidence-backed-test-matrix.md:318 |
| FND-003 | low | Which `cause` is reported is unstated when one obligation has one binding whose suite has no run and another whose run failed. The ladder stops at the first rung (`unrecorded_evidence` before `failed_run`), so only `missing-run` is emitted. FR-115-AC-19's "a consumer distinguishes the two stale conditions from the `cause` field alone" overstates this, because a co-occurring failed run is hidden. | spec/functional/FR-115-evidence-backed-test-matrix.md:216-228 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-115-AC-18 now covers only a failed latest run. The passing-symbol case is split into AC-19 with a recorded mock inspection, and AC-20 covers a missing inspection as undischarged. |
| FND-002 | fixed | FR-115 now refuses a cause on any finding kind other than stale-evidence with invalid-stale-cause (Behavior and FR-115-AC-23). |
| FND-003 | fixed | FR-032 now emits at most one stale finding per obligation, choosing the first stale suite in bytewise order (FR-032-AC-20); FR-115 no longer claims every stale condition is separately visible. |
