---
id: SR-2026112016223475846355016
title: "PLAT-1855 quoin PR 1046 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1046 (FR-161-AC-12, FR-161-AC-15..17, FR-165-AC-20, FR-193-AC-9, FR-194-AC-9) against rust/crates/quoin-cli/src/class_obligations.rs, rust/crates/quoin-dossier/src/{architecture,trace_matrix}.rs and rust/crates/quoin-assurance/tests/tc_1080_matrix.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
  - target: "ix://agent-ix/quoin/FR-165"
    type: references
  - target: "ix://agent-ix/quoin/FR-193"
    type: references
  - target: "ix://agent-ix/quoin/FR-194"
    type: references
---

# PLAT-1855 quoin PR 1046 gap-analysis

## Summary

Ticket: PLAT-1855. Planless gap analysis of the diff `origin/main...HEAD` (head f9a0d3ca). I ran `quoin matrix --repo . --json` (quoin 0.36.8) on the head. All seven touched criteria are `tagged`:

- FR-161-AC-12: `relation_backed_entries_are_refused_when_malformed`
- FR-161-AC-15: `a_pinned_design_trace_reads_only_its_relation_and_lists_it_missing`
- FR-161-AC-16: `a_relation_a_module_adds_turns_an_unpinned_design_trace_unmet_naming_it`
- FR-161-AC-17: `design_trace_soup_and_segregation_entries_report_one_row_each`
- FR-165-AC-20: `tc_1080_012b_each_uncovered_block_is_its_own_design_gap_line`
- FR-193-AC-9: `one_no_design_element_gap_per_uncovered_relation_naming_its_check`
- FR-194-AC-9: `one_no_design_element_gap_per_uncovered_block_naming_its_check`

I checked each test by reading it against the mutations the brief lists:

- Dropping the pinned-relation filter fails AC-15, because the covered `r` plus uncovered `s` case would read `unmet`.
- Listing every uncovered relation for a pinned entry fails AC-15, which expects `missing == ["r"]`.
- Collapsing the per-block loop in either dossier generator fails AC-9 in both FR-193 and FR-194, because FR-2 expects two gaps.
- Changing the check token or the detail text fails both AC-9 tests.
- Adding `missing` on `met` rows is an equivalent mutant for design-trace: with no archetypes, an `uncovered` row always folds to `unmet`.

The spec-PR gap analysis (SR-2026112016223475846355014) carried a deferred FND-001: the AC-12 test asserted that a pinned `design-trace` is refused. That is closed on this branch: the assertion is inverted and the accepted case is in the `ok` list. No stub or unowned production code is in the diff, and the semantic review was not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-161-AC-12 has an untested refusal case: a `design-trace` entry naming a `method`. Nothing asserts that `{"kind":"design-trace","method":...}` is refused. The refusal message "must not name a method or element archetypes" names both possible causes, not the one present, while AC-12 says the entry is "refused naming the cause". Add the method case and split the message so it names which field was given. | rust/crates/quoin-cli/src/class_obligations.rs:251-256; rust/crates/quoin-cli/src/class_obligations.rs:937-975 |
| FND-002 | low | The FR-165-AC-20 test checks only that A has two `Design gap:` lines, that `(p)` appears somewhere, and that B and C have one line and none. It never asserts that A's second line is `(q)` or that B's line is `(q)`, so a renderer that repeats the first check token passes. Both fixture blocks also use relation `realized-by` and differ only by `check`, while the AC describes relations P and Q. Assert the exact lines under each section and give the blocks distinct `relation` names. | rust/crates/quoin-assurance/tests/tc_1080_matrix.rs:747-786 |
| FND-003 | low | The FR-161-AC-17 test reaches only one combination for the three-entry policy: design-trace `unmet`, soup `unmet` and segregation `not-available`. The AC's `met` arm ("each row is `met` when the rows it reads are `covered`") is never reached under that policy, so a regression in routing a covered row through the combined policy would not show here. Other tests cover each kind alone. | rust/crates/quoin-cli/src/class_obligations.rs:1040-1075 |

## Coverage

Plan completion: not assessed

| Criterion | Tests (binding) |
| --- | --- |
| FR-161-AC-12 | relation_backed_entries_are_refused_when_malformed (correct; method case missing, FND-001) |
| FR-161-AC-15 | a_pinned_design_trace_reads_only_its_relation_and_lists_it_missing (correct) |
| FR-161-AC-16 | a_relation_a_module_adds_turns_an_unpinned_design_trace_unmet_naming_it (correct) |
| FR-161-AC-17 | design_trace_soup_and_segregation_entries_report_one_row_each (correct; met arm unexercised, FND-003) |
| FR-165-AC-20 | tc_1080_012b_each_uncovered_block_is_its_own_design_gap_line (correct; weak oracle, FND-002) |
| FR-193-AC-9 | one_no_design_element_gap_per_uncovered_relation_naming_its_check (correct) |
| FR-194-AC-9 | one_no_design_element_gap_per_uncovered_block_naming_its_check (correct) |

Semantic review: not run (reviewer subagent; mutation reasoning above stands in for the AC-level checks the brief asked for).

## Verdict

CONDITIONAL. Every touched criterion is tagged by a test that fails under the main mutations. The three findings are weak or missing assertions, with no untested behaviour on the main path.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a4b8680f: a design-trace entry naming a method is refused with "must not name a method", and element archetypes get their own message. The test asserts that the method message does not mention archetypes. Mutation run: restoring the combined message "must not name a method or element archetypes" makes `relation_backed_entries_are_refused_when_malformed` fail (class_obligations.rs:965). |
| FND-002 | fixed | a4b8680f: the AC-20 test asserts the exact `Design gap:` lines per section: FR-101 `(p)` then `(q)`, FR-102 `(q)` only, FR-103 none. Mutation run: hard-coding the rendered check token to `p` in `render_markdown` makes tc_1080_012b fail (tc_1080_matrix.rs:783). The fixture still gives both blocks relation `realized-by`. The renderer keys on `check`, so the oracle no longer depends on it. |
| FND-003 | still-open | At a4b8680f `design_trace_soup_and_segregation_entries_report_one_row_each` is unchanged. It still asserts only `["unmet", "unmet", "not-available"]`, and no test runs the three-entry policy over an all-covered join to get `met`. The fix commit's message claims "AC-17 tests", but no AC-17 change is in the diff. |
| FND-003 | fixed | FR-2 has a covered row in each of real, S and G. One call to build_rows applies the three-entry policy and the assertion checks the identity, status and count of its three activities. |

### Disposition pass 2 evidence

Reviewed commit `de205cd1b04584e11bc8e0518da8f46b1b419d5a`.

FND-003 after_excerpt (rust/crates/quoin-cli/src/class_obligations.rs, lines 1093–1096)

```rust
        assert_eq!(
            statuses(1),
            [("trace", "met"), ("soup", "met"), ("seg", "met")]
        );
```
