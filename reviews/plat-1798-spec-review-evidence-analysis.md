---
id: SR-20261122002
title: "Evidence review disposition for PLAT-1798 FR-169 amendment"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin PR #1002 at 466596fa; FR-169-AC-2 and FR-169-AC-28"
review_set: all
---

# Evidence review disposition for PLAT-1798 FR-169 amendment

## Summary

This is a focused disposition of the evidence analysis finding from the selected all-analysis review of PR #1002. The catalog has no method recommendation for FR-169-AC-2; `Test` is retained as an explicit human method choice because the criterion checks the report's parsed JSON key set against the exact output contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `quoin advise` has no catalog recommendation for FR-169-AC-2; its authored `Test` method is a human choice. | spec/functional/FR-169-risk-summary-command.md:145 |

## Verdict

**Disposition accepted; no requirement amendment needed.** The chosen method is recorded as human judgment rather than catalog advice. FR-169-AC-2 remains `Test` because an executable fixture can compare the exact keys emitted by JSON output; the existing matrix binds it to `rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs::a_complete_chain_is_one_row_with_exact_keys_and_no_findings`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | human-method-choice-recorded | `quoin advise --repo . --json` reports `inconclusive: true`, `recommended: []`, and authored method `Test` for FR-169-AC-2. The method is retained by human judgment because exact output-key equality is directly observable in a test. No catalog recommendation is claimed. FR-169-AC-28, separately, has catalog recommendations in the Test class (`bdd-spec-by-example` and `unit-testing`); its present untagged matrix state is recorded in the base disposition and remains an implementation-stage obligation. |

## Verification record

At PR head `466596fa8e90ce800bcc1483ab927dcc26cfee54`, `quoin advise --repo . --json` reports AC-2 as inconclusive with no recommendation and AC-28 with Test-class recommendations. `quire matrix --scope . --format json` reports AC-2 as `tagged` and AC-28 as `untagged`. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
