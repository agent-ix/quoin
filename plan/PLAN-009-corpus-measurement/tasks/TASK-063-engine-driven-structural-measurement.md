---
id: TASK-063
title: "Structural conformance through the Quire engine"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-087"
    type: references
  - target: "ix://agent-ix/quoin/TC-1518"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1519"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1520"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1521"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1522"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1523"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1524"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1572"
    type: verifies
---

# TASK-063: Structural conformance through the Quire engine

## Scope

Run the Quire engine over each measured document against the resolved module
set and record its diagnostics. The engine decides conformance; the measurement
records what it said. An error is a `fail`, a warning is an advisory finding and
never a failure, an abnormally terminated batch is `could-not-run` for every
document in it. The resolved module set must be the only module source the engine
sees, so that no installed catalog copy or repository-local module can supply a
contract the run did not pin.

## Subtasks

- [ ] Invoke the engine with `--diagnostics-format json` and the resolved module set as its only module source.
- [ ] Prove the isolation: a repository-local module declaring the same type must not change an outcome.
- [ ] Record every diagnostic against its document — code, severity, reason, line and message — in one evaluation record.
- [ ] Record `pass` on no error, `fail` on any error, `could-not-run` on an absent outcome or an abnormal termination.
- [ ] Record the engine version and source revision beside the outcomes.
- [ ] Batch documents so a full corpus run stays inside the NFR-022 budget.

## Deliverables

- An evaluation record per measured document, attributable to an exact engine revision.

## Notes

- This is the finding three of the eight reviews reached independently: Quire owns validation, the epic forbids a parallel replacement, and the mapping semantics a reimplementation would get wrong are English prose in the modules.
- The engine search order was checked rather than assumed: `IX_FILAMENT_MODULES_PATH` takes precedence over both a repository-local module and the install root.
