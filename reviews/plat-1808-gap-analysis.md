---
id: SR-20261123202515
title: "Gap analysis of release snapshot risk and anomaly-gate open findings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR #1035 (PLAT-1808) diff origin/main...HEAD against spec/functional/FR-166-release-snapshot-command.md and spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-166"
    type: references
  - target: "ix://agent-ix/quoin/FR-167"
    type: references
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1035). Planless gap analysis over the criteria this PR implements:
FR-166-AC-12, FR-166-AC-13, FR-166-AC-14 and FR-167-AC-15 to FR-167-AC-19, plus FR-167-AC-7, whose test the PR edits.
`quire matrix --scope . --format json` shows every one of them bound to a trace-tagged test in
`rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs`; no criterion in scope is untagged. Each tagged test runs
and passes, and each was shown to fail under a targeted mutation of the code it covers. No stubs. No reverse gap: every
added production item serves FR-166 or FR-167.

The gaps are in the depth of the tests and in one criterion that cannot be met. The coder asked about two of them.
FR-167-AC-18's "an anomaly that fails both modes" describes an anomaly that cannot exist. The code is right and the
criterion needs an amendment (FND-001). FR-167-AC-16 is only half tested (FND-002).

## Verdict

**CONDITIONAL**: medium and low findings only. FND-001 needs a spec amendment. FND-002 and FND-003 need a test
each.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-167-AC-18's "an anomaly that fails both modes yields both items" cannot be met, and the same holds for the Description's "an anomaly that fails both modes yields two items". Mode `unassessed` needs `safety_impact` `unassessed`, mode `unjustified-impacting` needs `impacting`, and FR-164 states that an `unassessed` anomaly SHALL NOT fail `unjustified-impacting`. The code is right. Amend FR-167 to drop both clauses, or to say "each gate failure is one item; an anomaly fails at most one mode". | spec/functional/FR-167-release-snapshot-format.md; FR-167-AC-18; FR-164 |
| FND-002 | medium | No test covers FR-167-AC-16's second clause, "a complete chain yields no item of any FR-170 token". The tagged test only removes the `control-without-verification` item. Add a full hazard, situation, harm, estimate, control, verification, residual-risk and benefit-risk fixture and assert that no FR-170 token appears. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1726; FR-167-AC-16 |
| FND-003 | medium | No test covers FR-167-AC-17's second clause: an `accepted-outside-acceptable-region` finding that names a residual risk and its benefit-risk ids, giving one item whose `obligation` is the residual-risk id and whose `summary` lists every name in order. As a result, the `names.first()` and `names.join(", ")` mapping never runs with more than one name. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1709; FR-167-AC-17 |
| FND-004 | low | Neither FR-167-AC-15 nor FR-166-AC-12 is checked against the other command. Neither test runs `quoin risk-summary` or `quoin anomaly report --fail-on unassessed` over the same fixture to compare tokens and failures. The FR-166-AC-12 test also never asserts the `control-without-verification` item. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1742; FR-166-AC-12; FR-167-AC-15 |
| FND-005 | low | The FR-166-AC-14 test leaves out the "risk record file" case and uses only a `docs/` non-UTF-8 file and an invalid-YAML file. It has no valid-but-empty frontmatter block either; that fixture would have caught code-review FND-001 (SR-20261123202514). | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1867; FR-166-AC-14 |
| FND-006 | low | The FR-167-AC-19 test checks the order only on (`obligation`, `kind`). Over the mixed-source fixture it does not check the closed item keys or the absence of severity, score and acceptance words. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1742; FR-167-AC-19 |

## Coverage

- Criteria in scope: 9; tagged: 9; untagged: 0.
- Mutations: 6 applied, 6 killed, all reverted.
- Plan completion: not assessed
- Semantic review: skipped (not requested).

## Dispositions

Round 1 (fix commit 4b3fc242). I checked every finding against the fix commit. The FR-167-AC-17 test uses
`benefit-risk-not-accepted` (names `RR-2, BR-1`) in place of `accepted-outside-acceptable-region`. That substitution
is the only way to test the clause's intent: `token_for` in quoin-risk-advice emits
`accepted-outside-acceptable-region` only when no benefit-risk record covers the residual risk, so that token can
never name covering benefit-risk ids. The test now covers the multi-name mapping, but the AC text names a token
that cannot take that shape (FND-007).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | No test can meet FR-167-AC-17's second clause as written. `accepted-outside-acceptable-region` is emitted only when no benefit-risk record covers the residual risk (FR-170-AC-7; `token_for` in quoin-risk-advice), so its `names` hold the residual-risk id alone and never "its covering benefit-risk ids". Amend the clause to name `benefit-risk-not-accepted` (or `benefit-risk-undecided`), which the round-1 test already exercises. FR-170's table row for the token, which lists covering ids, is out of this PR's scope. | spec/functional/FR-167-release-snapshot-format.md; FR-167-AC-17; FR-170 |
