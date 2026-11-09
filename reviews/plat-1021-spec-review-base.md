---
id: SR-16393
title: "base review of PLAT-1021 graded-corpus collection producer (FR-108-AC-15..21)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-108-measurement-verdict-checker.md (FR-108-AC-15..AC-21, Graded-corpus collection producer section, Dependencies bullet), spec/matrix.md (FR-108 row) -- PR #779 diff only"
review_set: subset
---

## Summary

Ticket: PLAT-1021. Base checklist over the PR #779 diff, checked against the
code on quoin `main` (quoin-jev `constant_predictor.rs`, quoin-measurement
intake `validate/` and `store/publish.rs`, `verify/`, `tests/support/grading.rs`,
`tests/live_criterion_strength.rs`, `spec/assurance/MP-228`). The new ids are
unique (FR-108-AC-15..AC-21 and TC-2013..TC-2020 are unused on `main`), the
refusal code `QM-COLLECTION-INVALID` is the existing
`MeasurementErrorCode::CollectionInvalid`, and the aggregate shape
(`ratio`/`fraction`/`examined`/`matched`/`complete`/`repetitions`) matches what
the checker reads. Two high findings: the cited test cases have no Test Case
Summary rows, and the plan the spec names as governing (MP-228) carries no
decision rule, so AC-21's verify claim cannot hold.

Assurance profiles: AP-201 (measurement persistence) and AP-202 (Jev lens
evaluation) apply and set `review_policy.mode: require` for spec-review; this
direct review does not by itself constitute the enforced ix-flow run, whose
final transition needs human acknowledgement.

Analyses run: base, integrity, ears-conformance, failure-domain.
criterion-strength skipped: the skill states "Do not run this skill yet", and
MP-228 records the lens NO-GO; criterion strength was judged by hand in this
base review instead (each new AC names a concrete, failing observable).

## Scope examined

- FR-108-AC-15 .. FR-108-AC-21 (examined)
- "Graded-corpus collection producer (PLAT-1021)" section incl. the PLAT-1020 forward note (examined; data-shape only, does not specify PLAT-1020)
- FR-108 Dependencies bullet for `graded_corpus` (examined)
- spec/matrix.md FR-108 row (examined)
- FR-108-AC-10, MP-228 (context_only)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | TC-2013..TC-2020 are cited by FR-108-AC-15..AC-21 and the FR-108 matrix row but have no rows in spec/matrix.md's Test Case Summary; spec-ahead criteria elsewhere (TC-1656) carry a planned row, so these eight ids are undefined and TC-2020's offline method (cassette or factored mapping) is unstated | spec/matrix.md Test Case Summary; FR-108-AC-15..AC-21 |
| FND-002 | high | FR-108-AC-21 says the written collection "is decided by quoin measurement verify on the governing plan with a recomputed constant-predictor baseline", and the section names MP-228 as that plan; MP-228 on main has no statistical_design (no estimator, no decision_rule, no constant-predictor baseline) and its metric jev.criterion_strength.gate is a three-bar gate, so verify returns inconclusive no_decision_rule; the PR neither amends MP-228 nor names a plan that carries such a rule | FR-108-AC-21; spec/assurance/MP-228-jev-criterion-strength-gate.md; quoin-measurement src/verify/mod.rs:139 |
