---
id: SR-20261123115002
title: "Code review — PLAT-1796 drop old-v1 release-snapshot reader (quoin#1023)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier/src/release_snapshot.rs, rust/crates/quoin-dossier/src/release_record.rs, rust/crates/quoin-soup/src/soup/report/snapshot.rs, rust/crates/quoin-cli/src/release.rs, rust/crates/quoin-cli/src/release/tests.rs, rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1796. Code review with the Rust lane folded in, of quoin PR #1023 at head
bbbf48096d95c1401654513cd756f89c56821d9a, scoped to the PR diff. quoin 0.36.8;
spec-artifacts-process v0.36.0.

The compatibility reader is gone: `tool_identity` and `clean_tree_precondition` are plain
required fields (the `Option` wrappers and `deserialize_non_null_option` are deleted), the
writer builds them unconditionally, and the reader refuses a `scan_unordered` kind through
an allow-list. No fallback, shim or legacy branch remains; `git grep scan_unordered` finds
only the refusal test. Every snapshot fixture in the tree (`release/tests.rs`,
`release_record.rs` tests, three `tc_1614` fixtures) was updated; no other consumer reads the
snapshot.

The extra change in `quoin-soup/src/soup/report/snapshot.rs` (an unordered scan fails the
snapshot instead of emitting `scan_unordered`) is justified: FR-167 on `main` (from #1020)
already stated that `scan_unordered` "is not emitted by the current command because an
invalid retained timestamp refuses the snapshot", while `main`'s code emitted it. The PR
brings the code into line with that statement. `soup_section` has one caller
(`quoin-cli/src/release.rs`), so no other consumer changes behaviour; `quoin soup report`
keeps its own unordered rendering. The refusal is classified as an operational failure,
which is FND-001.

## Verdict

**PASS WITH FINDINGS** — no compatibility path remains and the reader contract matches
FR-167. FND-001 (medium) should be fixed in this PR.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | An unordered retained scan maps to `SoupFailure::Failed`, so `quoin release snapshot` exits 1 as an operational failure with no code, not a refusal (exit 2, `release_snapshot_input_invalid` naming `--repo`) as FR-166 Outputs require and as the sibling unreadable-scan path does. | rust/crates/quoin-soup/src/soup/report/snapshot.rs:294-298; rust/crates/quoin-cli/src/release.rs:324 |
| FND-002 | low      | `NotAssessed.kind` stays a `String`; the kind vocabulary now lives in three places (writer literals, reader `NOT_ASSESSED_KINDS`, and a hand-typed error message), and the `parse` `# Errors` doc omits the new refusals. A closed serde enum would refuse unknown kinds at deserialization with one vocabulary. | rust/crates/quoin-dossier/src/release_snapshot.rs:67-106; rust/crates/quoin-soup/src/soup/report/snapshot.rs:124-131 |
