---
id: TASK-066
title: "Finding identity, classes, owners and dispositions"
type: Task
status: todo
track: D
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-089"
    type: references
  - target: "ix://agent-ix/quoin/TC-1531"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1532"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1533"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1534"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1535"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1536"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1537"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1575"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1576"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1577"
    type: verifies
---

# TASK-066: Finding identity, classes, owners and dispositions

## Scope

Partition every finding — every `fail`, every advisory finding, every module
finding, every contested-type finding — into exactly one of the eight classes with
a named owner and a disposition. The identity is the tuple of repository, path,
check and diagnostic code, deliberately excluding the line number so a moved line
does not silently lose its classification. A ledger entry that matches nothing is
reported, and every unmatched finding is `unknown` and `undispositioned`.

## Subtasks

- [ ] Implement the finding identity and the ledger match, prefix matching included.
- [ ] Implement the assignment rule for each of the eight classes, each requiring its ledger evidence.
- [ ] Refuse a disposition missing its required nomination: later campaign, module repository, or accepting human.
- [ ] Refuse an owner that is absent or a bare role.
- [ ] Report `unmatched-ledger-entries`, and publish the `unknown`, `undispositioned` and unmatched counts as headline figures.
- [ ] Assert per-class counts sum to the finding count and no finding is dropped by any classification.

## Deliverables

- A partition in which every finding has somebody's name on it, and every gap is a number rather than a silence.

## Notes

- Every rule is written so an unstated case raises the `unknown` count rather than lowering it. A hand-maintained ledger drifts; the report has to make the drift visible.
