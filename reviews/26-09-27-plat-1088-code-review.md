---
id: SR-170
title: "PLAT-1088 code review — campaign test lint under --all-features"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@d6d3d60bcae21619c042a5b2757befbfe98d29ff; rust/crates/quoin-cli/tests/tc_1946_campaign_cli.rs, rust/crates/quoin-measurement/src/campaign/run/domain.rs, rust/crates/quoin-measurement/src/campaign/source.rs, rust/crates/quoin-measurement/tests/tc_1942_campaign_verification.rs, rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs"
review_set: subset
---

# SR-170: PLAT-1088 code review — campaign test lint under --all-features

## Summary

Ticket: PLAT-1088. PR: agent-ix/quoin#648. This review covers the code-review
pass, with the rust-review lane folded in. The diff is lint-only: it adds
`#[allow(..., reason = ...)]` attributes to test code, fixes doc_markdown
backticks in two test doc comments, and adds digit separators to two integer
literals inside a `serde_json::json!` macro. The literal values are unchanged.

Every added `#[allow]` is on test code: an integration-test crate root under
`tests/`, a test fn or test helper under `tests/`, or a `#[cfg(test)] mod tests`.
None is on production code, and every one carries a `reason =`. The unit-module
allows use the reason text documented in `.claude/skills/rust-style/SKILL.md`
verbatim. The function-level `too_many_lines` and `too_many_arguments` allows
follow precedent that already exists on main, including in
`tc_1946_campaign_cli.rs:81` and `tc_1942_campaign_verification.rs`. The
`too_many_arguments` allow is on a 7-parameter test helper, and
`clippy.toml` sets `too-many-arguments-threshold = 6`, so the lint really fires.
No CI workflow, `[workspace.lints]`, `clippy.toml` or `deny.toml` changed.

The coder said `plans.rs::load_selected_measurement_plans` is not dead code.
This review verified that claim for the gate configuration: under
`--workspace --all-features` it is called from `campaign/run/mod.rs:262` and
`campaign/verify/mod.rs:132`. It is still reported dead in a default-feature
build of the crate alone. See FND-002, which is pre-existing.

## Verdict

**PASS.** There are no behaviour changes and no production-code allows.
`make rust-lint` exits 0 at d6d3d60. Both findings are low severity, and
neither blocks the merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Unit-test allows widen to expect_used/panic, but the rust-style skill's documented allow set and its "never widen without adding the reason here" rule were not updated | .claude/skills/rust-style/SKILL.md:130 |
| FND-002 | low | Pre-existing: load_selected_measurement_plans is only called from campaign-gated modules but is not itself cfg-gated, so a default-feature clippy of quoin-measurement fails dead_code | rust/crates/quoin-measurement/src/plans.rs:122 |

## Finding detail

### FND-001

`.claude/skills/rust-style/SKILL.md:119-132` documents the test-module allow
set as `unwrap_used, indexing_slicing`. It says: "Never widen this to
`expect_used` or `panic` without adding the reason here too." This PR adds
`clippy::expect_used` at `source.rs:409` and `run/domain.rs:463`, and
`clippy::panic` at `tc_1946_campaign_cli.rs:8`. It does not touch the skill.

Main already has about 40 test-code sites that allow `expect_used` and
`panic`, so the code follows real practice and the doc is what is stale. This
PR did not introduce that gap. The failure scenario is procedural: the next
reviewer who applies the documented rule literally flags these allows again.
The fix is a single line: add `expect_used` and `panic` to the documented set
in the skill.

### FND-002

`campaign.rs:14-27` gates `run` and `verify` behind `#[cfg(feature = "campaign")]`.
`plans.rs:122`, the `pub(crate) fn load_selected_measurement_plans`, is not
gated. Running `cargo clippy -p quoin-measurement --lib -- -D warnings` at
d6d3d60 fails with `function load_selected_measurement_plans is never used`.

The gate runs `--workspace --all-features`, and `quoin-cli` enables
`campaign`, so no current CI lane reaches this configuration. The coder's
"not dead" claim is therefore true for the gate. The problem is latent for a
per-crate or default-feature lane. `plans.rs` was last changed on main by
99d4dae and is untouched here, so this is pre-existing and outside this PR's
scope. The fix is to add `#[cfg(feature = "campaign")]` on the fn and its test.

## Coverage

- `git diff origin/main...HEAD`: 5 files, +46/-5, one commit (d6d3d60).
- `make rust-lint` (`cargo fmt --all --check`, then `cargo clippy --workspace
  --all-targets --all-features --locked -- -D warnings`) passed with exit 0.
- `cargo clippy -p quoin-measurement --lib --locked -- -D warnings` (default
  features) failed with one dead_code error, recorded as FND-002.
- Checked: allow placement (test-only), reason text against the rust-style
  skill, main precedent for `too_many_lines` and `too_many_arguments`, the
  `clippy.toml` threshold, literal values inside `json!`, doc-comment-only
  doc_markdown edits, and the CI workflows (unchanged).
- Not run by the reviewer: `make rust-test` and `rust-deny`. The change
  touches no executable statements. The coder's log reports `make test`
  exit 0 at d6d3d60.
- Gap analysis was not run because the change has no behaviour. No source
  edits were made.
