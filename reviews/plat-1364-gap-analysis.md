---
id: SR-16394
title: "Gap analysis of quoin PR 777 (semantic-module fixtures from quire-fixtures)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@a4ffd114; PR 777 diff against PLAT-1364 intent: FR-071-CON-2, FR-072, FR-073-AC-1, FR-074, FR-104, spec/matrix.md, tests reading the module-ok and mapping fixtures"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-071"
    type: "reviews"
---

# Gap analysis of quoin PR 777

## Summary

Ticket: PLAT-1364. This is a planless audit (plan completion was not assessed). The ticket's
intent: remove quoin's divergent copy of the mapping fixtures and read the authoritative ones
from `quire-fixtures`, with no vendoring, after first confirming the dependency is usable.

- Copy removed: `tests/fixtures/semantic-module/` is deleted. Outside historical `reviews/` files,
  the only remaining references to the old path are the quoin-jev eval corpus (eval data, left
  as is) and a checked TASK-041 item (a historical plan record).
- Dependency usable: `quire-fixtures` resolves at quoin's existing quire-rs rev `ce172a7`, in
  dev-dependencies only (`cargo tree -e normal,build -i quire-fixtures` is empty).
- Tests read the dependency: tc_446, tc_452_module_install_criteria, tc_452_semantic_boundary,
  quoin-semantic `common`, contract_gate unit tests and tc_module_command. All 45 focused tests
  pass against the repo-pinned semantic contract.
- Trace bindings checked: tc_452_616 still targets FR-073-AC-1. tc_452_650 targets FR-071-AC-1
  and tc_452_651 targets FR-071-AC-2. tc_452_654 targets FR-074-AC-1/AC-2/CON-1 and keeps its
  population floors.

## Verdict

**PASS WITH FINDINGS (one low).** The PR meets the intent. TC-1352 was reworded to describe
another repo's content instead of the restated obligation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-1352 now reads "Golden fixtures in `quire-fixtures` carry expected diagnostics" and is marked Complete. FR-071-CON-2 now obliges quoin to read the goldens from quire-fixtures and hold no copy. The TC describes a property of quire-rs's content, not quoin's obligation, which is met by the dev-dependency and the absence of a local copy | spec/matrix.md:946 |
