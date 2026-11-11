---
id: SR-20261111182229
title: "Code review of quoin PR 806 (constant-predictor answer families)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@bf12d74d; rust/crates/quoin-measurement/src/plans/design.rs, rust/crates/quoin-measurement/src/verify/{constant_predictor,mod,reason}.rs, rust/crates/quoin-measurement/tests/{tc_1016_constant_predictor_intake,tc_958_stage_verdicts}.rs, rust/crates/quoin-jev/tests/tc_1016_constant_predictor.rs"
review_set: subset
---

# Code review of quoin PR 806

## Summary

Ticket: PLAT-1020. Scoped to `git diff origin/main...HEAD` at bf12d74d. Rust lane
(`rust-review`, with the repo's `.claude/skills/rust-style`) folded in. Ran via
locked-build.sh (debug=0, jobs=4): `cargo test --locked -p quoin-measurement -p quoin-jev`
(all pass, including `tc_1020_001`, `tc_1020_002`, `tc_1016_006`, `tc_958_021` and
`item_row_families_must_equal_the_declared_list`), `cargo clippy -p quoin-measurement -p
quoin-jev --all-targets --all-features -D warnings` (clean), `cargo fmt --all --check` (clean).
A throwaway probe test (deleted, never pushed) confirmed FND-001.

Clean: set semantics are exact on both sides. `AnswerFamilies` (EA rev ce36020, the existing
workspace pin) is a `BTreeSet<AnswerFamily>` that refuses empty lists, empty entries and
duplicates and does not normalise. The rows' families are read as written and grouped in a
`BTreeMap`, so duplicate family values across rows collapse to one key. Case or whitespace
variants, renames, a merge into an undeclared name, a split, and a superset all fail closed
as `family_unexpected`. A subset, or a merge into a declared name, fails closed as
`family_missing`. The two checks run in a fixed order over ordered collections, so the
result is deterministic. The check runs after the row scan and before `examined`
consistency and `best_constant`, so no baseline value is ever computed on a mismatch. In
`Check::decide`, a baseline `Err` records the reason for the candidate and `holds` stays
`None` on both the point and interval paths. All three reasons map to
`Verdict::Inconclusive`, so a family mismatch cannot produce pass or fail. EA types are
imported, not copied, and no Cargo file changes. Removing `slice` from `verify::baseline()`
is behaviour-preserving: it was always `observation.dimensions.entries()`. The
`tc_1016_006` edit (drop one coverage row, not four) keeps the RowsMismatch assertion. The
old whole-family drop is now covered as `family_missing` in `tc_1020_001`, which adjusts
`examined` so the mismatch cannot mask the result. The fixtures that gain families
(`tc_958_021`, the quoin-jev plan) still assert their original outcomes.

## Verdict

**PASS WITH FINDINGS (one medium).** The checker is correct. The load-time refusal also
applies to the history parser, so the `definition_change_without_version_bump` tamper walk
silently skips constant-predictor plan revisions committed before this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new refusal lives in `statistical_design_from`, which `plan_from_text` (the history parser) shares. `quoin-cli` `tamper::definition_changed` silently skips any revision that fails to parse. A constant-predictor plan revision committed without `answer_families`, which was valid before this PR, is now unreadable as history, and the probe confirms it returns `QM-PLAN-INVALID`. A same-version rule edit between such revisions (for example a lowered margin), or adding `answer_families` without a `definition_version` bump, is no longer reported as `definition_changed_without_version_bump`, which drops a Reject. Apply the requirement only on current-plan load (for example a `PlanLoadOptions` flag that `plan_from_text` leaves off) and add a history test for a pre-families revision | rust/crates/quoin-measurement/src/plans/design.rs:173-188, rust/crates/quoin-measurement/src/plans/history.rs:38-51, rust/crates/quoin-cli/src/measurement/tamper.rs:284-288 |

## Examined scope

```yaml
clean: false
scope:
- {id: statistical_design_from, path: rust/crates/quoin-measurement/src/plans/design.rs, role: examined, excerpt: "if constant_predictor && answer_families.is_none() { return Err(MeasurementError::new(CODE, ... \"statistical_design.answer_families is required when decision_rule.baseline is constant-predictor\")) }"}
- {id: check_families, path: rust/crates/quoin-measurement/src/verify/constant_predictor.rs, role: examined, excerpt: "let declared = declared.ok_or(Reason::ConstantPredictorFamiliesUnstated)?; ... if by_family.keys().any(|family| !declared.contains(family)) { return Err(Reason::ConstantPredictorFamilyUnexpected); } if declared.iter().any(|family| !by_family.contains_key(family)) { return Err(Reason::ConstantPredictorFamilyMissing); }"}
- {id: constant_predictor::baseline, path: rust/crates/quoin-measurement/src/verify/constant_predictor.rs, role: examined, excerpt: "if total == 0 { return Err(Reason::ConstantPredictorRowsAbsent); } check_families(families, &by_family)?;"}
- {id: verify::baseline, path: rust/crates/quoin-measurement/src/verify/mod.rs, role: examined, excerpt: "let slice = observation.dimensions.entries(); ... constant_predictor::baseline(collection, observation, answer_families)?"}
- {id: Check::decide, path: rust/crates/quoin-measurement/src/verify/mod.rs, role: examined, excerpt: "if let Err(reason) = &resolved && is_candidate { record(*reason); } ... Some(Err(Refusal::Unevaluable)) if resolved.is_err() => None"}
- {id: Reason::verdict, path: rust/crates/quoin-measurement/src/verify/reason.rs, role: examined, excerpt: "| Self::ConstantPredictorFamiliesUnstated | Self::ConstantPredictorFamilyUnexpected | Self::ConstantPredictorFamilyMissing ... => Verdict::Inconclusive"}
- {id: tc_1016_006, path: rust/crates/quoin-measurement/tests/tc_1016_constant_predictor_intake.rs, role: examined, excerpt: "// Drop the last `coverage` row: 14 items against `examined: 15`. observations.truncate(15);"}
- {id: tc_958_021, path: rust/crates/quoin-measurement/tests/tc_958_stage_verdicts.rs, role: examined, excerpt: "\"    comparator: gt\\n    baseline: constant-predictor\\n  answer_families:\\n    - a\\n\""}
- {id: quoin-jev plan fixture, path: rust/crates/quoin-jev/tests/tc_1016_constant_predictor.rs, role: examined, excerpt: "answer_families: Some(AnswerFamilies::new([\"weakness_kind\", \"coverage\"]).unwrap()),"}
- {id: plan_from_text, path: rust/crates/quoin-measurement/src/plans/history.rs, role: context_only, excerpt: "plan_from(path, &value, options).map(Some)"}
- {id: definition_changed, path: rust/crates/quoin-cli/src/measurement/tamper.rs, role: context_only, excerpt: "let Ok(Some(revision)) = quoin_measurement::plans::plan_from_text(&path_then, &text, PlanLoadOptions::default()) else { continue; };"}
- {id: AnswerFamily::new, path: "engineering-assurance@ce36020 src/measurement.rs", role: context_only, excerpt: "if family.is_empty() { return Err(AnswerFamilyError::Empty); } Ok(Self(family))"}
```
