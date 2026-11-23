---
id: SR-17093
title: "Gap analysis of PLAT-1839 module lag warning"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-211-module-lag-warning.md, rust/crates/quoin-modules/src/lag.rs, rust/crates/quoin-cli/src/module.rs, rust/crates/quoin-cli/src/catalog.rs, rust/crates/quoin-cli/tests/tc_1839_module_lag.rs; PR agent-ix/quoin#1027"
review_set: subset
---

## Summary

Ticket: PLAT-1839. Planless gap analysis scoped to the PR diff. Plan completion: not assessed.

`quoin matrix --json` on the branch (quoin 0.36.8) shows FR-211-AC-1, AC-2 and AC-3 each `static_status: tagged`: AC-1 by `doctor_warns_naming_module_installed_and_pinned_version_and_exits_zero` and `tests::an_older_installed_release_is_reported_with_both_versions`; AC-2 by `doctor_is_silent_when_every_module_is_at_its_pin` and `tests::an_equal_newer_missing_or_unversioned_install_is_not_reported`; AC-3 by the doctor test and `catalog_validate_warns_without_failing`. No stubs, `todo!()` or placeholder returns in the changed code. Production code added by the PR (`lagging_modules`, `lag_diagnostics`, `doctor`, the catalog hook) all has an owning requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-211 Behavior states "`quoin module doctor` SHALL report the registry state", but no AC or test covers it and `doctor()` renders only a one-line summary, so the statement is neither verified nor clearly implemented. | spec/functional/FR-211-module-lag-warning.md:39-40, rust/crates/quoin-cli/src/module.rs:121-136 |
| FND-002 | low | The unit test tagged FR-211-AC-2 also asserts the not-installed and non-semver (`main`, no ref) cases, which AC-2 ("at or above its pin") does not state; those Behavior rules have no criterion of their own, so the tag overstates AC-2 and the skip rules are untraced. | rust/crates/quoin-modules/src/lag.rs:89-98 |

## Verdict

Coverage of the three stated criteria is complete and the bindings are correct. The two findings are traceability gaps in Behavior statements that have no criterion.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-212-AC-7 claims a failed check "does not change the exit status of `quoin catalog validate`", but its only binder is the unit test of `lag_diagnostics_or_warning`; no test drives `catalog validate` with an unreadable module listing and asserts the exit status. | rust/crates/quoin-cli/src/module.rs:566-580, spec/functional/FR-212-module-lag-warning.md |

## Dispositions

Round 1, head 35474e19. `quoin matrix --json`: FR-212-AC-1..AC-7 all `tagged`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The doctor statement is split into SHALLs for warnings, summary line and exit status; FR-212-AC-5 covers the summary and `doctor_warns_for_an_older_module_summarises_and_exits_zero` asserts it. |
| FND-002 | fixed | Skip and incomparable rules now have their own criteria (AC-3, AC-4) and tests; the AC-2 test asserts only at-or-above cases. |
| FND-003 | fixed | Round 2 (20bafb66): `validate` delegates to `with_lag_warnings(response, json, lag)`, and `a_failed_lag_check_leaves_the_validate_outcome_unchanged` asserts that a failed check leaves the outcome (`Outcome::Ok`, from which the exit status is derived) and payload equal to a clean run and adds one QM010 warning. A run-level test cannot reach this path: measured, a corrupt registry makes `catalog validate` exit 1 earlier in `modules.ensure_defaults` (QM010). |
