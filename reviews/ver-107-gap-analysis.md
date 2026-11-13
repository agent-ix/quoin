---
id: SR-20261113155335
title: "Gap analysis — VER-107 FR-116 qualifier and quoin-quire bridge (quoin PR 877)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-153-analysis-result-counting-by-disposition.md, spec/functional/FR-184-analysis-act-producer-definition.md, spec/functional/FR-116-owner-qualified-quire-evidence.md (context), spec/test/TC-002-analysis-result-counting.md, rust/crates/quoin-quire/tests/analysis_counting.rs, rust/crates/quoin-evidence/src/qualify/**, rust/crates/quoin-quire/src/evidence.rs"
review_set: subset
---

## Summary

Ticket: VER-107. PR head `db0ed2ae`. Plan completion: not assessed. This is a planless audit of
FR-153-AC-1..11 and FR-184-AC-1..16 against the tagged tests in
`rust/crates/quoin-quire/tests/analysis_counting.rs` (13 tests, all passing on this head) and the
code they exercise.

`quoin matrix --json` on this head reports only 11 `tagged` rows out of 1831 repo-wide, and none
of the FR-153 or FR-184 rows, although the tests carry `#[trace(...)]`. Earlier merged tests in
`quoin-evidence/tests` read as untagged too. That is a matrix or traceability-model condition on
main, not something this PR introduced. The AC-to-test mapping below was therefore done by reading
the trace attributes and the assertions.

Every FR-153 and FR-184 criterion has a tagged test. Most assert the stated behaviour: status,
`unbacked_rows`, `record_binders`, cause codes with premise-named details, locators and the strict
decision. The gaps are the four criteria whose tests assert a weaker condition than the criterion
states, plus missing implementation trace tags.

## Verdict

The coverage gaps are real but medium or low. None blocks on its own; the merge blocker is
code-review FND-001 (SR-20261113155334). FND-001 and FND-002 here should be closed in the same fix
round, because each is a criterion its tagged test does not actually exercise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-153-AC-8 ("a bound pass entry with a healthy `quoin evidence audit` and no FR-116 decision is not counted") is tagged on a test that never runs the audit. It only withholds the assessment or the SHA, so the healthy-audit condition (TC-002 step 3, "then run `quoin evidence audit`") is untested. | rust/crates/quoin-quire/tests/analysis_counting.rs:341-372 |
| FND-002 | medium | FR-153-AC-9 requires that "a source-symbol query for `<analysisKind>::<requestDigest>` returns no claim, and the id appearing in prose stays a citation". The test checks only that `criterion.binders` is empty. It issues no source-symbol (inverse by symbol or file) query and has no prose-citation case. | rust/crates/quoin-quire/tests/analysis_counting.rs:631-655 |
| FND-003 | low | FR-184-AC-12 names three missing-field cases. The test covers a missing `identity.version`. Its relative `configuredExecutable` case asserts only `counted == 0`, not the `unevaluated` tool and configuration cause. The missing `status`/`outcome` branch is untested. | rust/crates/quoin-quire/tests/analysis_counting.rs:374-427 |
| FND-004 | low | FR-153-AC-10 says "even where its symbol or digest text equals an obligation id". The test records without `--discharges` against ordinary ids only, so the colliding-text case is not exercised by the counting path. | rust/crates/quoin-quire/tests/analysis_counting.rs:657-664 |
| FND-005 | low | The new public entry points `quoin_evidence::qualify::qualify` and `quoin_quire::evidence::compute` carry no `#[implements("FR-116")]` / `#[implements("FR-153")]`, unlike the crate's other requirement-owning functions (trust.rs FR-093, independence.rs FR-094, differential_report.rs FR-151). They read as code with no owning requirement. | rust/crates/quoin-evidence/src/qualify/mod.rs:257; rust/crates/quoin-quire/src/evidence.rs:127 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-153-AC-1 / FR-184-AC-2, AC-13, AC-15 | examined | Covered: qualified-record, locators, consistency-only provenance, sibling not credited |
| FR-153-AC-2 / FR-184-AC-1, AC-14 | examined | Covered, premise named |
| FR-153-AC-3 / FR-184-AC-3..5 | examined | Covered with locators and strict rejection |
| FR-153-AC-4 / FR-184-AC-6 | examined | Covered; equal to never-recorded |
| FR-153-AC-5 / FR-184-AC-7, AC-8 | examined | Covered, both supply orders and equal timestamps |
| FR-153-AC-6 | examined | Covered |
| FR-153-AC-7 | examined | Covered, same and different suite |
| FR-153-AC-8 | examined | FND-001 |
| FR-153-AC-9 | examined | FND-002 |
| FR-153-AC-10 | examined | FND-004 |
| FR-153-AC-11 / FR-184-AC-9 | examined | Covered |
| FR-184-AC-10, AC-11 | examined | Covered |
| FR-184-AC-12 | examined | FND-003 |
| FR-184-AC-16 | examined | Covered: two act-scope causes, none counted |
| TC-002 steps 3-6 | context_only | Steps 3 to 6 map to the tests above; step 3's audit is FND-001 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `a_passing_entry_with_a_healthy_audit_and_no_decision_is_not_counted` runs `quoin_auditor::audit`, asserts no findings and `healthy` contains AC1, then shows the entry is not counted without a decision. |
| FND-002 | fixed | The trace test now runs `trace_search` with `Query::SymbolName(<entry identity>)` (no claim, unresolved) and with `Query::Id(FR-001-AC-1)`, which shows the id in fixture `lib/prose.rs` as a citation and not a claim. |
| FND-003 | fixed | Missing `identity.version`, `identity`, `status`, `outcome` and a relative `configuredExecutable` are each asserted `unevaluated`, naming the tool and configuration premise. |
| FND-004 | fixed | Added a report whose `analysisKind` and `requestDigest` both equal `FR-001-AC-1`, recorded with no discharges: no binding and no claim. |
| FND-005 | fixed | `#[implements("FR-116")]` is on `qualify` and `#[implements("FR-153")]` is on `compute`. |
