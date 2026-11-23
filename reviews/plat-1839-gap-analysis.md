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
