---
id: SR-160
title: "SOL gap analysis — generic measurement campaign"
type: SpecReview
analysis: gap-analysis
scope: "FR-114; quoin-measurement campaign run, verify, source and store; PLAT-1043"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-114"
    type: "reviews"
---

# SR-160: SOL gap analysis — generic measurement campaign

## Summary

Independent review of draft PR #627 at 69bafe0. The generic campaign implementation uses the EA executor and generated contracts, retains request and result identities, and has a non-TL fixture. The targeted plan and matrix do not yet trace FR-114's seven named test cases. The retained collection replay omits identity fields required by FR-114-AC-3.

## Verdict

**FAIL** — a high severity collection identity gap and missing matrix coverage remain. The draft's Linux direct-process fixture and full CLI package auth are pending gates, not independently observed failures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Retained collection replay accepts a wrong subject, scope, source revision or verification stack when the collection and attempt digests are recomputed | rust/crates/quoin-measurement/src/campaign/verify/attempt.rs:68 |
| FND-002 | medium | FR-114 cites TC-1940 through TC-1946, but spec/matrix.md has no rows for them and no targeted implementation plan | spec/functional/FR-114-generic-measurement-campaign.md:37 |
| FND-003 | medium | The non-TL producer and checker fixture is Linux-only, leaving the local macOS gate to run only the missing-member/source-tamper case | rust/crates/quoin-measurement/tests/tc_1942_campaign_verification.rs:247 |

## Finding detail

### FND-001

`check_attempt` compares the collection ID and the existence of an observation with the member's plan ID and definition version, then compares the `rawEvidence` object. It never compares `subject` to `definition.subject_name`, `scope` to the campaign/member/run/attempt, or `sourceRevision` and `verificationStack` sources to the exact source graph. A retained collection with one of these claims changed, written under its new byte digest with the attempt's `collectionDigest` updated, still reaches the same plan-verdict recomputation. Add explicit comparisons and a tamper test for each identity before granting acceptance.

### FND-002

The requirement cites seven test cases, while the matrix has no TC-1940..TC-1946 entries. The existing Rust test names cover 1940, 1941, 1942 and 1945; 1943, 1944 and 1946 are not tagged. This is a tracking and closure gap even where behavior has some test coverage. No implementation plan for FR-114 was found in `plan/`.

### FND-003

The direct-process test is under `#[cfg(target_os = "linux")]`. Its gate must run on Linux for a non-TL end-to-end claim. On macOS the feature-gated test command reports one passed test, which covers missing members and dirty source refusal only.

## Coverage

Focused local checks used Rust 1.98.1 and `TMPDIR=/private/tmp`: `cargo test --offline -p quoin-measurement --features campaign --test tc_1942_campaign_verification` passed 1 test on macOS. The full Quoin CLI package-auth gate and Linux direct-process fixture were not run in this review. No product code was edited. Optional semantic gap review was not invoked as a formal skill step; the code finding above came from the requested code review.
