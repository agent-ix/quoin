---
id: SR-20261117175262
title: "Gap analysis of Quoin PLAT-1790 actor admission"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR #1004 changed files and actor-admission criteria FR-116-AC-18, FR-153-AC-16, FR-153-AC-18"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-153", type: references }
---

## Summary

Repository-driven gap analysis for the PLAT-1790 actor-admission paths compared the relevant requirements, computed Quoin Test Matrix, tagged tests, and changed implementation. The newly added criteria are tagged, but the positive and refusal integration tests that invoke the external EA verifier cannot run reliably in the current Quoin test workflow; plan completion was not assessed.

## Verdict

**FAIL** — the changed real-verifier test path fails under the available EA 0.8.1 executable, and the Quoin test workflow does not provision the required EA CLI.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-153 actor-admission test fixture discovers `engineering-assurance` from ambient PATH and runs `signature-verify`, but Quoin's test workflow does not install that CLI. With the available 0.8.1 binary, the focused unsigned-refusal test fails because the command is unknown; in a runner without any EA binary, the fixture panics at its `expect`. Make the ordinary test lane pass without the external binary and reserve real CLI integration for the authorized manual workflow, or identify a supported executable distribution. No `build-test.yml` change is authorized by this review. | rust/crates/quoin-cli/src/analysis_evidence.rs::signed_pair_repo; .github/workflows/build-test.yml |

## Coverage

- Reconciliation: `quoin matrix` (quoin 0.36.8, quire matrix embedded; no run evidence read).
- Relevant criteria: FR-116-AC-18 tagged; FR-153-AC-16 tagged; FR-153-AC-18 tagged. The engine reports comment-only criterion IDs on some test doc comments as `unmarked-trace-id`; the corresponding tests also have `#[trace(...)]` attributes, so those are not untagged criteria.
- Whole-matrix context: 2,214 criteria total: 1,402 tagged, 625 untagged, 146 method-without-symbol, 5 tagged only by ignored tests, and 36 retired. These repository-wide totals are not attributed to this PR; this artifact's source/test comparison is scoped to the changed actor-admission area.
- Reverse-gap/stub inventory: inspected all nine changed files and the CLI test/config boundary; no other PR-introduced unowned user-visible behavior or stub found in that bounded surface.
- Plan completion: not assessed
- Semantic review: skipped
