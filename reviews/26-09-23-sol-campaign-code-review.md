---
id: SR-161
title: "SOL code review — generic measurement campaign"
type: SpecReview
analysis: code-review
scope: "Draft PR 627 at 69bafe0; FR-114 and Rust campaign implementation"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-114"
    type: "reviews"
---

# SR-161: SOL code review — generic measurement campaign

## Summary

Reviewed handwritten run, verify, source, store and CLI campaign code against FR-114, the Quoin Rust idioms, and the Rust review checklist. The implementation uses typed EA campaign contracts and EA's bounded executor. It keeps nonzero process outcomes and retained bytes separate from verdict claims.

## Verdict

**FAIL** pending the high severity identity check below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Collection replay does not bind subject, scope, source revision and verification stack to the campaign member and exact source graph | rust/crates/quoin-measurement/src/campaign/verify/attempt.rs:68 |
| FND-002 | medium | The only non-TL direct producer/checker integration test is Linux-gated, so macOS reports a partial campaign test pass | rust/crates/quoin-measurement/tests/tc_1942_campaign_verification.rs:247 |

## Finding detail

### FND-001

The collection's file bytes and claimed digest are compared, and `rawEvidence` is matched. The other campaign identity fields are not. If a retained run and its collection are altered together, a wrong subject or source claim can survive `verify_retained_campaign` and receive the same accepted plan verdict. Compare all collection identity fields that `publish_collection` emits against the definition, source graph, attempt and producer result, and add retained-byte tamper tests.

### FND-002

The Linux fixture is a useful end-to-end boundary test, but it does not run on this macOS reviewer host. Require its Linux gate before promoting the draft. This is an evidence limit, not a claim that the fixture fails.

## Coverage

Focused gate: Rust 1.98.1 with `TMPDIR=/private/tmp`, `cargo test --offline -p quoin-measurement --features campaign --test tc_1942_campaign_verification`: 1 passed, 0 failed, with the Linux-only direct test absent. Full CLI package authentication was outside this review. No source code edits were made.
