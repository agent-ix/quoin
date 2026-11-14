---
id: SR-20261113203417
title: "Code and Rust review of PLAT-1687 Test Result export"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1687 PR #888: rust/crates/quoin-auditor/src/audit/stale.rs, rust/crates/quoin-cli/src/evidence.rs, rust/crates/quoin-cli/src/evidence/test_result.rs, rust/crates/quoin-cli/src/help.rs, rust/crates/quoin-cli/src/tests.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-cli/tests/tc_test_result_export.rs, rust/crates/quoin-evidence/src/export/mod.rs, rust/crates/quoin-evidence/src/export/test_result.rs, rust/crates/quoin-evidence/src/lib.rs, rust/crates/quoin-evidence/src/source.rs, rust/crates/quoin-evidence/src/stale.rs, rust/crates/quoin-evidence/tests/tc_test_result_export.rs, spec/assurance/MP-250-nfr003-actionable-failures.md, spec/assurance/MP-251-nfr003-silent-failures.md, spec/functional/FR-185-in-toto-test-result-export.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-185"
    type: references
---

## Summary

Ticket PLAT-1687, PR #888. I reviewed the frozen Rust diff, its tests, the new FR-185, and the CLI-to-store path. The focused CLI/evidence tests and formatting check pass; a reproduced duplicate-symbol run can export PASSED despite containing a bound-symbol failure, and two file-error paths do not meet FR-185's stated exit/output behavior.

## Assurance Context

AP-201 is scoped to Quoin's finding scoring, measurement persistence, comparisons, gates, and report rendering; AP-202 is scoped to live evaluation of Jev semantic lenses. Neither profile governs this Test Result exporter, so neither applies to this review.

## Verdict

FAIL — one high-severity incorrect Test Result and two medium-severity contract failures remain.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Duplicate entries for a bound symbol can hide a later fail/error and export the obligation as PASSED. | rust/crates/quoin-evidence/src/stale.rs:13-22; rust/crates/quoin-evidence/src/export/test_result.rs:158-170 | implementation-bug-despite-evidence |
| FND-002 | medium | An unreadable `runs/` directory is treated as an empty directory, so export succeeds with FAILED instead of returning `evidence_store_invalid` (exit 3). | rust/crates/quoin-cli/src/evidence/test_result.rs:81-88 | implementation-bug-despite-evidence |
| FND-003 | medium | A final directory-fsync failure is returned as an output error after the rename has already created or replaced the destination, violating the no-output-on-error contract. | rust/crates/quoin-cli/src/evidence/test_result.rs:105-109 | implementation-bug-despite-evidence |

## Review Evidence

- `cargo fmt --all -- --check` passed.
- `cargo test -p quoin-evidence --test tc_test_result_export -- --test-threads=1` passed (5 tests), run through `locked-build.sh` with the worktree-local target directory.
- `cargo test -p quoin-cli --test tc_test_result_export -- --test-threads=1` passed (6 tests), run through `locked-build.sh` with the worktree-local target directory.
- I reproduced FND-001 with one run containing `test: pass` then `test: fail`; the command exited 0 and emitted `result: PASSED`, `passedTests: [FR-1-AC-1]`, and empty `failedTests`.
- I reproduced FND-002 by making `evidence/runs` mode `000` while keeping bindings readable; the command exited 0 and wrote a FAILED statement instead of reporting an unreadable store.
- FND-003 follows the called publisher's operation order: it renames the temporary to the destination before `fsync_directory`; a failure at that final step returns `Err` after the destination has been published. The CLI maps that error to exit 4.

## Scope Examined

- FR-185-AC-1 through FR-185-AC-8, plus FR-185-CON-1 and FR-185-CON-2.
- All Rust source and test files listed in the frontmatter scope, including the CLI route inventory and the two measurement population edits.
- No CI workflow files changed. The diff does not modify a build or test gate.
