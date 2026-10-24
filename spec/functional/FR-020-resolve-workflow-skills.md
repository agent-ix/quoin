---
id: FR-020
title: "Expose workflow launchers and resolve their skills"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-005"
    type: "implements"
---

# FR-020: Own and materialize specification workflows

## Description

The CLI SHALL own the `review` and `to-plan` workflow definitions and instructions
as first-party Quoin source. The CLI SHALL embed them into each native executable.
When a workflow is launched, Quoin SHALL materialize an immutable snapshot below
the selected config root. Quoin SHALL supply its absolute path to ix-flow. The published binary SHALL
require no workflow npm package, private repository, or source checkout.

## Inputs

- The workflow name, selected config root, launch repository and targets.
- The running executable and the published ix-flow executable on PATH.

## Outputs

- A materialized owned workflow and a run initialized with repository context.
- An actionable refusal when the runner, targets or runtime directory is unavailable.

## Behavior

- Quoin SHALL recognize review and to-plan.
- Quoin SHALL retain matrix as a native computed view.
- Quoin SHALL reuse byte-identical snapshots.
- Quoin SHALL preserve existing snapshots
  when materializing a changed release or repairing incomplete assets.
- Quoin SHALL require ix-flow >=0.3.0 before creating a run.
- Quoin SHALL record canonical repository/config roots and contained targets in
  one quoin_context item before reporting a successful initialized launch.
- Quoin SHALL ignore the obsolete IX_SPEC_WORKFLOWS_ROOT override.
- Quoin SHALL NOT resolve definitions from node_modules or a private checkout.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-020-AC-1 | Review and to-plan launch from a relocated native executable with an empty home and no source checkout; matrix resolves no workflow. | Test |
| FR-020-AC-2 | An unknown workflow is refused before spawning the runner. | Test |
| FR-020-AC-3 | A missing or incompatible ix-flow reports the supported installation command without creating a run. | Test |
| FR-020-AC-4 | Immutable snapshot reuse and repair preserve existing runtime snapshots. | Test |
| FR-020-AC-5 | Run context resolves relative documents under the launch repository, including paths with spaces. | Test |

## Dependencies

- Upstream: [StR-004](../stakeholder/StR-004-governed-workflows.md).
- Downstream: [FR-021](./FR-021-launch-ix-flow-runs.md).
