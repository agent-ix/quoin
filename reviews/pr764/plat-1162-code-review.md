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
