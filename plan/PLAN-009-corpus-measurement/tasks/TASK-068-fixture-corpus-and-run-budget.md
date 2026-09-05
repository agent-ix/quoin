---
id: TASK-068
title: "The reproducibility fixture corpus and the run budget"
type: Task
status: todo
track: E
priority: P1
relationships:
  - target: "ix://agent-ix/quoin/NFR-021"
    type: references
  - target: "ix://agent-ix/quoin/NFR-022"
    type: references
  - target: "ix://agent-ix/quoin/TC-1557"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1558"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1559"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1560"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1561"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1562"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1584"
    type: verifies
---

# TASK-068: The reproducibility fixture corpus and the run budget

## Scope

Commit a fixture corpus under `tests/fixtures/corpus-measurement/` exercising
each state, each outcome and each exclusion rule, so the reproducibility claim is
made over a population anybody can reconstitute rather than over one developer's
workspace. Assert digest equality across repeated runs, across a shuffled
enumeration order and with networking disabled, and record the machine every
timing was measured on.

## Subtasks

- [ ] Build the fixture corpus: repositories with and without an origin, a dirty tree, a worktree link, a nested repository, a symlink, documents of each state and outcome.
- [ ] Assert two runs write digest-identical artifacts apart from the run manifest timestamp.
- [ ] Assert a shuffled enumeration order produces the same digests.
- [ ] Assert an offline run produces the same digests.
- [ ] Record CPU count, memory and platform in the run manifest, and assert the wall-clock and memory budgets against them.
- [ ] Assert every corpus and module file is opened read-only.
- [ ] Assert every repository outside the reproducibility claim is marked `clean: false` or `stable: false`.

## Deliverables

- A committed fixture corpus, and timing figures that name the machine they were taken on.

## Notes

- The property and benchmark harnesses these rows need do not exist in this repository yet (SR-148 FND-1481). Either they are stood up here or the affected rows say so; they are not quietly retyped `Unit`.
