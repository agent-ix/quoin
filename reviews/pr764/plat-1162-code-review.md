---
id: SR-196
title: "Code review — PLAT-1162 independent evidence gates without apparatus lists (quoin PR 764)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-measurement/src/plans.rs, rust/crates/quoin-measurement/src/plans/apparatus.rs, rust/crates/quoin-measurement/tests/, rust/crates/quoin-core/tests/tc_478_measurement_intake.rs, rust/crates/quoin-measurement/Cargo.toml, rust/Cargo.lock, spec/assurance/MP-216..MP-219, MP-228..MP-230"
review_set: subset
---

## Summary

Ticket: PLAT-1162. PR head: `ac41053`. This review replaces SR-184 on the closed PR #689. Code review with the Rust lane folded in. Scope is the PR diff only.

`apparatus_from` no longer requires `protected_apparatus` on a gate. The new `validate_gate_definition` requires a non-retired gate to state `ground_truth_kind`, `statistical_design` and `statistical_design.decision_rule`. The existing `execution_procedure` coverage check is unchanged. An `apparatus-edit` control with no apparatus is still refused. That matches the conditional `allOf` branches of the locked EA schema (`48e3763`, which includes EA #181).

Tests run here: `cargo test -p quoin-measurement --locked` for `tc_975_protected_apparatus` (23), `tc_958_stage_verdicts` (26), `tc_960_statistical_design` (18) and `tc_479_fr044_fr045_criteria` (24). All passed. Not run: the full workspace gate, clippy, cargo-deny, and `quoin-core`'s `tc_478` (quoin-core needs the quoin-semantic build, which needs pnpm, and npm.ix returns 401 here). `quire validate` on the edited `spec/assurance` plans was run (0.36.2) and fails; see FND-001.

## Verdict

**FAIL**. The runtime change is correct and small. But the seven real gate plans the PR edits now fail the EA schema, which shows that Quoin's new gate check still admits plans EA refuses. The test also restates the EA lock SHA.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The seven active gate plans the PR edits fail `quire validate` and the locked EA schema. Each new `statistical_design` states only `estimator` and `decision_rule`. EA requires `population`, `sampling`, `repetitions`, `error_model` and `uncertainty` as well. Reproduced: `quire validate --scope . spec/assurance/MP-216-guidance-correctness.md` reports 5 missing members per plan. | spec/assurance/MP-216-guidance-correctness.md:11-15; MP-217, MP-218, MP-219, MP-228, MP-229, MP-230 :11-15 |
| FND-002 | medium | `validate_gate_definition` checks only that `statistical_design` and `decision_rule` are present. A non-retired gate whose design lacks EA's other required members loads in Quoin but is refused by EA, so Quoin and EA still disagree on what a valid gate is. FND-001 got through Quoin's tests because of this. | rust/crates/quoin-measurement/src/plans.rs:229-262 |
| FND-003 | medium | The PR invents decision rules (`proportion ge 1.0`) for seven real active gate plans and says in its body that they are placeholders. They come from no stated threshold, and the plan bodies already describe a population and method. A gate then decides pass or fail against a made-up bar. | spec/assurance/MP-216-guidance-correctness.md:11-15; MP-217..MP-219, MP-228..MP-230 :11-15 |
| FND-004 | medium | `tc_975_010` finds EA in `cargo metadata` only when its source ends with `#48e3763fc47c89cfd3b45f6e7b8a47b03ab10ab9`. That restates the Cargo.toml rev as a pin. It fails on every EA bump and checks nothing the schema validation does not. Selecting the package by name is enough. | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:229-237 |
| FND-005 | low | `apparatus_from` refuses any `gate` with no `negative_controls`, retired ones included. EA requires `negative_controls` only for a non-retired gate, so a retired gate EA accepts is refused by Quoin. | rust/crates/quoin-measurement/src/plans/apparatus.rs:63-68 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `apparatus_from` gate branch | examined | Apparatus no longer unconditional; retired divergence (FND-005) |
| `validate_gate_definition` | examined | Presence-only design check (FND-002) |
| `execution_procedure` coverage check in `plan_from` | examined | Unchanged; still refuses an unprotected procedure |
| `apparatus-edit` control with no apparatus | examined | Still refused; tested in `tc_975_001` |
| Locked EA schema `measurement-plan-frontmatter.schema.json` at 48e3763 | context_only | Conditional procedure/apparatus-edit branches match Quoin |
| Edited spec/assurance gate plans | examined | Fail EA schema (FND-001); placeholder rules (FND-003) |
| tc_958/tc_960/tc_479/tc_478 fixture edits | examined | Consistent with the new rule; tc_958_021 builds a rule-less gate by mutating `stage` after load, which is acceptable |
| `ix-trace-rs` dev-dependency and Cargo.lock | examined | Workspace dependency; no new pin |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | MP-228's design states no `minimum_population`, and the checker does not bind `examined` to `repetitions`. So an observation of 1 clearing pass out of 1 (or 2 of 2), stating `repetitions: 5`, recomputes to 1.0 and passes `ge 0.6`, although the plan's condition is 3 of 5 passes. `minimum_population: 5` would close it. MP-228's NO-GO is final and no producer writes its collections today, so nothing returns a wrong verdict right now. | spec/assurance/MP-228-jev-criterion-strength-gate.md:11-20; rust/crates/quoin-measurement/src/verify/rows.rs:199-212 |
| FND-007 | low | `tc_975_028` asserts plan data, not behaviour: `repetitions == 5`, and `"0.6"` found in the `Debug` string of the decision rule. A Debug substring match is brittle (it also matches `0.65`), and pinning a spec value in a test adds no functional coverage beyond "the real MP-228 loads as a gate". | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:565-576 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | ffdd4209 adds population, sampling, repetitions, error_model and uncertainty to all seven plans; `quire validate --scope .` on MP-216..219, MP-228..230 and FR-110 exits 0 |
| FND-002 | fixed | ffdd4209 `validate_gate_definition` requires every EA-required design member and names it; tc_975_024 removes each member in turn and asserts the refusal |
| FND-003 | still-open | Fixed for MP-216..219 (their text says "exactly 100% is required" / "any miss blocks") and faithful for MP-229 and MP-230, whose text defines a composite boolean that is GO only when every bar holds (proportion of bars met >= 1.0). Not faithful for MP-228: its Comparison and Enforcement section makes GO need all three bars on the gate run and in at least 3 of 5 repeated runs. The typed rule encodes only the gate run, `repetitions: 1` contradicts the five repeats, and the 3-of-5 condition sits only in `uncertainty` prose. So a `pass` from the rule is weaker than the plan's GO. The rule must encode the repeat condition, or the plan owner must confirm the weaker rule in the plan text |
| FND-004 | fixed | ffdd4209 selects the EA package by name only; the SHA filter is gone |
| FND-005 | fixed | ffdd4209 exempts a retired gate in `apparatus_from`; tc_975_026 loads a retired gate with no controls |
| FND-003 | fixed | 0a0f6d26: MP-228 now encodes the repeat condition as `repetitions: 5`, a proportion of passes that clear all three bars, and `ge 0.6`, which is exactly 3 of 5 when the five passes are the population. The gate-run requirement cannot be typed, because the schema has one rule over one estimate. It is stated in both `uncertainty` and Comparison and Enforcement, so the plan says the typed `accept` is necessary but not sufficient for GO. The remaining hole is recorded as FND-006 |
| FND-006 | fixed | 0d07018c adds `minimum_population: 5` to MP-228's design; tc_975_028 publishes a 1-of-1 observation stating `repetitions: 5` and intake refuses it |
| FND-007 | fixed | 0d07018c replaces the Debug substring with `assert_eq!(design.decision_rule, Some(DecisionRule::against_threshold(Comparator::Ge, 0.6)))` and adds behaviour: 3 of 5 accepts, 2 of 5 rejects through `verify` |
