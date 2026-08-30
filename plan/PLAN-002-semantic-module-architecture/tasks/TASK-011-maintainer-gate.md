---
id: TASK-011
title: "Open the architecture PR and stop at maintainer review"
type: Task
track: "Promotion gate"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: "part_of"
  - target: "ix://agent-ix/quoin/NFR-014"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-010"
    type: "depends_on"
---

# TASK-011: Open the architecture PR and stop at maintainer review

## Status

**pending**

## Scope

Commit and push the reviewed architecture branch, open a PR linked to issue #289, move the project
item to In review, and present the exact normative decisions and evidence to Quoin/Quire maintainers.
Do not merge until named maintainers approve the record.

## Exit criteria

- PR is open with validation, code-review, gap-analysis, scope, and decision-status evidence.
- Project 18 marks issue #289 In review.
- TC-1155 remains open until named maintainer approval is recorded.
- No merge is attempted during this task without that approval.
