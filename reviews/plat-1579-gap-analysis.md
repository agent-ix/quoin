---
id: SR-20261113203415
title: "Gap analysis — PLAT-1579 per-class evidence floors, FR-160 (quoin PR 880)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-160-per-class-evidence-floors.md, rust/crates/quoin-auditor/src/audit/{class.rs,ladder.rs,scores.rs,mod.rs}, rust/crates/quoin-auditor/tests/tc_160_class_floors.rs, rust/crates/quoin-assurance/tests/tc_1080_matrix.rs, rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs, rust/crates/quoin-core/src/ops/quire/tests.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1579. PR head `46d853e4`. Plan completion: not assessed. The ticket's four ACs (policy
as data with no built-in floor; a finding naming class, floor and shortfall; a class column in
audit and matrix output; tests for each floor kind and for an unclassified obligation) are all met
by the code and the tests.

`quoin matrix --json` (quoin 0.30.1) reports every FR-160 criterion `untagged`, even though the tests
carry `#[trace("FR-160-AC-n")]`. The same happens across the repository: 1656 untagged and 11 tagged,
and FR-039 and FR-094 are also all untagged. So this is a matrix harvest limit with this repo's
attribute form, not something this PR caused. The AC-to-test map below was built by reading the
tags.

| AC | Tests | Result |
| --- | --- | --- |
| FR-160-AC-1 | without_a_policy_...; tc_160_300; tc_160_matrix_001 | covered |
| FR-160-AC-2 | a_required_method_needs_...; an_unbound_obligation_...; tc_160_301 | covered for runs; scan join untested (FND-001) |
| FR-160-AC-3 | a_class_mutation_floor_...; another_classs_mutation_floor_... | covered |
| FR-160-AC-4 | class_independence_needs_...; the_class_assessment_record_... | covered; shortfall text weak (FND-002, FND-003) |
| FR-160-AC-5 | criticality_and_class_floors_... | covered; tie untested (FND-004) |
| FR-160-AC-6 | an_unclassified_obligation_...; tc_160_301; tc_160_matrix_002 | covered (exit 0 via `ok()`) |
| FR-160-AC-7 | a_class_without_a_policy_entry_... | covered |
| FR-160-AC-8 | a_policy_that_cannot_be_applied_...; a_report_without_the_vocabulary_...; tc_160_303 | covered for the AC's causes; new spec causes untested (FND-005) |
| FR-160-AC-9 / CON-1 | the_class_is_read_only_from_the_declared_carried_name; criticality_is_never_read_as_a_class | covered |
| FR-160-AC-10 | profile_and_class_independence_are_assessed_separately_... | covered |
| FR-160-AC-11 | the_class_report_carries_...; tc_160_301; tc_160_302; tc_160_matrix_002 | covered |
| FR-160-AC-12 | two_audits_of_the_same_input_are_identical; tc_160_304 | covered |
| FR-160-AC-13 | inspection | holds: class values and the field name appear only in tests |

A targeted `cargo mutants --in-place -p quoin-auditor` run over `independence_failed`, `floor_for`,
`shortfall`, `evidence_findings` and `mutation_floor` tested 31 mutants: 25 caught, 3 unviable,
3 missed. The ladder early return after an independence failure (`independence_failed -> false`)
**is caught**. The three missed are FND-002, FND-003 and FND-004.

## Verdict

FR-160 is implemented and its ACs are backed by tests that can fail. Two medium test gaps should be
closed in this PR: the scan-record join that the spec amendment adds, and the shortfall text naming
a dimension that is not shared. The rest is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec amendment widens the required-method join to "run or finding-shaped scan record", but no test binds a scan-only suite. Every `scene()` supplies `runs` only, and the CLI test binds no evidence. If `scan` were dropped from `[run, scan]` in `evidence_findings`, every test would still pass, and a scanner-only method's floor could never be met again. Fix: add a case in tc_160_class_floors where the obligation's only bound suite has a `FindingRecord` (in `scans`) whose `evidence_kind` is the required method's kind, and assert no `class-method-missing`. | rust/crates/quoin-auditor/src/audit/class.rs:256-268; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:107-146 |
| FND-002 | medium | The class independence finding can name a dimension that is not shared, and no test fails. Mutant `class.rs:389 replace < with <=` survives. Under it, the AC-4 shared-actor case also reports `technique (shared)` although the two lines differ on technique. The test checks only that `actor (shared)` is present. FR-160 requires the finding to name "the dimensions still shared or missing", so a wrong dimension is a wrong result. Fix: in `class_independence_needs_...`, also assert the summary does not mention `technique`. | rust/crates/quoin-auditor/src/audit/class.rs:389; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:323-335 |
| FND-003 | low | This is the survived early-return mutant. `shortfall`'s `if lines < 2` early return is untested: `class.rs:380 replace < with >` survives. The one-suite case in AC-4 asserts only that one finding exists, so its shortfall text could become "No single pair of lines differs on every dimension at once" (a pair claim with one line) and nothing would notice. It does need a test, because the text is the shortfall FR-160 says the finding names. A one-line assertion on the existing `alone` report (summary contains `Fewer than two evidence lines`) is enough. | rust/crates/quoin-auditor/src/audit/class.rs:380-382; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:347-353 |
| FND-004 | low | The tie between a class floor and a criticality floor is untested: mutant `scores.rs:252 replace > with >=` survives, which flips the source a tie names from criticality to class. Fix: add a tie case (both 0.7) to `criticality_and_class_floors_...` asserting `criticality high`, alongside the FR-160 wording in SR-20261113203416 FND-002. | rust/crates/quoin-auditor/src/audit/scores.rs:252; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:400-438 |
| FND-005 | low | Two refusal causes the amendment adds are untested. One is a required method the catalog declares with no (or an empty) `evidence_kind`, handled by `declared_kind`'s `.filter`. The other is an unknown key in the policy file (`deny_unknown_fields`); its refusal path is serde's, not `QAU-CLASS-POLICY-REFUSED`, and no test shows it is refused with no output. Fix: add a `method("bare", "")` or a `None`-kind case to the refusal table, and one CLI or unit case with a misspelt key such as `mutation_flor`. | rust/crates/quoin-auditor/src/audit/class.rs:365-375; rust/crates/quoin-auditor/src/audit/class.rs:47,60 |
| FND-006 | low | FR-160 says quoin builds a class independence assessment record "for each obligation of the class". The code builds one only for obligations the ladder reaches (`Pass::class_independence` says "that the ladder reached"). A class-3 obligation that is undischarged, stale or rests on a vacuous scan has no record in `classIndependence`. Its other finding is still raised, so nothing passes wrongly, but the record set is narrower than stated. Fix: either state the narrower set in FR-160 or build the record before the ladder's early returns. | rust/crates/quoin-auditor/src/audit/ladder.rs:101-109; rust/crates/quoin-auditor/src/audit/mod.rs:131-133 |
| FND-007 | low | FR-160 says the profile assessments "carry `source: profile`". Under a class policy only the profile *finding* gains `source`. The profile `IndependenceAssessment` records in the report's `independence` member do not. Fix: say in FR-160 that `source` is on findings, or add it to the profile records under a policy. | rust/crates/quoin-auditor/src/audit/ladder.rs:191-205; spec/functional/FR-160-per-class-evidence-floors.md:85-86 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-160-AC-1 .. FR-160-AC-12, FR-160-CON-1 | examined | Mapped above |
| FR-160-AC-13, FR-160-CON-2 | examined | Inspection: hold |
| PLAT-1579 ticket ACs (4) | examined | Met |
| `ClassContext` (class.rs) | examined | FND-001, FND-002, FND-003, FND-005 |
| ladder `independence_failed` | examined | Early-return mutant caught; FND-006, FND-007 |
| `floor_for` | examined | FND-004 |
