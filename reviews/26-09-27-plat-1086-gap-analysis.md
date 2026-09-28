---
id: SR-169
title: "Gap analysis — PLAT-1086 auditor failed-run stale evidence"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@e829cdf6e06e82c534cb84ca2267d97412f5adb8; PR 647 diff vs merge base 870f6c5: FR-032 (AC-17 and CR note), spec/matrix.md (FR-032 row, TC-1963), rust/crates/quoin-auditor/src/audit/ladder.rs, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-032"
    type: "reviews"
---

# SR-169: Gap analysis — PLAT-1086 auditor failed-run stale evidence

## Summary

Ticket: PLAT-1086. PR agent-ix/quoin#647 at `e829cdf`. This is a planless audit of the chain
FR-032-AC-17, then TC-1963, then the tagged tests, then `failed_run`. `quire coverage --scope .
--json` (quire 0.33.0) mints FR-032-AC-17 and reports it `backed: true`, bound through the four
`tc_1963_*` tests' `Trace: FR-032-AC-17` lines. The code path exists and implements the AC's core
claim. The gaps are matrix accuracy (a phantom test locus) and claims in AC-17 and TC-1963 that no
test backs, or that the code does not implement.

## Verdict

**CONDITIONAL**. There are no high findings. The code gap is closed. Matrix and spec text overclaim
in two places (FND-001, FND-002), and both should be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1963's matrix row cites `quoin-auditor` `src/audit/ladder.rs :: tests::tc_1963_a_failed_or_errored_run_leaves_stale_evidence`, and no such test exists. The four tests are in `tests/tc_1963_failed_run_stale_evidence.rs`. The FR-032 row likewise cites `quoin-auditor/src/audit/ladder.rs` for TC-1963. The engine does not catch this, because it binds through Trace tags, so a reader following the ✅ lands on a phantom locus. | spec/matrix.md:1455, spec/matrix.md:75 |
| FND-002 | medium | TC-1963's row says "whether or not the run is at HEAD", and AC-17 says the rung "fires independently of AC-3's behind-HEAD check". Every fixture records its run at HEAD (`COMMIT` is both the run commit and `head_commit`), so the behind-HEAD half of the row is unbacked. The row reads ✅ over a case no test runs. | spec/matrix.md:1455, spec/functional/FR-032-evidence-auditor.md:126, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:28 |
| FND-003 | low | AC-17 says "newest recorded run at or before HEAD", but nothing implements "at or before HEAD". The store's `latest_runs` is newest by timestamp, then commit string, with no ancestry check, and the auditor takes what it is handed. A run recorded later at a commit not reachable from HEAD (for example, another branch in the same store) is "newest". The AC overclaims. It should state the FR-030 "newest by timestamp" rule, or an ancestry filter should be specified and built. | spec/functional/FR-032-evidence-auditor.md:126, rust/crates/quoin-evidence/src/store/records.rs:207-230 |
| FND-004 | low | FR-032's "Severity says what kind of wrong" section lists the High cases (suspect link, missing run, all-skipped) and does not add a failed run. AC-17's high severity matches that section's principle, but the list is now incomplete. | spec/functional/FR-032-evidence-auditor.md:39-44 |
| FND-005 | low | Once merged, text on main goes stale. FR-115's "`bound`'s passing promise depends on an upstream auditor gap (CR-002)" section, its Dependencies bullet, and the FR-115 matrix row ("Blocked on ... PLAT-1086") all describe this gap as open. The branch predates FR-115 (main `e15f317`), so the PR cannot edit that text without a rebase. The merge is clean (`git merge-tree` produced no conflicts). | spec/functional/FR-115-evidence-backed-test-matrix.md:88-101 (origin/main), spec/matrix.md FR-115 row (origin/main) |

## Coverage

- Matrix verification: `quire coverage --scope . --json` shows FR-032-AC-17 minted and backed. No
  TC-1963 or FR-032-AC-17 entries appear in `unbacked_rows`, `status_lies` or `untracked_symbols`.
  Totals: backed 518 of 2197, repo-wide and unchanged in kind by this PR.
- Tagged tests: 4 `tc_1963_*` functions, each carrying `/// Trace: FR-032-AC-17`, with `Provenance:
  PLAT-1086` on the module header. All 4 pass.
- Reverse gap: `failed_run` is owned by FR-032 line 28 and AC-17. Nothing in the diff lacks an owner.
- Stubs and inflation: none in source. Test-oracle weaknesses are recorded in SR-168 FND-002 through
  FND-005.
- Semantic review: covered by the mutation run recorded in SR-168. No separate subagent fan-out,
  because the scope is a single rung.
- Plan completion: not assessed

## Dispositions

Round 1, reviewed at `f72ec1cb765737ff6996c80f794cb795a0db81a1`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f72ec1c: the TC-1963 and FR-032 matrix rows now cite `tests/tc_1963_failed_run_stale_evidence.rs` and name all nine `tc_1963_*` functions. All nine exist, and `quire coverage` reports no unbacked rows and no untracked TC-1963 symbols. |
| FND-002 | fixed | f72ec1c: AC-17 and TC-1963 state that the failed-run finding takes precedence over the behind-HEAD finding, and `tc_1963_a_failed_run_behind_head_reports_only_the_high_finding` runs at `OLDER_COMMIT` while HEAD is `COMMIT`. |
| FND-003 | fixed | f72ec1c: AC-17 now reads "newest recorded run — the same run the store's `latest_runs` selects (FR-030, newest by timestamp)". |
| FND-004 | fixed | f72ec1c: FR-032's Severity section lists a failed or errored newest run under High. |
| FND-005 | fixed | f72ec1c: after the rebase onto e15f317, FR-115's CR-002 section, CON-2, Dependencies and the FR-115 matrix row all describe PLAT-1086 as resolving the gap. |

Round 2, reviewed at `258981a4076b1d851f640c1319841791c0a97245`: no rows. Every finding's latest outcome was already `fixed` in round 1, and the rebase changed no spec or matrix content (`git range-diff` shows the three prior commits unchanged).
