---
id: TASK-069
title: "Run the measurement, publish, file findings and close"
type: Task
status: todo
track: E
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-067"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-068"
    type: depends_on
  - target: "ix://agent-ix/quoin/US-022"
    type: references
---

# TASK-069: Run the measurement, publish, file findings and close

## Scope

Run the measurement over the pinned governed corpus, publish the artifacts and
the report, file an issue for every contract defect and tool defect it found with
the owner named, re-run until every finding carries a class and a disposition, and
close the cycle with a code review, a gap analysis and a pull request.

## Subtasks

- [ ] Run the full measurement and retain the raw results.
- [ ] Classify every finding; re-run until the `unknown` and `undispositioned` counts are either zero or explicitly accepted by the owner.
- [ ] File one issue per contract defect and per tool defect, naming the owning repository.
- [ ] Publish the report under `analysis/corpus-measurement/` with its manifest and digests.
- [ ] Run `/code-review` and `/gap-analysis` and apply every finding.
- [ ] Open the pull request with a mergeable comment; do not merge and do not publish.

## Deliverables

- The published advisory measurement, and a promotion gate that has evidence to consume.

## Notes

- A high failure rate pauses promotion. It does not justify weakening the validator, and no corpus repository is edited by this task.
