---
id: SR-20261111182228
title: "Gap analysis of quoin PR 805 (FR-130 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@2747fe6a; FR-130 criteria, spec/matrix.md, rust/crates/quoin-evidence/src/scenario, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs"
review_set: subset
---

# Gap analysis of quoin PR 805

## Summary

Ticket: PLAT-78. Plan completion: not assessed. The diff was measured with
`quire matrix --scope .` (quire 0.40.0, engine 0.56.0; quoin 0.30.0). FR-130-AC-1 through
AC-9 are each `tagged`, AC-1..AC-8 in `quoin-evidence/src/scenario/tests.rs`, AC-9 (with
AC-1 and AC-3) in `tc_2208` of `tc_458_evidence_boundary.rs`. FR-130-CON-1 is Inspection
(`method-without-symbol`, as specified). The targeted tests pass.

TC-2200..TC-2208 and FR-130 do not occur on origin/main. Open PRs #802, #803 and #804 add
no TC-22xx id; #804 adds FR-125, so FR-130 does not collide.

The intake's shape was compared read-only with quire-protocol FR-016 and FR-017 and its
`bounds::inventory::BudgetDimension`. The 20 dimension labels match FR-016 exactly, in
spelling and in count. The kebab-case outcome kinds `row`, `refusal` and
`resource-incomplete` match FR-017 `typed_outcome`. The `full`, `partial` (reached plus
causing entry) and `legacy-compatibility` (owner, contract, immutable revision) shapes match
FR-017 `resolution_state`. The camelCase member names (`resolution`, `valueOwners`,
`outcome`, `budget[].granted/consumed`, `traceBinding`) differ from FR-017's prose
property names (`resolution_state`, `value_owners`, `typed_outcome`, `budget_granted` /
`budget_consumed`, `trace_binding`). That is not a defect: the intake wire is Quoin's
contract and FR-017 names properties, not a serialization. The Protocol owner should
confirm the spelling. One more point needs owner confirmation and is not a defect: an
FR-012 version-lock entry also carries `immutability_assertion`, `content_digest` and
`status`. `PinnedEntry` admits only `owner`, `contractVersion` and `revision`, under
`deny_unknown_fields`, which matches the ticket's "exact owner/contract/revision
attribution".

The ticket's "immutable accepted revision" is satisfied by the published owner `quoin`
plus contract `quoin.protocol-scenario-intake/v1`. The revision that carries the intake is
the Quoin release a consumer depends on through its manifest. Neither the spec nor the code
invents a pin or SHA for it, and none should be recorded.

## Verdict

**PASS WITH FINDINGS (one medium, one low).** Coverage is complete by tag, but the
contractual `refusal` context key that AC-9 and the Outputs section promise is asserted by
no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test asserts the `refusal` context key that `map_error` adds. TC-2208 checks that stderr contains `wrong-contract` and `QE-E010`, but the Display message ("scenario evidence refused (wrong-contract): ...") already carries the kind. The test still passes if `.with_context("refusal", ...)` is deleted. Parse the stderr diagnostic and assert `context.refusal == "wrong-contract"` and `context.evidence_code == "QE-E010"` | rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs:634-647, rust/crates/quoin-core/src/ops/evidence/taxonomy.rs:53-59 |
| FND-002 | low | The FR-130 matrix row cites `quoin-evidence` `scenario.rs`, but the module is `scenario/mod.rs` and its tests are in `scenario/tests.rs` | spec/matrix.md:138 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-130-AC-1, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A complete record is filed under `scenarios/<integration>/<scenario>.json` and the same record again is accepted without a rewrite."}
- {id: FR-130-AC-2, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A `partial` resolution and a `legacy-compatibility` selection are each admitted."}
- {id: FR-130-AC-3, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A document naming another contract version, or none, is refused before its shape is read, as `wrong-contract` or `missing-component`."}
- {id: FR-130-AC-4, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A missing member, an unknown member, a blank component and an empty budget are each refused."}
- {id: FR-130-AC-5, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A value owner that the resolution state does not hold at the same owner, contract version and revision, a scenario of another integration, a scenario id that is not path-safe, and a causing entry also listed as reached are each refused as `cross-wired`."}
- {id: FR-130-AC-6, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A repeated resolution entry, a value with two owners and a repeated budget dimension are each refused as `duplicate-component`."}
- {id: FR-130-AC-7, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "The budget is stored per dimension, a scalar budget is refused, and an absent grant stays absent."}
- {id: FR-130-AC-8, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "A second, different record for one integration and scenario is refused and the first is unchanged."}
- {id: FR-130-AC-9, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "An accepted record is answered with the intake contract version and owner, and a refusal exits with the bad-request status carrying `QE-E010` and the refusal kind."}
- {id: FR-130-CON-1, path: spec/functional/FR-130-protocol-scenario-evidence-intake.md, role: examined, excerpt: "The intake SHALL accept a typed document and read no transcript, file or environment."}
bindings:
- {test_id: a_complete_record_is_filed_under_its_integration_and_scenario, ac_id: FR-130-AC-1, trace: correct}
- {test_id: partial_and_legacy_shapes_are_admitted_without_a_full_manifest, ac_id: FR-130-AC-2, trace: correct}
- {test_id: a_wrong_or_absent_contract_version_is_refused_before_the_shape_is_read, ac_id: FR-130-AC-3, trace: correct}
- {test_id: a_missing_field_an_unknown_field_and_a_blank_component_are_refused, ac_id: FR-130-AC-4, trace: correct}
- {test_id: a_cross_wired_attribution_is_refused, ac_id: FR-130-AC-5, trace: correct}
- {test_id: duplicate_entries_values_and_dimensions_are_refused, ac_id: FR-130-AC-6, trace: correct}
- {test_id: the_budget_is_per_dimension_and_an_absent_grant_is_not_zero, ac_id: FR-130-AC-7, trace: correct}
- {test_id: a_second_different_record_for_one_scenario_is_refused_and_the_first_kept, ac_id: FR-130-AC-8, trace: correct}
- {test_id: tc_2208_the_scenario_intake_answers_a_versioned_result_or_a_typed_refusal, ac_id: FR-130-AC-9, trace: correct}
```
