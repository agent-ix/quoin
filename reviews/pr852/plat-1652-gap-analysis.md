---
id: SR-20261113073534
title: "Gap analysis of quoin risk advise (PR 852, FR-145 to FR-148, NFR-031)"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quoin; PR #852 (git diff origin/main...HEAD at 03fe9207): spec/functional/FR-145..FR-148, spec/non-functional/NFR-031, rust/crates/quoin-risk-advice/**, rust/crates/quoin-cli/src/risk.rs, rust/crates/quoin-cli/tests/tc_145_risk_advise_command.rs"
relationships:
  - {target: "ix://agent-ix/quoin/FR-145", type: references}
  - {target: "ix://agent-ix/quoin/FR-146", type: references}
  - {target: "ix://agent-ix/quoin/FR-147", type: references}
  - {target: "ix://agent-ix/quoin/FR-148", type: references}
  - {target: "ix://agent-ix/quoin/NFR-031", type: references}
---

# Gap analysis of quoin risk advise (PR 852)

## Summary

Ticket: PLAT-1652 (with PLAT-1653 and PLAT-1654). This is a planless gap analysis scoped to the
PR diff. Plan completion was not assessed. Coverage was read from `quire matrix --format tsv`
(quire 0.44.0, engine 0.59.1) at 03fe9207. All 43 acceptance criteria of FR-145..148 and
NFR-031 are `tagged`, and each binds a test in `quoin-risk-advice/tests/tc_145_risk_advice.rs` or
`quoin-cli/tests/tc_145_risk_advise_command.rs`. NFR-031-M-1..4 are metric rows with no tag of
their own; the ACs measure them.

Each test was read against the AC it claims. The tests are behaviour tests over authored
tempdir fixtures. None asserts a hash, pin, file count or copied file. Every production module
traces to an FR: `policy.rs` to FR-146, `evidence.rs` to FR-147, `queue.rs` to FR-148,
`render.rs` to FR-146..148 and NFR-031, and `corpus`, `records`, `links` and `lib` to FR-145.
No stub or dead branch was found.

Examined and backed with no finding: FR-145-AC-1..6, FR-146-AC-2..10, FR-147-AC-1..3,
FR-147-AC-5..9, FR-147-AC-11, FR-148-AC-1, -2, -4, -6..10, and NFR-031-AC-1..3 and -6.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-148-AC-3 (reordering the policy's region list leaves the queue order unchanged) is tested on a queue of one entry, so the order cannot change and the test cannot fail for a region-ordered queue. Use two or more undecided residual risks whose regions sort opposite to their ids. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:597-639 |
| FND-002 | medium | The second clause of FR-148-AC-5 (two residual risks on different policies each show their own region) is untested. The tagged test has one residual risk on one policy. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:597-639 |
| FND-003 | low | FR-146-AC-1 asks for two pre-control and two post-control estimates. The fixture has one of each, so ordering inside a phase is not exercised. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:248-282 |
| FND-004 | low | FR-147-AC-10's clause "two entries with different gap numbers are ordered by estimate id" is untested. The tagged test has one evidence entry, and the two entries in the estimate-level test carry two tokens each. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:393-448, rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:488-515 |
| FND-005 | low | NFR-031-AC-5 needs an all-acceptable policy. The fixture policy flags `High` as outside the acceptable region, so the AC's condition is not built. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:671-691 |
| FND-006 | low | NFR-031-AC-4 needs a fixture with every optional input absent. The tagged test is the level-finding test over the full chain, which leaves many optional inputs present, such as anomalies and requirements. | rust/crates/quoin-risk-advice/tests/tc_145_risk_advice.rs:301-335 |
| FND-007 | low | Behaviour that this PR adds to the spec has no test: FR-145's skipped estimate (bad `phase` or no Judgement table), and FR-147's several-situation rule (one `situation-without-hazard` per situation, `hazards` over all situations, `situation` the first by id). | spec/functional/FR-145-risk-advise-command.md:44, spec/functional/FR-147-risk-advise-evidence-gaps.md:63, rust/crates/quoin-risk-advice/src/evidence.rs:64-101 |

## Verdict

PARTIAL. Every criterion is tagged and the implementation meets each one I traced. Two medium
test gaps (FR-148-AC-3 cannot fail, and FR-148-AC-5 is half covered) and five low ones need tests
inside this PR. No production gap was found.

## New findings (disposition pass 1)

Against head e7389f42.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | FR-145's new behaviour, that a type to check which no loaded module declares makes the command exit 2 naming the document and the type, has no test. Every test passes `--module` or a closed module selection. | spec/functional/FR-145-risk-advise-command.md:45, rust/crates/quoin-cli/tests/tc_145_risk_advise_command.rs:38-47 |

## Dispositions

Round 1, against commit e7389f42.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
