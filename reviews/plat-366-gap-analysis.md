---
id: SR-20261111195248
title: "Gap analysis of quoin PR 812 (FR-036 AC-5/AC-7/AC-8 backing)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3a2c4228; spec/functional/FR-036-architecture-conformance.md, spec/matrix.md, rust/crates/quoin-core/src/ops/evidence/tests.rs, rust/crates/quoin-core/tests/tc_2901_process_executables.rs, rust/crates/quoin-core/tests/tc_workspace_dependency_graph_is_acyclic.rs, rust/crates/quoin-evidence/src/adapters/audit_script.rs"
review_set: subset
---

# Gap analysis of quoin PR 812

## Summary

Ticket: PLAT-366. Planless, scoped to the PR diff. Plan completion: not assessed. I ran `quoin
matrix` (quoin 0.30.0) at 3a2c4228. FR-036-AC-2/3/4/5/7/8 are tagged. AC-1 (TC-197), AC-6
(TC-204/205), CON-1 and CON-3 are untagged. CON-2 and CON-4 have the expected
method-without-symbol status. No `criterion-tagged-outside-row` diagnostic appears for FR-036.

The ids TC-2900, TC-202 and TC-203 do not collide with origin/main or with open PRs #810 and #811.
TC-202 and TC-203 are the existing rows on main, rebacked in place. There is no TC-2901 id anywhere.
The matrix rows for TC-2900, TC-202 and TC-203 are present and name the tagged tests. The FR-036
status row now reads "⚠️ Partial" and names TC-197 and TC-204/205 as unbacked, which matches the
computed matrix.

## Verdict

**PASS WITH FINDINGS (one low).** AC-5, AC-7 and AC-8 are now backed by tests that exercise the
production adapter, the resolved cargo graph and the production sources. The weakness of TC-203's
scan is reported in the code-review artifact. The status row cites TC-203 for CON-1, but the test's
marker does not carry CON-1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rewritten FR-036 matrix status row says "CON-1 → TC-203", and FR-036-CON-1 declares `Test (TC-203)`. The TC-203 tests carry only `#[trace("FR-036-AC-8", "TC-203")]`, so the computed matrix reports FR-036-CON-1 as untagged. Fix: add `"FR-036-CON-1"` to both TC-203 markers | spec/matrix.md:104 |
