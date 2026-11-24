---
id: SR-1854005
title: "PLAT-1854 quoin PR 1039 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1039 diff origin/main...HEAD, FR-029-AC-15..AC-23 and the Behavior bullets they verify"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: references
---

# PLAT-1854 quoin PR 1039 gap-analysis

## Summary

Ticket: PLAT-1854. This is a planless gap analysis of PR agent-ix/quoin#1039 at head 7f9ab78c, scoped to FR-029-AC-15 to AC-23 (spec merged in #1036) and the production code the diff adds. The matrix comes from `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2). Every criterion from AC-15 to AC-23 is `tagged`, with binder counts AC-15 3, AC-16 1, AC-17 1, AC-18 1, AC-19 2, AC-20 2, AC-21 15, AC-22 2 and AC-23 13. The tags were then checked against what each test asserts. Each existing require_complete_model call site, and each mapping, was mutated in turn (see SR-1854004).

Reverse gap: every production item the diff adds (`Error::TraceabilityDeclarationsDropped`, `DroppedDeclaration`, `require_complete_model`, `relation_error`, `map_error`'s `dropped_declarations` context, `declarations_dropped`, `Response::refusal`, `Failure`, `StatusStop`, the dossier up-front check, and the suite-trace `Error::Refused`) is owned by an FR-029 Behavior bullet. No code without an owner was found. Stubs: none. No `todo!`, `unimplemented!` or placeholder return appears in the diff, and no test asserts a tautology.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers FR-029-AC-20 for `quoin formal-rules status` with every declaration dropped. The CLI status test uses a one-drop scope. The all-dropped unit test reaches `assurance_declarations`, a different function. Moving `status`'s `require_complete_model` after the absent-model `ok_or_else` survives all 13 tests in `tc_1854_dropped_declarations_b` (mutation run). Add an all-dropped case, `required_relations: 5` under `traceability:`, to the status test. | rust/crates/quoin-cli/src/formal_rules.rs:309-318; rust/crates/quoin-cli/tests/tc_1854_dropped_declarations_b.rs:540-564 |
| FND-002 | medium | No test covers FR-029-AC-23 for `quoin formal-rules coverage`. AC-23 lists 15 commands and has 13 binders, and the dossier commands share one test. Coverage is the only command with no clean-scope test. A clean run exits 3 because of the pre-existing missing `document` projection (SR-1854004 FND-002), so the criterion cannot pass today, not merely lacks a test. | rust/crates/quoin-cli/tests/tc_1854_dropped_declarations_b.rs:556-564; spec/functional/FR-029-consume-the-quire-json-contract.md:132 |
| FND-003 | low | The `quoin anomaly check` clean-scope test, tagged FR-029-AC-23, accepts exit 0 or 1 and asserts nothing about stdout. AC-23 requires exit 0 and the command's normal output. Assert the exact status the fixture yields, and that the report lines are printed. | rust/crates/quoin-cli/tests/tc_1854_dropped_declarations_a.rs:325-336 |
| FND-004 | low | Nothing tests the Behavior bullet that the trace-tag walk asks about drops "before any absent-model check". `test_traces` is exercised only with a one-drop scope, so moving its check after the `TraceabilityModelUndeclared` return survives `fr029_dropped_declarations` (mutation run). Add an all-dropped scope to the direct-readers test. | rust/crates/quoin-quire/src/coverage.rs:165-170; rust/crates/quoin-quire/tests/fr029_dropped_declarations.rs:267-295 |

## Coverage

- FR-029-AC-15: tagged (3). `a_coverage_run_over_a_dropped_declaration_is_qq_1022_not_qq_1020`, `the_records_keep_the_engine_order`, and the unit test `a_coverage_drop_error_maps_to_the_typed_qq_1022_refusal`. Correct binding.
- FR-029-AC-16: tagged (1). It asserts the per-line rendering and the code. Correct.
- FR-029-AC-17: tagged (1). The real unreadable section yields `traceability_section` / `traceability`. Correct.
- FR-029-AC-18: tagged (1). It asserts the class `Refused`, exit 2, `quire_code`, and `dropped_declarations` in engine order with a non-alphabetical pair. Correct.
- FR-029-AC-19: tagged (2). A unit test (QQ-1110 mutant killed) and the integration test through design_join, the design matrix and the flow report. Correct.
- FR-029-AC-20: tagged (2). Coverage, evidence, flow report and assurance export are tested with all declarations dropped, and the walk-order mutant is killed. formal-rules status is not covered (FND-001).
- FR-029-AC-21: tagged (15). One per command: matrix, evidence audit, evidence record, advise, class obligations, anomaly check, soup promote, release snapshot, assurance, dossier generate/check/export, formal-rules status/coverage and quoin-suite-trace. Each asserts exit 2, `QQ-1022` and the drop line on stderr, empty stdout, and no written output.
- FR-029-AC-22: tagged (2). The trace-tag walk, the evidence compute and `assurance::build` (library), plus `assurance_declarations` (unit). formal-rules status is tested at the CLI under AC-21.
- FR-029-AC-23: tagged (13). formal-rules coverage is missing (FND-002), and the anomaly check test is weak (FND-003).
- Semantic review: skipped; not requested.
- Plan completion: not assessed

## Verdict

CONDITIONAL. Every criterion is tagged and the tests assert real exit codes, stderr content, empty stdout and files left unwritten. The gaps are two untested branches of AC-20 and the Behavior ordering rule, one AC-23 command that cannot pass because of a pre-existing defect, and one loose AC-23 oracle.

## Dispositions

Round 1, fix head 7021f18a. Verified on the branch, with the focused runs listed in SR-1854004 Dispositions, and mutation re-checks at 7021f18a: moving formal-rules `status`'s check after the absent-model answer now fails `formal_rules_status_over_an_all_dropped_model_is_qq_1022_not_qq_1020`, and moving `test_traces`'s check after the `TraceabilityModelUndeclared` return now fails `the_trace_tag_walk_refuses_an_all_dropped_model_with_qq_1022`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `formal_rules_status_over_an_all_dropped_model_is_qq_1022_not_qq_1020` asserts exit 2, empty stdout, QQ-1022 present, QQ-1020 absent, and the `traceability_section` line; ordering mutant killed. |
| FND-002 | fixed | `formal_rules_coverage_without_a_dropped_declaration_reports` asserts exit 0 and a JSON object on stdout; it passes now that the obligation projection carries `document`. |
| FND-003 | fixed | The anomaly clean run installs a `hazard` object-type module and asserts exit 0 and non-empty stdout. |
| FND-004 | fixed | `the_trace_tag_walk_refuses_an_all_dropped_model_with_qq_1022`; ordering mutant killed. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `the_properties_classification_refuses_a_dropped_declaration` is a library test of `properties::classify` but is tagged FR-029-AC-21, whose criterion is about a CLI command's exit status, stderr and stdout. Likewise `the_trace_tag_walk_refuses_an_all_dropped_model_with_qq_1022` also carries FR-029-AC-20, whose list of computes does not name the trace-tag walk. Both inflate matrix bindings for criteria they do not assert. Tag the properties test with no AC, or with one the spec adds for `quire.properties`, and keep the walk test on AC-22. | rust/crates/quoin-quire/tests/fr029_dropped_declarations.rs:297-326 |
