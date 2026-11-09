---
id: SR-17046
title: "Gap analysis of quoin PR 781 (FR-071 criterion backing)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@f17b3c2d; spec/functional/FR-071-typed-properties-mapping.md, rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, rust/crates/quoin-semantic/tests/tc_452_sweep_criteria.rs"
review_set: subset
---

# Gap analysis of quoin PR 781

## Summary

Ticket: PLAT-278. Plan completion: not assessed. Each FR-071-AC-3..AC-8 test was compared with
its criterion text and with the `quire-fixtures` cases at the pinned rev ce172a7d
(`fixtures/mapping/cell-cases.json`, `both-forms.md`, `both-forms.expected.json`). Tools:
quoin 0.29.1.

Bindings checked:

- AC-3 checks that the table comes before the fence. It requires `firstForm.line` to equal the
  table header line (12) and the diagnostic line to equal the fence line (16), and it checks the
  code and severity.
- AC-4 covers all six listed tokens, plus the advisory finding and the placeholder target for
  `Mystery`.
- AC-5 validates every mapped multiplicity against semantic-core `Multiplicity.json`, and refuses
  `5..2` and `1 ordered` with a row locus.
- AC-6 covers all five listed cells, plus the `identity` flag and the `mnimum` refusal.
- AC-7 covers the three fence lines.
- AC-8 covers the bare `Decimal` reader-rule diagnostic at the row.

## Verdict

**PASS WITH FINDINGS (one medium, one low).** AC-3..AC-8 are now backed by tests that can fail.
FR-071-AC-1 still reads as covered in the computed matrix through a test that does not assert it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-071-AC-1 is tagged by `tc_452_650` (`/// Trace: FR-071-AC-1`). That test asserts only that the table classifies as `TypedTable` and that `config-version.expected.json` has 7 fields. Nothing in it validates against `FieldDecl.json`. The matrix therefore reports AC-1 as backed, while the PR's own note says AC-1 cannot be asserted yet | rust/crates/quoin-semantic/tests/tc_452_sweep_criteria.rs:88-107 |
| FND-002 | low | FR-071 says `Type` resolves "in this order" (KernelScalar, then id, then title, then import). The test name says "resolve in the specified order", but every case has exactly one candidate, so precedence is never exercised. The fixture set has no token that two rules could both resolve | rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs:128-182 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-071-AC-1, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Quoin's copy of the config-service FR-006 rows, rewritten as a typed table, has an expected `FieldDecl[]` fixture whose every element validates against the semantic-core `FieldDecl.json`."}
- {id: FR-071-AC-3, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "The both-forms fixture's expected diagnostic is a failure at the second form's locus."}
- {id: FR-071-AC-4, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "`Type` tokens `UUID`, `Decimal(10,2)`, `Duration [ms]`, `ConfigOverlay` (object by title), `Status` (enumeration by id), and an imported export resolve as specified; `Mystery` yields the advisory finding and the placeholder target."}
- {id: FR-071-AC-5, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Multiplicity cells `1`, `0..1`, `1..* ordered unique`, `2..5`, and empty map to the expected `Multiplicity` objects; `5..2` and `1 ordered` are errors."}
- {id: FR-071-AC-6, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Constraint cells `min: 1, maxLength: 64, identity`, `pattern: /^[a-z]+$/`, `enumValues: draft|final`, `nonEmpty`, `format: agent-ix:email` map as specified; `mnimum: 1` is an error with locus."}
- {id: FR-071-AC-7, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Fence lines `item x : Y;`, `part def X;`, and `:> Y` are errors with locus."}
- {id: FR-071-AC-8, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "A fixture row violating a semantic-core reader rule (`Decimal` without `(p,s)`) yields the rule's diagnostic at the row locus."}
- {id: FR-071-CON-2, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Quoin SHALL read the golden fixtures ... from the `quire-fixtures` crate in `agent-ix/quire-rs`, holding no copy of them."}
bindings:
- {test_id: tc_278_003_both_forms_fails_at_the_second_forms_locus, ac_id: FR-071-AC-3, trace: correct}
- {test_id: tc_278_004_type_cells_resolve_in_the_specified_order, ac_id: FR-071-AC-4, trace: correct}
- {test_id: tc_278_005_multiplicity_cells_map_and_invalid_cells_are_errors, ac_id: FR-071-AC-5, trace: correct}
- {test_id: tc_278_006_constraint_cells_map_to_the_closed_vocabulary, ac_id: FR-071-AC-6, trace: correct}
- {test_id: tc_278_007_fence_lines_outside_the_subset_are_errors_with_locus, ac_id: FR-071-AC-7, trace: correct}
- {test_id: tc_278_008_a_reader_rule_violation_is_reported_at_the_row, ac_id: FR-071-AC-8, trace: correct}
- {test_id: tc_452_650_the_typed_table_fixture_classifies_as_the_authored_form, ac_id: FR-071-AC-1, trace: wrong}
```
