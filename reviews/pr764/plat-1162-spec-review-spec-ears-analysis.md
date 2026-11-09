---
id: SR-198
title: "Spec review (EARS) — PLAT-1162 FR-110 plan intake (quoin PR 764)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-110-protected-apparatus.md (Plan intake, FR-110-AC-1), spec/matrix.md (FR-110, FR-108, TC-1810 rows)"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-110", type: references}
---

## Summary

Ticket: PLAT-1162. PR head: `ac41053`. This review replaces SR-186 on the closed PR #689. It examines the edited FR-110 Plan intake statements and AC-1, and compares them with the code and the locked EA schema. `quire validate` on FR-110 was not separately broken by this diff. The matrix rows were read for consistency only.

## Verdict

**FAIL**. The defect SR-186 FND-001 found on #689 has come back in the rebase. The FR also understates the gate definition that EA requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "A gate over independently produced, admitted evidence MAY omit `protected_apparatus`" cannot be decided at plan intake, which runs before any evidence exists. The code's real trigger is "no `execution_procedure` and no `apparatus-edit` control". The statement does not say that, and it does not name apparatus-edit in this sentence. SR-186 FND-001 found the same defect on #689, and the rebase restored it. | spec/functional/FR-110-protected-apparatus.md:63-68 |
| FND-002 | medium | The new Plan intake bullet and AC-1 require only `ground_truth_kind`, `statistical_design` and `decision_rule`. The EA schema they claim to match also requires `population`, `sampling`, `repetitions`, `error_model` and `uncertainty` in a non-retired gate's design. So the FR makes the Quoin/EA disagreement in SR-196 FND-002 normative. | spec/functional/FR-110-protected-apparatus.md:57-60; spec/functional/FR-110-protected-apparatus.md:175 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-110 Plan intake bullet 1 (gate definition) | examined | Narrower than EA (FND-002) |
| FR-110 Plan intake bullet 2 (apparatus/controls) | examined | Undecidable trigger (FND-001) |
| FR-110-AC-1 | examined | Testable; inherits FND-002 |
| spec/matrix.md FR-108/FR-110/TC-1810 rows | context_only | Binding issue recorded in SR-197 FND-002 |
