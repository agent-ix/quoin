---
id: SR-20261123115003
title: "Gap analysis — PLAT-1796 drop old-v1 release-snapshot reader (quoin#1023)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-167-release-snapshot-format.md, rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs, rust/crates/quoin-soup/src/soup/report/snapshot.rs, rust/crates/quoin-dossier/src/release_snapshot.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1796. Planless gap analysis scoped to the PR #1023 diff at
bbbf48096d95c1401654513cd756f89c56821d9a. Plan completion: not assessed.

FR-167-AC-13 is tagged by two tests: `one_file_is_written_with_exactly_the_named_members_and_nothing_else_changes`
(writer: exact `tool_identity` and `not_verified`) and
`the_snapshot_reader_requires_both_members_and_refuses_dirty_or_unordered_values` (reader).
The reader test starts from a healthy full snapshot that parses and changes one member per
case: missing `tool_identity`, missing `clean_tree_precondition`, a `scan_unordered` item,
`null` for either, an unknown member, another tool name, a blank version, and `verified`. All
AC-13 refusal clauses have a case. No stubs or untraced production code were found in the
diff. A focused run of `cargo test -p quoin-cli --test tc_3604_release_snapshot` (through locked-build.sh, debug=0) passed: 25 passed, 0 failed.

## Verdict

**PASS WITH FINDINGS** — AC-13 is covered; the writer-side unordered refusal added in this PR
has no test, and the kind allow-list has no positive control.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | The new writer behaviour (an unordered retained scan stops `quoin release snapshot` and writes no file) has no test; no test drives `soup_section` with an unordered scan, and no AC states it. tc_3604:1339 covers only the undecodable-file refusal; tc_1518 reaches Unordered only through `quoin soup report`. | rust/crates/quoin-soup/src/soup/report/snapshot.rs:270-298 |
| FND-002 | medium   | The reader's `not_assessed` kind allow-list has no healthy control with a non-empty list of valid kinds: the control has `not_assessed: []`, so a mutation that refuses every non-empty list still passes. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:692-754; rust/crates/quoin-dossier/src/release_snapshot.rs:85-95 |
| FND-003 | low      | Every reader refusal is asserted with `is_err()` only, not the specific error, so a case can pass for an unrelated reason. The one-change-from-a-healthy-control design limits this. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:712-754 |
