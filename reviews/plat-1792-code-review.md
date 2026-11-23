---
id: SR-20261117175272
title: "Code review — PLAT-1792 declared population and revision binding in test-result export"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-evidence/src/export/provenance.rs, rust/crates/quoin-evidence/src/export/test_result.rs, rust/crates/quoin-cli/src/evidence/test_result.rs, rust/crates/quoin-evidence/tests/tc_test_result_export.rs, rust/crates/quoin-evidence/tests/tc_test_result_provenance.rs, rust/crates/quoin-cli/tests/tc_test_result_export.rs, rust/crates/quoin-evidence/Cargo.toml, rust/crates/quoin-cli/Cargo.toml"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-185", type: references }
---

## Summary

Reviewed agent-ix/quoin PR #1014 (head 5950ff99) for PLAT-1792 with the Rust lane folded in. Ticket: PLAT-1792. The export now takes its required population from the provenance `testMatrix`, binds the subject revision to `sourceRevision`, reads only digest-bound run records, and fails omitted, zero-symbol and missing-run obligations. The disclosed non-verification of the DSSE signature is out of scope for these findings.

## Verdict

**CHANGES REQUESTED.** The empty-graph, omitted-obligation, zero-symbol, missing-run, duplicate-failure, unrelated-binding and revision-mismatch cases all refuse or give FAILED, each with a healthy PASSED control. Removing the revision check, the empty-population check, the zero-symbol rule, the population filter or the run-digest comparison makes a test fail. The `recordPath` traversal guard is sound: only `Component::Normal` parts are accepted, so `..`, a root, a Windows prefix and a leading `.` are all refused, and a symlink inside the store can only yield bytes that must still match the signed digest. One real false-PASSED path remains, through the unbound `bindings.json`. One load-bearing guard has no test. Focused tests (30) pass.

## Review Method

- `/code-review` and `/rust-review`, with the repository `.claude/skills/rust-style/SKILL.md` taking precedence.
- Mutation reasoning: for each new guard, I checked whether removing it fails a test.
- Ran the focused tests on the PR head: `cargo test --offline --locked -p quoin-evidence -p quoin-cli --test tc_test_result_export --test tc_test_result_provenance`, giving 14 + 14 + 2 passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `bindings.json` is not bound by the provenance, and `classify` inspects only the symbols that the binding names. A digest-bound run entry traced to obligation X with outcome fail/error is ignored when its symbol is not in X's binding. Editing bindings to drop the failing symbol, or to re-point X at another suite whose traced entry passes, gives PASSED. | rust/crates/quoin-evidence/src/export/test_result.rs:249-295 |
| FND-002 | medium | The suite-identity guard in `bound_runs` (record suite must equal the provenance suite) has no test. Without it, a record for suite B listed under suite A is keyed as B, which overrides B's signed-null digest and can make B-bound obligations pass. | rust/crates/quoin-cli/src/evidence/test_result.rs:178-185 |
| FND-003 | low | The payloadType and empty-signatures negative controls in tc_1792_011 use `payload: ""`, which fails JSON parsing anyway, so removing either check leaves the test green. The non-empty `signatures` check is also satisfied by `[{}]` and guards nothing while signatures go unverified. | rust/crates/quoin-evidence/tests/tc_test_result_provenance.rs:84-88; rust/crates/quoin-evidence/src/export/provenance.rs:107-109 |
| FND-004 | low | `base64 = "=0.22.1"` is declared inline in two crates rather than once in `[workspace.dependencies]` and consumed as `{ workspace = true }`, as the repository rust-style requires. | rust/crates/quoin-evidence/Cargo.toml:33; rust/crates/quoin-cli/Cargo.toml:64 |
| FND-005 | low | `sourceRevision` and `annotations.revision` are checked only for being non-empty. FR-186 says the value is the full resolved commit, yet tc_1792_103 exports against the revision `"bad"`. A ref name such as `main` on both sides would bind nothing immutable. | rust/crates/quoin-evidence/src/export/provenance.rs:118-122; rust/crates/quoin-evidence/src/export/test_result.rs:161-167 |

## Recommended fixes

- FND-001: in `classify`, mark X failed when any entry in any provenance-bound run has `traceIds` containing X with outcome fail/error, whatever the binding says. Alternatively, have FR-186 sign a digest of `bindings.json` and verify it here. Add a test where bindings omit a failing traced symbol.
- FND-002: add a CLI test where the provenance lists suite A with suite B's record path and digest (and B as null), and assert refusal (exit 3).
- FND-003: build both negative controls from a valid payload so that each check is isolated. Delete the empty-signatures check, or keep only the payloadType check.
- FND-004: add `base64 = "=0.22.1"` to `[workspace.dependencies]` and use `{ workspace = true }` in both crates.
- FND-005: require both revisions to be a full lowercase hex commit (40 or 64 characters), and use a real SHA in tc_1792_103.

## Rust Review

The new types are closed (`deny_unknown_fields` on the envelope, payload and run), the errors are `thiserror` enums, and the pure/IO split is respected: `quoin-evidence::export` stays pure and the CLI does the file reads. No `unwrap` is used in production code. The CLI reads the store's latest runs only to refuse an unreadable store and discards the result, which is acceptable. Input reads are unbounded `fs::read` of local files, which matches the existing store readers.
