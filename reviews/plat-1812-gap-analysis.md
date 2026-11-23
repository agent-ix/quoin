---
id: SR-20261123200830
title: "Gap analysis of quoin PR 1031 against FR-141, FR-041 and FR-150"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@9f5cf57a; spec/functional/FR-141-promote-sbom-components-to-soup-records.md (AC-18..25, CON-1, CON-3, CON-4), spec/functional/FR-041-sbom-inventory-evidence.md (AC-11, AC-12), spec/functional/FR-150-licence-and-ban-policy-findings.md (AC-17); rust/crates/quoin-evidence/**, rust/crates/quoin-soup/src/soup{,/plan}.rs, rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs"
review_set: subset
---

# Gap analysis of quoin PR 1031

## Summary

Ticket: PLAT-1812. This is a manual acceptance-criteria-to-test check scoped to the PR's criteria. The `quoin:gap-analysis` skill's matrix, reverse-gap and stub procedures were not run, so this is not a full repository gap audit. For each criterion, the check names the tagged test and asks whether that test would fail on the pre-PR code. All tagged tests pass: `tc_1515_soup_promote` 23/23, `tc_licence_policy` 16/16, quoin-evidence lib 75/75, quoin-soup lib 57 passed and 1 ignored.

## Verdict

**PASS WITH FINDINGS: four low coverage gaps, and no criterion without a test.** These tests have a real oracle that the pre-PR code fails:

- FR-141-AC-18 to AC-25 each have a CLI test, and AC-20, AC-21, AC-24 and AC-25 also have plan unit tests. On the pre-PR code no supplier or licence was written and the report had no difference, conflict or omitted section, so every one of these fails.
- FR-141-CON-4 is asserted byte-identical through `snapshot`, together with AC-22 and AC-23.
- FR-041-AC-11 and AC-12 have reader unit tests.
- FR-150-AC-17 has a test of all three message shapes. The updated `tc_1671_014` now asserts that the entry JSON is not in the message.

CON-1 and CON-3 were checked by inspection. `read_components` is the only reader, and licence text is written, not joined; only the report joins texts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The SPDX half of the FR-141-AC-18 test promotes `pkg:npm/a@1` (an id) and `pkg:npm/c@1` (an expression), but asserts only `a`. A wrong `supplier` or `license` on the SPDX expression record would pass. | rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs:620-677 |
| FND-002 | low | The FR-041-AC-11 clause "the run entries emitted for the same documents are byte-identical to those emitted before the supplier was exposed" has no test. `tc_1812_001` checks only the exposed suppliers. `tc_1671_003` makes the matching comparison for licences, by stripping the field and comparing `parse_sbom` entries; supplier has no such comparison. | rust/crates/quoin-evidence/src/adapters/sbom.rs:317-340; rust/crates/quoin-evidence/tests/tc_licence_policy.rs:152 |
| FND-003 | low | No test exercises the field-difference branch for a record already at `--out` when `--records` names another directory. That branch is the `adopted` loop that calls `plan::differences`. Removing the loop leaves every test green, so a regression there would report nothing for a person-edited record at the destination. | rust/crates/quoin-soup/src/soup.rs:280-286 |
| FND-004 | low | The FR-150 precedence for an unreadable entry has no test: "an `unreadable` licence entry, and no denied or not-allowed licence, as `licence-unknown`". No test pairs an unreadable entry with a denied text, or with an id that is not allowed, to show that `licence-denied` or `licence-not-allowed` wins. The code is correct by inspection. | rust/crates/quoin-evidence/src/adapters/licence_policy.rs:177-215 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-141-AC-18, role: examined, test: a_stated_supplier_and_one_licence_are_written_as_stated_for_each_licence_kind}
- {id: FR-141-AC-19, role: examined, test: an_unstated_supplier_or_licence_is_absent_and_never_the_word_unknown}
- {id: FR-141-AC-20, role: examined, test: several_licences_leave_the_licence_absent_and_are_listed_with_their_texts; tc_1812_002 (plan)}
- {id: FR-141-AC-21, role: examined, test: control_characters_and_unreadable_licences_omit_the_field_and_are_listed}
- {id: FR-141-AC-22, role: examined, test: an_existing_record_is_never_rewritten_and_its_differing_fields_are_reported}
- {id: FR-141-AC-23, role: examined, test: a_field_set_where_the_sbom_states_none_and_the_reverse_are_differences}
- {id: FR-141-AC-24, role: examined, test: control_characters_and_unreadable_licences_omit_the_field_and_are_listed}
- {id: FR-141-AC-25, role: examined, test: a_repeated_purl_carries_the_first_occurrences_values_and_lists_the_conflict; tc_1812_001 (plan)}
- {id: FR-141-CON-4, role: examined, test: an_existing_record_is_never_rewritten_and_its_differing_fields_are_reported}
- {id: FR-041-AC-11, role: examined, test: tc_1812_001_the_declared_supplier_is_exposed_as_written}
- {id: FR-041-AC-12, role: examined, test: tc_1812_002_an_unreadable_licence_entry_carries_no_text}
- {id: FR-150-AC-17, role: examined, test: tc_1812_003_an_unreadable_entry_message_is_the_fixed_text_after_unlisted_texts}
```
