---
id: SR-168
title: "Code review — PLAT-1086 auditor failed-run stale evidence"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@e829cdf6e06e82c534cb84ca2267d97412f5adb8; PR 647 diff vs merge base 870f6c5: rust/crates/quoin-auditor/src/audit/ladder.rs, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs, rust/crates/quoin-auditor/tests/tc_383_module_sizes.rs, spec/functional/FR-032-evidence-auditor.md, spec/matrix.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-032"
    type: "reviews"
---

# SR-168: Code review — PLAT-1086 auditor failed-run stale evidence

## Summary

Ticket: PLAT-1086. PR agent-ix/quoin#647 at `e829cdf`. This review covers `code-review` with the
`rust-review` lane folded in, per the reviewer contract.

The PR adds a `failed_run` rung to `quoin-auditor`'s ladder. The rung reports `stale-evidence` at high
severity when the newest run of a bound suite records `fail` or `error` for a bound symbol. The rung
logic is correct. It reads `Indexed::runs_by_suite`, which keeps the last run per suite (last one wins).
In production that is the store's `latest_runs` (`latest_each`: newest per suite by timestamp), so a
failing run that a later passing run has superseded does not flag. The rung checks only the binding's
own symbols, so a failure on another obligation's test in the same run does not spill over. High
severity matches FR-032's rule: evidence that claims to exist and does not hold.

The blocking defect is in the new test file. It fails the workspace `clippy -D warnings` gate on
`clippy::indexing_slicing`. That failure is masked because the pre-existing `quoin-measurement` lint
failure aborts cargo before the auditor's test targets are linted. Mutation testing killed three of
six mutants and let three survive: the severity mutant, the "latest run" mutant and the single-finding
mutant.

## Verdict

**FAIL**. FND-001 (high) must be fixed: the PR's own code breaks `make rust-lint`, independently of
the pre-existing `quoin-measurement` failure. The medium findings are test-oracle gaps. Each is a
short test addition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | New test file fails workspace `cargo clippy --all-targets -- -D warnings` with 5x `clippy::indexing_slicing` (`stale[0]`). Its `#![allow]` omits `clippy::indexing_slicing`, which every sibling test in the crate allows. Cargo aborts on the pre-existing `quoin-measurement` failure first, so this failure is masked. Once PLAT-1088 fixes main, `make rust-lint` still fails on this PR's own code. | rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:17-22, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:119, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:121, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:155, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:157 |
| FND-002 | medium | AC-17 requires high severity and a summary naming suite, symbol and commit. No test asserts severity: mutating `Severity::high()` to `Severity::medium()` in `failed_run` survives the whole crate suite. The summary check is `contains("unit") && contains("fail")`, which never checks the symbol or the commit. | rust/crates/quoin-auditor/src/audit/ladder.rs:312, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:118-122 |
| FND-003 | medium | The "latest run" semantics are unpinned. The tests are named `pass_then_fail` / `pass_then_error`, but each fixture supplies a single run, so no case has two runs of one suite. Mutating `Indexed::of` so the oldest run per suite wins survives the whole crate suite. Neither pass-then-fail nor fail-then-pass (which must stay healthy) is asserted. | rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:93-96, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:129-132, rust/crates/quoin-auditor/src/audit/mod.rs:72-75 |
| FND-004 | medium | Placing the rung before `behind_head` means a failed run that is also behind HEAD reports only the high failed-run finding, and the medium behind-HEAD finding is suppressed. Both are `stale-evidence`, so the ratchet key is the same, which makes this defensible. But no test covers the failed-and-behind case (every fixture has run commit == HEAD), although TC-1963 claims "whether or not the run is at HEAD". Also, removing the rung's early `return` survives the suite, so "one ladder finding per obligation" is not asserted for this rung. | rust/crates/quoin-auditor/src/audit/ladder.rs:98-101, rust/crates/quoin-auditor/src/audit/ladder.rs:162-166, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:28 |
| FND-005 | low | The `healthy` assertions in the fail and error tests are vacuous. Neither test sets `mock_inspection_suites`, so the obligation is `unevaluated` (never healthy) whether or not the rung exists. With the rung removed, both tests failed only on the stale-count assertion, never on the healthy one. | rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:97-111, rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:133-147 |
| FND-006 | low | The rung's doc comment is inaccurate. It says "`#[cfg(test)]`-only inspection had checked `Outcome::Skip` alone", but the vacuity rung is production code, not `cfg(test)`, and the sentence narrates history rather than the incident. | rust/crates/quoin-auditor/src/audit/ladder.rs:271-274 |
| FND-007 | low | The module-size ceiling entry's argument contradicts the crate's own pattern. Rungs already live in sibling modules and are called from `run`: `method_conformance` (method.rs), `mocked_finding` (mocks.rs), `multiplicity_finding` and `mutation_finding` (scores.rs). Moving `failed_run` (or the stale-evidence trio `unrecorded_evidence`/`failed_run`/`behind_head`) into e.g. `audit/stale.rs` keeps `run` readable in one screen. The soft ceiling does permit a listed entry, so this is not a gate violation. The edit also deleted the previous rationale for keeping the list empty. `ladder.rs` is 548 lines (soft 500, hard 700). | rust/crates/quoin-auditor/tests/tc_383_module_sizes.rs:45-54, rust/crates/quoin-auditor/src/audit/ladder.rs:1-15 |
| FND-008 | low | `DIVERGENCE.md` "What is NOT divergent" still lists "The ladder order", but the ladder now has a rung the retained TypeScript lacks. The golden corpus holds only `pass` (24) and `skip` (3) outcomes, so `tc_383_parity` passes without testing anything on this point. The new rung is an undeclared divergence. | rust/crates/quoin-auditor/DIVERGENCE.md:165-169, rust/crates/quoin-auditor/tests/goldens/auditor.json |

## Finding detail

**FND-001.** Reproduced with `cargo clippy -p quoin-auditor --no-deps --all-targets --all-features
--locked -- -D warnings`, which fails with `error: indexing may panic` at 119:9, 119:38, 121:9, 155:9
and 157:9. `cargo clippy --workspace ...` shows the same five errors. The file's own `#![allow]`
covers `unwrap_used`, `expect_used` and `panic` only. Fix: add `clippy::indexing_slicing` to the
allow list, or bind with `let [only] = stale.as_slice() else { panic!(..) };`.

**Mutation results.** A copy of `rust/` was taken via `git archive HEAD` and each mutant was run
through `cargo test -p quoin-auditor`:

| Mutant | Result | Killing assertion |
| --- | --- | --- |
| M1 drop the rung call | killed | fail/error tests: stale count == 1 (lines 113, 149); sibling test: OTHER obligation stale (line 233). This is the intended reason. |
| M2 match `Fail` only, not `Error` | killed | error test, stale count (line 149). This is the intended reason. |
| M3 ignore symbol identity (any failing entry in the run) | killed | sibling test, watched obligation stays healthy (line 224). This is the intended reason. |
| M4 `Severity::high()` to `medium()` | **survived** | none (FND-002) |
| M5 oldest run per suite wins in `Indexed::of` | **survived** | none (FND-003) |
| M6 no early return after the rung | **survived** | none (FND-004) |

**Checks the brief asked for.** On "latest run", the answer is yes in production. `quoin-core`
`ops/evidence/store.rs:200` calls `latest_runs`, which `records::latest_each` resolves to the newest
per suite by timestamp, then commit. `Indexed::of` keeps the last run in input order. So an older
failing run that a passing run has superseded cannot flag. This is correct but untested (FND-003). On
the scope of the fail check, the rung matches `entry.symbol == binding symbol`, so a failure on a
different obligation's test stays out, and M3 proves that.

## Coverage

Examined: `ladder.rs` (whole file), `audit/mod.rs` (`Indexed`, `audit`), `quoin-evidence`
`store/records.rs` (`latest`, `latest_each`), `quoin-core` `ops/evidence/store.rs:200`, the new test
file, `tc_383_module_sizes.rs`, `tc_383_parity.rs`, `DIVERGENCE.md`, the golden corpus's outcome
census, the FR-032 diff and the matrix diff. Rust-review lane: panics in production (none added),
`unsafe` (none), integer conversions (none), resource bounds (the per-obligation loop is linear in the
entries of each run times its symbols, the same shape as `vacuous_run`), the tc_NNN/`Trace:`
convention (followed), and the lint policy (FND-001).

Gates at `e829cdf` (own `CARGO_TARGET_DIR`, after `pnpm install --frozen-lockfile`):
`cargo fmt --all --check` clean. `cargo test -p quoin-auditor` all pass. `cargo test --workspace
--locked` all pass (0 failed). `cargo clippy --workspace --all-targets --all-features -D warnings`
FAILS, both on the PR's test file (FND-001) and on pre-existing `quoin-measurement` errors
(`plans.rs:122` dead code, `expect_used` in `campaign/`, `indexing_slicing` in
`tc_1942_campaign_verification.rs`). `git diff 870f6c5 e829cdf -- rust/crates/quoin-measurement` is
empty, so those errors are pre-existing (PLAT-1088's lane).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Two doc-accuracy slips from the fix round. First, the test module header at rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:8 says "`src/audit/stale.rs`'s only outcome-aware rung was vacuity", but vacuity lives in `ladder.rs` (`vacuous_run`), so this looks like a find-replace of `ladder.rs` with `stale.rs`. Second, the DIVERGENCE.md intro (rust/crates/quoin-auditor/DIVERGENCE.md:17-19) still counts "three divergences, one non-divergence and one inherited refusal", and does not mention the new §6. Both are cosmetic and neither affects behaviour. | rust/crates/quoin-auditor/tests/tc_1963_failed_run_stale_evidence.rs:8, rust/crates/quoin-auditor/DIVERGENCE.md:17-19 |

## Dispositions

Round 1, reviewed at `f72ec1cb765737ff6996c80f794cb795a0db81a1`, after a rebase onto `e15f317`. The fix commit is `f72ec1c`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f72ec1c: `clippy::indexing_slicing` is added to the allow list, and the single-finding path now uses let-else. `cargo clippy -p quoin-auditor --no-deps --all-targets --all-features -D warnings` is clean, and workspace clippy reports no auditor errors. |
| FND-002 | fixed | f72ec1c: the tests now assert `Severity::high()` and check that the summary contains `unit:tests::tc_900` and the commit. Mutant M4 (high → medium) is killed by 5 tests. |
| FND-003 | fixed | f72ec1c: two tests go through the store (`write_run`/`latest_runs`), and two tests go through `Indexed::of` directly, covering both pass-then-fail and fail-then-pass. Mutant M5 (oldest run wins) is killed by both index tests. |
| FND-004 | fixed | f72ec1c: `tc_1963_a_failed_run_behind_head_reports_only_the_high_finding` pins the case to exactly one high finding. Mutant M6 (no early return) is killed, with 2 findings seen. AC-17 now states the precedence. |
| FND-005 | fixed | f72ec1c: the `an_input` helper sets `mock_inspection_suites`. With the rung dropped (M1), the `!healthy` assertions now fail at lines 144 and 185. |
| FND-006 | fixed | f72ec1c: the doc comment in stale.rs:53-61 is corrected. |
| FND-007 | fixed | f72ec1c: `unrecorded_evidence`, `failed_run` and `behind_head` move to `audit/stale.rs` (152 lines), leaving ladder.rs at 420 lines. `OVER_SOFT_CEILING` is empty again, and the rationale it states is restored. |
| FND-008 | fixed | f72ec1c: DIVERGENCE.md gets a new §6, and the "What is NOT divergent" ladder-order bullet now names the exception. |
| FND-009 | fixed | 258981a: the test module header now attributes the vacuity check to `ladder.rs`, and the DIVERGENCE.md intro counts four divergences, covering §6. Round 2 reviewed `258981a4076b1d851f640c1319841791c0a97245`, rebased onto `fe52d51`. `git range-diff e15f317..f72ec1c fe52d51..258981a` shows the three prior commits unchanged (=) and one new commit that touches only these two lines and the `reviews/` copies of SR-168/SR-169. |
