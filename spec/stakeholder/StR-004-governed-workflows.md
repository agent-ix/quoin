---
id: StR-004
title: "Review, matrix, and planning run as governed workflows"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-020"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "satisfied_by"
---

# StR-004: Review, matrix, and planning run as governed workflows

## Stakeholder Need

Teams running the spec lifecycle require that `quoin` shall start the review,
matrix, and planning workflows as governed runs whose phase progression and human
approval gates are managed by a dedicated workflow engine, so that these
multi-step, human-in-the-loop processes are tracked and resumable rather than
ad hoc.

## Rationale

Review, test-matrix construction, and planning are staged processes that pause for
human judgement and may span multiple sessions. Running them as untracked
one-shot commands would lose progress and skip the approval gates that make the
outputs trustworthy. Delegating lifecycle control to a workflow engine (`ix-flow`)
gives each run an identity, durable state, and explicit gates, while `quoin`
remains the single launch point an author already knows.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-004-VC-1 | An author starts a review, matrix, or planning workflow through `quoin` and then drives its progression — resume, advance, acknowledge gates, inspect status — through the workflow engine, with the run's state persisted across invocations. | Inspection |

Satisfaction is demonstrated by launching a workflow and continuing it to completion through the engine.

## Stakeholders

The primary stakeholders are spec authors and reviewers running the lifecycle;
the workflow-engine maintainers are an affected party that owns post-launch
control.

## Dependencies

**Downstream**: the workflow-launcher and run-spawning functional requirements
([FR-020](../functional/FR-020-resolve-workflow-skills.md),
[FR-021](../functional/FR-021-launch-ix-flow-runs.md)). Post-launch lifecycle is
owned by `ix-flow`.
