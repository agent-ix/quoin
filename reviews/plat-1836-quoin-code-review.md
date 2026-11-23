---
id: SR-2026112016223475846355008
title: "PLAT-1836 quoin PR 1032 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR 1032 (rust/crates/quoin-auditor/src/audit/{class,scores,ladder}.rs, rust/crates/quoin-cli/src/{class_obligations.rs,evidence/audit.rs}, rust/crates/quoin-auditor/tests/tc_160_class_floors.rs, rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-160"
    type: references
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
---

# PLAT-1836 quoin PR 1032 code-review

## Summary

Ticket: PLAT-1836. Code review with the Rust lane (rust-review + rust-style) folded in, scoped to `git diff origin/main...HEAD` at dcc95f29. Production code has no `unwrap`, indexing or `todo!`; ordering is deterministic (BTreeSet, policy order, join order); `cargo fmt --check` is clean. The gate only changes the `Response.outcome` (to `Partial`, exit 1, which still carries the payload), never the rendered report, and since only `Ok`/`Partial` carry a payload it can never downgrade a worse outcome. No class value, default policy or ratio appears in production source (test fixtures only). The `tc_161_002` fixture rename is legitimate: `store::records::read_all` skips any run file whose path differs from `record_path(suite, timestamp)`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `run_passed` judges the whole suite run, not the binding's symbols. Every other outcome check in the auditor (`stale::failed_bound_symbols`, `vacuous_run`) reads only the entries for `binding.symbols`. Consequences: (a) silent pass - a binding whose own symbols are all `skip` or absent, in a run where any other symbol passed, satisfies the required method, so under `gate: true` the class gate passes (the `vacuous-evidence` finding the ladder also emits carries no `class` member and does not gate); (b) false failure - one unrelated failing test anywhere in a suite raises `class-method-missing` "did not pass" for every class obligation bound to that suite and fails the gate. FR-160's new passing-run sentence states the whole-run rule, so spec and code agree with each other but not with the binding-symbol model of FR-032. Judge pass over the entries of the bound symbols (at least one bound symbol `pass`, none `fail`/`error`) and restate FR-160 and FR-160-AC-14 accordingly. | rust/crates/quoin-auditor/src/audit/class.rs:374-385; rust/crates/quoin-auditor/src/audit/class.rs:263-284; spec/functional/FR-160-per-class-evidence-floors.md:73-78 |
| FND-002 | low | The gate tests assert only `!= Some(0)` for the gated case (`audit("gate.yaml")`, and twice in tc_plat1836_004). A refused policy or any error exit (2, 3) satisfies them, and nothing asserts the class finding or report is present in that gated run, so they do not show the gate "only changes the exit status". Assert `Some(1)` and that stdout carries the same findings/rows as the ungated run. | rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:600; rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:704; rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:709 |
| FND-003 | low | The activity kind is a stringly-typed closed vocabulary: kind names are matched in `parse_activities` and again in `build_rows` (via `RELATION_KINDS` and literals), and `build_rows` keeps a catch-all `_ => not-available "unknown activity kind"` arm that maps an unknown kind to a valid status. rust-style: match closed enums exhaustively; do not map an unknown variant to a valid value. Parse the kind once into an enum (serde `rename_all = "kebab-case"`) so a sixth kind is a compile error at both sites. | rust/crates/quoin-cli/src/class_obligations.rs:71; rust/crates/quoin-cli/src/class_obligations.rs:182-188 |
| FND-004 | low | `relation_status` treats every non-`covered` row state as unsatisfied, while `design_status` (design-trace, same FR-086 join) counts only `uncovered` as unmet and falls back to `not-available` otherwise. Any third row state the join emits reads `unmet` for `detailed-design` and `not-available` for `design-trace` on the same row. Either share one row reading or state the difference in FR-161. | rust/crates/quoin-cli/src/class_obligations.rs:300-338; rust/crates/quoin-cli/src/class_obligations.rs:350-404 |

## Gates

Run via `/home/peter/dev/worktrees/locked-build.sh` (debug=0, incremental=0, shared CARGO_HOME) at dcc95f29: `cargo test -p quoin-auditor --test tc_160_class_floors` 25 passed; `cargo test -p quoin-cli --bin quoin class_obligations` 8 passed; `cargo test -p quoin-cli --test tc_160_class_policy_command` 11 passed (includes tc_161_001/002 and tc_plat1836_001..004); `cargo clippy -p quoin-auditor -p quoin-cli --all-targets` emits no warning in any file of the diff (pre-existing warnings elsewhere only); `cargo fmt --check -p quoin-cli -p quoin-auditor` clean.

## Verdict

CHANGES REQUESTED (FND-001). The gate is implemented as an exit-status-only switch, as required; the `class` member is applied at every class-caused site (method-missing, unconfigured, class-set mutation/unmeasured/surviving-mutant, and the pre-existing class independence finding); policy parsing refuses malformed relation-backed entries.

## Examined

- rust/crates/quoin-auditor/src/audit/class.rs (examined)
- rust/crates/quoin-auditor/src/audit/scores.rs (examined)
- rust/crates/quoin-auditor/src/audit/ladder.rs (examined)
- rust/crates/quoin-cli/src/class_obligations.rs (examined)
- rust/crates/quoin-cli/src/evidence/audit.rs (examined)
- rust/crates/quoin-auditor/tests/tc_160_class_floors.rs (examined)
- rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs (examined)
- rust/crates/quoin-evidence/src/stale.rs, rust/crates/quoin-evidence/src/store/records.rs, PLAT-1806 commit 42fd3658 (context_only)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a102c716: `run_passed` replaced by `SymbolTally`, which reads only the entries for `binding.symbols` (pass sets passed, fail/error sets failed, skip or unreported counts for nothing; passing = passed and not failed), matching `stale::failed_bound_symbols` and `vacuous_run`. All-skipped or unreported bound symbols raise `class-method-missing` ("did not pass") with the `class` member, so `gate: true` catches it; an unrelated failing test no longer raises it. FR-160 and FR-160-AC-14 reworded. All-skipped also yields the ladder's `vacuous-evidence`; that is two distinct facts (vacuity, class floor), consistent with the existing "asked whatever the ladder found" design, not double reporting of one finding. Re-run on a102c716 via locked-build: tc_160_class_floors 27 passed. |
| FND-002 | fixed | a102c716: tc_plat1836_002 asserts exit `Some(1)` gated and `Some(0)` ungated/off, and `gate.stdout == ungated.stdout == off.stdout` with `class-method-missing` present; tc_plat1836_004 asserts `Some(1)` twice and equal gated/ungated JSON reports. Passed. |
| FND-003 | fixed | a102c716: `enum Kind` (serde kebab-case) with exhaustive matches in `parse_activities` and `build_rows`; the catch-all arm is gone and an unknown kind is refused by serde (`unknown variant`), asserted by `policy_activity_kinds_and_method_declarations_are_closed`. Passed. |
| FND-004 | fixed | a102c716: `design_status` and `relation_status` both fold rows through one `fold_rows`; `design_trace_and_relation_kinds_read_row_states_identically` asserts agreement over covered/uncovered/mixed/other states. quire-rs v0.62.2 `RowState` has only Covered and Uncovered. Passed. |
