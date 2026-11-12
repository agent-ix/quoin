---
id: SR-20261112014600
title: "Code review of quoin PR 822 (PLAT-1457 report decides paired-run plans)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@f847c866; rust/crates/quoin-cli/src/{intake_order.rs,report.rs,measurement.rs,measurement/source_state.rs,measurement/tamper.rs}, rust/crates/quoin-core/src/ops/measurement/{report.rs,verify.rs,wire.rs,mod.rs}, rust/crates/quoin-measurement/src/{lib.rs,report/build.rs,report/mod.rs,verify/mod.rs,verify/order.rs,verify/paired.rs,verify/types.rs}, rust/crates/quoin-measurement/tests/tc_3600_report_order.rs, rust/crates/quoin-cli/tests/tc_3513_paired_verify_command.rs"
review_set: subset
---

# Code review of quoin PR 822

## Summary

Ticket: PLAT-1457. Code review with the rust-review lane (repo `.claude/skills/rust-style`
loaded). PR head f847c866 on base 32e032c3; origin/main is at c694a2d6 (#820, #821), with
no file overlap and a clean three-way merge. PR #819 also merges cleanly with this one.
Builds ran through `locked-build.sh` with `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4` and
debug=0. `cargo test -p quoin-measurement -p quoin-core -p quoin-cli` passed (821 passed, 0
failed), including:

- TC-3600..3606 and TC-3500..3516
- the report golden tests
- the module-size tests (`tc_90`, `tc_468`)
- the TC-203 executable scan (`quoin_executes_exactly_git_and_ix_flow_and_the_operators_editor`)

`cargo clippy` on the same three crates with `--all-targets --all-features -D warnings`
passed.

A throwaway probe test, not pushed, reproduced FND-001. Two collections under `equal` with
equal values and `OrderSource::GitShallow`: `verify` returns `Inconclusive [OrderUnattested]`
with a paired count of 0, and the report row is `Observed { differing: 0 }` with no reasons.

The intake-order function moved verbatim into `quoin-cli/src/intake_order.rs`. It starts
exactly three git processes (two `rev-parse` and one `log`) per `quoin report`. The `log` is
one first-parent pass limited to the measurements pathspec, the same pass `verify` makes. The
process is spelled `std::process::Command::new("git")`, which the TC-203 executable scan
recognises as `git`.

## Verdict

**Not mergeable. One high and two medium findings must be fixed in this PR.**

- **FND-001 (high).** In a shallow clone, two collections under `equal`/`within_delta` get
  different answers from verify and report. That breaks the invariant the PR exists to
  establish. GitHub Actions' default checkout (`fetch-depth: 1`) is a shallow clone, so CI
  hits this case routinely. Rule: keep FR-108-AC-8, so a shallow clone is `order_unattested`
  for every plan, and have `report_pair` add `order_unattested` whenever the source is
  `GitShallow` and the pair is not `unpaired`. Put that decision inside `judge` (pass it the
  `OrderSource`) so there is still one function. Amend FR-132's bullet and AC-12 ("two
  collections ... need no order, except in a shallow clone, FR-108-AC-8"), and add the
  two-collection shallow case to TC-3602.
- **FND-002 (medium).** The report request now carries the whole intake order but is still
  bounded at 1 MiB, while `verify` takes 4 MiB for the same order.
- **FND-003 (medium).** The old no-order builders are default-argument shims. Each one is a
  second public entry point that silently attests no order. Remove them.
- **FND-004 (low).** `quoin report` drops both the git error and the warning. Rendering with
  no order is acceptable for a read-only report, because the row stays honest (`inconclusive`,
  no count). The cause should still reach stderr through the payload `warning` that `verify`
  already uses.
- **FND-005 (low).** The positions filter is duplicated.

What holds:

- `judge` is the single place the pair is chosen and its order attested. `report_pair` maps
  its findings.
- `ReportRequest` keeps `deny_unknown_fields`.
- The non-paired report output is unchanged. The report, golden and module-size tests pass.
- No module is near the 700-line cap. The largest touched module is `verify/mod.rs` at 574.
- `--portfolio` keeps `order_unattested` and reads no git, as FR-132 now states. That is
  acceptable: portfolio roots are arbitrary directories, and whether to attest order across
  several repositories is a separate decision.
- The git edge cases behave as before the move:
  - git absent or `PATH` empty: the report renders unattested (TC-3605).
  - Not a work tree: `none`.
  - Detached HEAD and worktrees: `rev-parse` and `log` work unchanged.
  - Plan in a subdirectory: `-C repo` with `--relative` and a repo-relative pathspec.
  - Rewritten history: the order follows the history this clone presents, as `tamper.rs`
    documents.
  - Submodule: `rev-parse` answers for the submodule's own repository.
- An unborn branch (`git init`, no commits) makes `git log` fail. `verify` errors on that and
  the report renders with no order. This predates the PR and only moves with the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | In a shallow clone, verify and report disagree for two collections under `equal`/`within_delta`. `verify` always adds a plan-level `order_unattested` for `OrderSource::GitShallow`, so the verdict is inconclusive. `report_pair` drops the positions and calls `judge`, which adds nothing because `order_matters` is false, so the report states `observed` with a count. Reproduced: verify `Inconclusive [OrderUnattested]` count 0, report `Observed { differing: 0 }`. FR-132-AC-12 claims the two decide a pair alike. Fix: `judge` takes the `OrderSource` and adds `order_unattested` for `GitShallow` whenever the pair is not unpaired, and verify's separate plan-level shallow push is dropped for paired plans so the finding is not duplicated. Amend FR-132 text and add a two-collection shallow case to TC-3602 | rust/crates/quoin-measurement/src/verify/mod.rs:144-146, rust/crates/quoin-measurement/src/verify/paired.rs:68-75, rust/crates/quoin-measurement/src/verify/paired.rs:90-118 |
| FND-002 | medium | `measurement.build_report`/`render_report` now carry the full intake order (one group per first-add commit, growing with the store), but `report_of` still bounds the request at `MAX_ASSURANCE_DOCUMENT_BYTES` (1 MiB). `verify` bounds the same order at `MAX_VERIFY_REQUEST_BYTES` (4 MiB) for exactly this reason. A store whose order serialises above 1 MiB (about 20k collections at about 50 bytes each) makes `quoin report` refused outright while `verify` still answers. Bound `ReportRequest` at `MAX_VERIFY_REQUEST_BYTES`, and fix the `MAX_ASSURANCE_DOCUMENT_BYTES` doc, which still says it bounds "the request that locates them" for build_report/render_report | rust/crates/quoin-core/src/ops/measurement/report.rs:92, rust/crates/quoin-core/src/ops/measurement/wire.rs:90-97 |
| FND-003 | medium | `build_measurement_report(source, repo)` and `build_measurement_report_from(...)` are kept as wrappers that default to `IntakeOrder::none()`, beside the new `_ordered` variants. `build_measurement_report` has no production caller, only tests. `build_measurement_report_from` has one, in `portfolio/build.rs:160`. These are default-argument shims, against the no-compat rule, and they are a trap: a new caller that picks the short name silently attests no order and gets `order_unattested` with no signal. Keep one function per builder that takes `&IntakeOrder`, under the original names. The portfolio and the tests pass `&IntakeOrder::none()` explicitly. Drop the `_ordered` names | rust/crates/quoin-measurement/src/report/build.rs:132-136, rust/crates/quoin-measurement/src/report/build.rs:180-196, rust/crates/quoin-measurement/src/portfolio/build.rs:160 |
| FND-004 | low | `quoin report` discards the `Err` from `intake_order` (`Err(_) => json!({repo})`) and the `Ok` case's `warning` (not a work tree, shallow clone). The operator then sees `order_unattested` with no cause, while `verify` reports the same condition on stderr through the payload `warning`. Rendering without an order is correct for a read-only view. Pass the warning or error text through the same `warning` payload member so it reaches stderr | rust/crates/quoin-cli/src/report.rs:90-97 |
| FND-005 | low | The "ignore positions unless the source attests them" filter (`intake.filter(\|_\| order.attests_positions())`) is written twice, in `verify` and in `report_pair`. Moving it into `IntakeOrder::rank`, so a non-attesting source yields no positions, makes the shared-function claim hold for positions too and removes one way for the two to drift | rust/crates/quoin-measurement/src/verify/mod.rs:126-133, rust/crates/quoin-measurement/src/verify/paired.rs:95-102 |
