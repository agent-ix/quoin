---
id: SR-20261116075203
title: "Code review — PLAT-1760 PR #945 release snapshot fixture"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs; PLAT-1760; PR #945"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-032", type: references }
  - { target: "ix://agent-ix/quoin/FR-115", type: references }
  - { target: "ix://agent-ix/quoin/FR-167", type: references }
---

## Summary

Reviewed PR #945's release snapshot fixture changes, including the cause member in the closed shape and the new passing setup run. The fix makes AC-3 genuinely bound before a failing result is stored, but the two run records share a timestamp, so the fixture does not test the chronological newest-run case it claims to exercise.

## Verdict

**CONDITIONAL** — one medium test-intent finding remains.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies because the fixture checks release-snapshot rendering of an evidence finding. Its `review_policy.mode` is `recommend`; this review ran as requested and is not a human-signoff gate. The fixture now asserts that both stale causes are present, but the timestamp mismatch below leaves the latest-run evidence case weaker than stated. No selected architecture, measurement plan, independence evidence, producer-reliance record, or active exception is in scope for this test-only follow-up. AP-202 does not apply; this change does not evaluate a Jev lens.

## Rust Review

The diff changes only an integration test. The added fixture invokes the real `quoin evidence record` command, preserves the binding created by the passing result, and asserts the exact `open_findings` cause values. The closed-shape update correctly makes only `cause` optional. No production code, unsafe code, public API, dependency, or workflow changes were present. The reviewer did not run the aggregate gate before the lead's post-review gate checkpoint; the coder reports 24 focused snapshot tests, formatting, and scoped Clippy passed through `locked-build.sh`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Both the passing setup and failing result use timestamp `2026-01-01T00:00:00Z`. The evidence store is last-write-wins for the same suite and timestamp, so this replaces one record rather than testing a strictly newer failed run as the helper comment and FR-032-AC-19 scenario claim. Give the failing result a later timestamp. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1470-1517 | correct-requirement-no-evidence |

## Coverage

- Examined: FR-032-AC-19, FR-167-AC-10, FR-167-AC-11, and AP-201; the test↔AC binding for `tc_1760_snapshot_carries_stale_causes_byte_for_byte` is correct for FR-167-AC-10.
- Code-review and Rust review were run together per the reviewer workflow. No code or test execution was performed by the reviewer; the aggregate gate remains for the lead after review.
