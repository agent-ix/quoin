---
id: SR-199
title: "Code review (Rust lane) — PLAT-1162 evidence gates without apparatus inventories (quoin PR 764, re-review)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-measurement/src/plans.rs, rust/crates/quoin-measurement/src/plans/apparatus.rs, rust/crates/quoin-measurement/tests/, rust/crates/quoin-core/tests/tc_478_measurement_intake.rs, rust/crates/quoin-measurement/Cargo.toml, rust/Cargo.lock, spec/assurance/MP-216..MP-219, MP-228..MP-230"
review_set: subset
---

## Summary

Ticket: PLAT-1162. PR head: `be4803e6`. An independent re-review of the whole PR diff (`git diff origin/main...be4803e6`), with the Rust lane (rust-style) folded in. It does not replace SR-196. SR-196 FND-003 (MP-228's typed rule leaves out its 3-of-5 repeated-run GO condition) is still open at this head. It is checked again here and not filed a second time.

The core change is correct against engineering-assurance FR-024 (origin/main) and the locked EA schema (`48e3763`). `apparatus_from` now requires `negative_controls` only on a non-retired gate (FR-024-AC-4, AC-9). It no longer requires `protected_apparatus` because of `stage: gate` alone (FR-024-AC-8). The `apparatus-edit`-needs-apparatus refusal is unchanged. The `execution_procedure` coverage check in `plan_from` still refuses a procedure that has no apparatus, at any stage. `validate_gate_definition` adds the gate requirements for `ground_truth_kind` and the seven `statistical_design` members, matching the schema's non-retired gate `then` and `$defs/statistical_design.required`.

Checks run: `cargo test -p quoin-measurement --locked`, every test passed (tc_975 27, tc_958 26, tc_960 18, tc_479 23), `quire validate` on the seven edited plans and FR-110 (quire 0.36.2, exit 0, no findings). Not run: the full workspace test, clippy, cargo-deny, and the `quoin-core` `tc_478` fixture edit. Those need the `quoin-semantic` build, which needs `node_modules/@agent-ix/semantic-core` from npm.ix.

## Verdict

**PASS with low findings.** The runtime change and its tests are right. The two findings below are low. SR-196 FND-003 is still open and blocks the merge until it is fixed or the MP-228 owner accepts the weaker rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `validate_gate_definition` checks only that a member is present: a non-empty string or any non-null value. A gate stating `population: 5`, `sampling: []` or `error_model: {}` loads in Quoin, but EA's schema refuses it because those members are `type: string, minLength: 1`. Quoin and EA still disagree on what counts as a valid gate, though less than before. No test covers a member of the wrong type. | rust/crates/quoin-measurement/src/plans.rs:258-266 |
| FND-002 | low | The seven edited gate plans gain `statistical_design.estimator` and `decision_rule` but keep their `definition_version`. EA's definition-change check counts the estimator and the decision rule as part of the definition. If any collection is stored under these versions (none is in this repo), FR-108's tamper check reports `definition_changed_without_version_bump` for them. | spec/assurance/MP-216-guidance-correctness.md:9-20; MP-217..MP-219, MP-228..MP-230 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `apparatus_from` gate branch (apparatus.rs:64-69) | examined | Correct: controls required only on a non-retired gate; apparatus not required by stage |
| `apparatus-edit` with no apparatus (apparatus.rs:70-82) | examined | Unchanged and still refused; tested in tc_975_001 |
| `execution_procedure` coverage in `plan_from` (plans.rs:188-203) | examined | Unchanged; tc_975_025 covers the gate case |
| `validate_gate_definition` (plans.rs:224-268) | examined | Matches the schema's required members; checks presence, not type (FND-001) |
| EA FR-024 Description, Behavior, AC-4, AC-8, AC-9, AC-10 (origin/main) | context_only | Quoin matches |
| Locked EA schema at 48e3763 (allOf, $defs/statistical_design) | context_only | Quoin matches on presence |
| spec/assurance MP-216..219, 228..230 | examined | quire validate passes; MP-228 rule still open (SR-196 FND-003); no version bump (FND-002) |
| tc_958 / tc_960 / tc_479 / tc_478 fixture edits | examined | Consistent; tc_958_021 builds a rule-less gate by changing `stage` after load, which is acceptable |
| tc_975_022..027 | examined | Real behaviour; tc_975_022 runs `cargo metadata --locked` to find the locked EA schema by package name, with no SHA restated |
| `ix-trace-rs` dev-dependency | examined | Workspace dependency, same as sibling crates |

Rust lane (rust-style): no panics on caller data in the changed production code, errors use the existing `MeasurementErrorCode::PlanInvalid` code, no new public API, no unsafe code. The `has_ground_truth: bool` parameter is a small readability nit and not recorded as a finding.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | `cargo clippy -p quoin-measurement --all-targets --all-features --locked -- -D warnings` fails on the pinned toolchain (1.98.1) at PR head 0d07018c. tc_975_024 builds two strings with `.map(\|line\| format!("{line}\n")).collect()`, which is `clippy::format_collect` (workspace `pedantic = warn`, made an error by `-D warnings`). `make rust-lint` therefore fails, and merging would leave main red. The PR body's clippy claim predates ffdd4209, which added this test. | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:448-452; rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:459-463 |
| FND-004 | high | The PR is CONFLICTING with main (4c174e58). A trial merge conflicts in `spec/matrix.md` (FR-108 and FR-110 rows). With the conflict set aside, the merged tree does not compile: tc_975_028's `plans[0].statistical_design.expect(...)` is E0507, because main's EA bump made `StatisticalDesign` no longer `Copy`. A borrow or clone fixes it. With that one-line change, all 28 tc_975 tests and the rest of quoin-measurement pass on the merged tree. | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:572; spec/matrix.md:135-136 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | At 0d07018c `validate_gate_definition` still checks only for a non-empty string or any non-null value (plans.rs:258-266). `design::statistical_design_from` type-checks only the counts, `estimator` and `decision_rule`, so `population: 5` or `sampling: []` still loads in Quoin while EA's `$defs/statistical_design` (string, minLength 1) refuses it. Low; does not block merge |
| FND-002 | rejected | No collection under any of these seven definition versions exists in this repository (`spec/evidence/measurements` is absent at 0d07018c), so the tamper check has nothing to report. These plans state a design for the first time; before this PR, no plan had a design for the check to compare against. A version bump guards no existing evidence |
