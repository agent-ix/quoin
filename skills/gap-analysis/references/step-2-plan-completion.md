# Plan Completeness (OPTIONAL — explicit plan only)

**Goal**: When — and only when — the caller explicitly supplied a plan, confirm that plan's
bookkeeping is sound: every unit of work is finished, done work completed in dependency
order, and `plan.md` agrees with its tasks.

## Gate first

**Skip this step entirely unless the caller named a plan** (`--plan Plan-001`, or an
equivalent explicit instruction — see [target selection](step-1-target-selection.md)). The
presence of a `plan/` directory is not an instruction. A planless run records
`Plan completion: not assessed` in `## Coverage` and produces no finding from this step.

## What this step is, and is not

This is **bookkeeping on top of a completed repository audit**, not the audit itself. It may
only add findings. It must never:

- narrow the matrix, reverse-gap, stub, or semantic steps to the plan's task set;
- suppress, downgrade, or omit a repository finding because it falls outside the plan;
- supply the scope, the requirement set, or the code surface for any other step;
- stand in for repository assurance — a plan whose tasks are all `done` says nothing about
  whether the matrix is backed or the code is traced.

Nor does it author: do not create or edit a plan, a `Task`, a requirement, or an acceptance
criterion, and do not fix a status you believe is stale.

## The Task contract

Tasks in the bundle are frontmatter-typed (`type: Task`). The machine contract
(see `quoin/skills/spec-to-plan/SKILL.md`):

```yaml
---
id: Task-001
type: Task
status: not_started | in_progress | blocked | done
track: A | B | C | S | J | <gate-name>
priority: P0 | P1 | P2 | P3
relationships:
  - { target: "ix://org/component/Task-000", type: depends_on }
  - { target: "ix://org/component/FR-002",   type: references }   # requirement ownership
  - { target: "ix://org/component/TC-004",   type: verifies }     # test-case trace
---
```

## The three checks

1. **All tasks done.** Read every `plan/<Plan-id>-<slug>/tasks/Task-*.md` and check `status`:
   - `done` → OK.
   - `not_started` / `in_progress` / `blocked` → **finding**. Severity:
     - `high` if `priority: P0`/`P1` or it is on the critical path (`track: A`/`S`).
     - `medium` otherwise. `blocked` → note the blocker (its unmet `depends_on`).
2. **Done-task dependency sanity.** Flag a `done` task whose `depends_on` target is **not**
   `done` (out-of-order completion) as a `medium` finding.
3. **Task status vs plan checkbox consistency.** `plan.md` mirrors requirements/tests as
   `- [ ]` / `- [x]`. Flag (`low`) any checkbox state that contradicts task status
   (e.g. an unchecked requirement whose owning task is `done`, or vice-versa) — the plan
   doc is drifting from its tasks.

Record the rollup: tasks `done` / total — feeds the SpecReview `## Coverage` line
`Plan completion: assessed (<Plan-id>) — tasks done X / Y`.

## Output of this step

A list of findings (incomplete tasks, out-of-order completion, checkbox drift) with their
`Refs` (the Task id, and the requirement/TC it owns) for the SpecReview Findings table, plus
the tasks-done/total count.

## Notes

- Do not edit task files or `plan.md` — gap-analysis only reports. If the user wants the
  plan reconciled, that's `spec-to-plan` (update flow) or `implement-plan`.
- A plan-assisted PASS still means the repository checks passed *and* the plan's books
  balance. The two claims stay separate in the artifact; do not merge them into one
  sentence that implies either proves the other.
