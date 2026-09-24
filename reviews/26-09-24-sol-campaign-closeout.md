---
id: SR-163
title: "SOL closeout — generic campaign code, Rust and gap review"
type: SpecReview
analysis: gap-analysis
scope: "FR-114; PLAT-1043; draft PR #627 at dc3d6ab after rebase onto main 40bcffe"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-114"
    type: "reviews"
---

# SR-163: SOL closeout — generic campaign code, Rust and gap review

## Summary

An independent SOL agent reviewed the full campaign diff with the code-review,
rust-review and gap-analysis checklists. The final pass found no high or medium
code or Rust defect. The earlier checkpoint finding is closed: resume rejects
an `InvalidRequest` prefix that has no EA result identity, and a Linux test
plants that exact checkpoint before retry. Unix CLI cancellation is exercised
by SIGINT and retained replay; Windows keeps the portable library run path.

The matrix backs TC-1940 through TC-1947 with tagged tests. TC-1941 remains
partial for direct `MalformedResponse` and `ContainmentFailure` integration
fixtures. The generic direct-process adapter accepts every normal untruncated
exit, and EA classifies truncated output as `Failed` before adapter decoding;
`ContainmentFailure` needs an EA kernel or descendant-containment failure.
Neither state was fabricated to make the row appear complete.

## Verdict

**CONDITIONAL** — the implementation has no open high or medium review defect,
but the TC-1941 direct terminal integration row remains partial. This review
does not claim a full 120-member TL campaign acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1941 has no honest direct Quoin fixture for `MalformedResponse` or `ContainmentFailure`; exhaustive typed mapping and EA behavior are present, but the integration row stays partial. | spec/matrix.md; rust/crates/quoin-measurement/src/campaign/run/execution.rs |
| FND-002 | low | Closed: a planted evidence-free `InvalidRequest` checkpoint could suppress an invocation on resume; validation now refuses that prefix before reuse. | rust/crates/quoin-measurement/src/campaign/run/checkpoint.rs; rust/crates/quoin-measurement/tests/tc_1942_campaign_verification.rs |

## Coverage

`make test` on Rust 1.98.1 passed locked package install, format, strict
all-target/all-feature Clippy, cargo deny and the full workspace test suite at
`/private/tmp/quoin-pr627-make-test-rebased.log`. In a local network-disabled
native ARM64 Linux container, 19 TC-1942/TC-1940/TC-1941/TC-1944 cases,
three TC-1947/origin cases and two native TC-1946 CLI cases passed. The CLI
SIGINT case asserts a Cancelled EA attempt with minted request/result
identities, an Inconclusive document and identical independent replay.

The 21 campaign commits rebased cleanly onto main `40bcffe`; `git range-diff`
found all 21 equivalent and `git diff --check` passed. No CI workflow changed.
The optional formal semantic gap-analysis pass was not invoked; the independent
SOL code/Rust review did inspect the requirement, test and implementation
agreement for the modified campaign path. The local repository has no
FR-114 plan bundle under `plan/`; delivery is tracked in Linear PLAT-1043.
