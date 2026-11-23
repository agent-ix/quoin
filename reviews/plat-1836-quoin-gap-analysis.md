---
id: SR-2026112016223475846355009
title: "PLAT-1836 quoin PR 1032 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1032 (FR-160-AC-14..16, FR-161-AC-8, FR-161-AC-10..13, quoin-auditor audit/class.rs, quoin-cli class_obligations.rs and evidence/audit.rs)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-160"
    type: references
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
---

# PLAT-1836 quoin PR 1032 gap-analysis

## Summary

Ticket: PLAT-1836. `quoin matrix --repo . --json` (quoin 0.36.8) on the PR head dcc95f29: FR-160-AC-14 (3 binders), AC-15 (1), AC-16 (1) and FR-161-AC-10 (2), AC-11 (2), AC-12 (1), AC-13 (2) are all `tagged`; FR-160-AC-13 stays `method-without-symbol` (Inspection, pre-existing). A grep of the diff's production code finds no class value, default policy, ratio or carried field name, consistent with FR-160-AC-13. No stub, `todo!` or `unimplemented!`; no new production code without an owning requirement. No pin, SHA or count assertion in the new tests. Admitted gaps confirmed and in spec: no module yet declares SOUP/segregation relations (rows read `not-available`), and `quoin matrix` reports but does not gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-160-AC-15 says every class-caused finding carries `class`; the test asserts it for `class-method-missing`, `insufficient-mutation-score` and `class-unconfigured` only. `unmeasured-mutation-score`, `surviving-mutant` (the new `ladder.rs` mapping) and class-sourced `insufficient-independence` are unasserted, so dropping `with_class` from the surviving-mutant map or the unmeasured branch passes. | rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:868-902; rust/crates/quoin-auditor/src/audit/ladder.rs:143-149 |
| FND-002 | low | FR-160-AC-16's middle clause, "over obligations with only non-class findings ... exits as without the policy", has no case: tc_plat1836_002 checks gated-with-class-finding and gated-with-everything-met, never gated with a non-class finding present and no class finding. Counting all findings instead of class findings would pass. | rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:582-611 |
| FND-003 | low | tc_plat1836_001 is tagged `FR-161-AC-2` but drives `quoin evidence audit` and asserts audit findings; it never runs `quoin class obligations`, so it does not verify FR-161-AC-2 (design-trace `unmet` on an uncovered row; `evidence` `unmet` listing the obligation). Its correct criterion is FR-160-AC-14 alone. | rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:552-580 |

## Coverage

Plan completion: not assessed

| Criterion | Tests (binding) |
| --- | --- |
| FR-160-AC-14 | a_run_that_did_not_pass_does_not_satisfy_a_required_method; a_missing_run_is_reported_as_unbound_not_as_failed; tc_plat1836_001 (correct) |
| FR-160-AC-15 | every_class_caused_finding_names_the_class_and_others_do_not (correct, partial; FND-001) |
| FR-160-AC-16 | tc_plat1836_002 (correct, partial; FND-002 and code-review FND-002) |
| FR-161-AC-2 | tc_plat1836_001 (wrong; FND-003) |
| FR-161-AC-10 | detailed_design_needs_a_covered_row_linking_a_declared_archetype; tc_plat1836_003 (correct) |
| FR-161-AC-11 | a_named_relation_reads_only_its_own_rows; tc_plat1836_003 (correct) |
| FR-161-AC-12 | relation_backed_entries_are_refused_when_malformed (correct) |
| FR-161-AC-13 | only_an_unmet_row_counts_toward_the_gate; tc_plat1836_004 (correct) |

## Verdict

APPROVE WITH LOW FINDINGS for coverage; every new criterion is tagged and each has at least one assertion that can fail. The merge-blocking issue is the passing-run semantics in code-review FND-001, not coverage.

## Examined

- FR-160-AC-2, FR-160-AC-13, FR-160-AC-14, FR-160-AC-15, FR-160-AC-16 (examined)
- FR-161-AC-2, FR-161-AC-8, FR-161-AC-10, FR-161-AC-11, FR-161-AC-12, FR-161-AC-13 (examined)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a102c716: `every_class_caused_finding_kind_names_the_class` asserts the `class` member on all six kinds (class-method-missing, insufficient-mutation-score, surviving-mutant, unmeasured-mutation-score, class-unconfigured, insufficient-independence with source class); `findings_the_class_did_not_cause_carry_no_class_member` covers the negative side. Passed. |
| FND-002 | fixed | a102c716: `tc_plat1836_006_non_class_findings_do_not_gate` runs `gate: true` with an `undischarged` finding present and no class finding and asserts exit 0. Passed. |
| FND-003 | fixed | a102c716: tc_plat1836_001 now tagged `FR-160-AC-14` only; new `tc_plat1836_005_class_obligations_reports_unmet_design_and_failing_evidence` runs `quoin class obligations` and asserts design-trace `unmet` on an uncovered row and `evidence` `unmet` listing FR-001-AC-1, FR-001-AC-2, then both `met`. `quoin matrix` shows FR-161-AC-2 tagged by it. Passed. |
