---
id: SR-139
title: "Specification integrity review — risk decisions PR #1048"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; changed spec/functional/FR-142, FR-145, FR-191, FR-199, FR-213, FR-214 and spec/non-functional/NFR-031"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-213", type: references }
  - { target: "ix://agent-ix/quoin/FR-214", type: references }
  - { target: "ix://agent-ix/quoin/FR-199", type: references }
---

## Summary

Reviewed changed requirements for completeness, consistency, hidden assumptions, and criterion testability. The principal integrity issue is the unresolved conflict between FR-141 bulk promotion and the new FR-142 per-component clause rule.

## Verdict

**FAIL** — the linked SOUP requirements permit a normal workflow that the new join rule makes unable to resolve advisories.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-141 allows one `--discharges` id to be written to all newly promoted components, but FR-142 now treats any shared clause as unresolved. The requirements need a defined multi-component behavior before this workflow is coherent. | FR-141-AC-4; FR-142-AC-37 |

## Scope Examined

Reviewed the changed behavior and ACs for FR-142, FR-145, FR-191, FR-199, FR-213, FR-214 and NFR-031; checked the new FR dependency links and all new test bindings. No additional completeness or consistency defect found.


## Dispositions

| FND | outcome | reason | after_excerpt |
| --- | --- | --- | --- |
| FND-001 | fixed | FR-141 now refuses a multi-record promotion with --discharges before writing records; the requirements define a coherent path with FR-142's unique-clause rule. | `If --discharges is given and the promotion plans to create more than one new soup_record, then the command SHALL refuse before writing any record.` |
