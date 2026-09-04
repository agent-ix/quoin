---
id: TASK-051
title: "Code review, gap analysis and pull request"
type: Task
status: todo
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-050"
    type: depends_on
  - target: "ix://agent-ix/quoin/StR-008"
    type: references
  - target: "ix://agent-ix/quoin/US-021"
    type: references
---

# TASK-051: Code review, gap analysis and pull request

## Scope

Close the governed cycle: review the landed code, verify the plan is complete and
the matrix is backed by real tests, and open the pull request.

## Subtasks

- [ ] `make lint` and `make test` green in this repository.
- [ ] `quire validate --scope . "spec/**/*.md"` and `"plan/**/*.md"` structurally clean.
- [ ] `/code-review` over the branch.
- [ ] `/gap-analysis` over PLAN-008: every task done, every matrix row backed by a real test, every code path owned by a requirement.
- [ ] Flip every PLAN-008 matrix row from `🚧` to its true state, with a reason on anything still `🚧`.
- [ ] Open the pull request against `main` and comment when it is mergeable. Do not merge.

## Notes

- Landing is part of the work, but merging is the maintainer's.
