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

# FR-020: Own specification workflow skills

## Description

Quoin SHALL own review and to-plan definitions and instructions as first-party
source beside their agent plugin skills. Each skill SHALL declare its relative
workflows directory in metadata. ix-flow SHALL load these ordinary files from
an explicit skill directory. The CLI SHALL supply the native invariant provider.
The workflow SHALL require no archived provider package or private checkout.

## Inputs

- The workflow name, explicit skill directory, config root, repository and targets.
- Installed Quoin and ix-flow executables on PATH.

## Outputs

- An ix-flow run referencing the supplied skill directory and repository context.
- An actionable refusal when a skill path or supported runner is unavailable.

## Behavior

- Quoin SHALL recognize review and to-plan and retain matrix as a computed view.
- The plugin SHALL ship SKILL.md and workflows/<name>/def.yaml for each workflow.
- The agent SHALL launch the installed skill through ix-flow run --path.
- The convenience CLI SHALL accept --path and initialize one quoin_context item.
- The CLI SHALL require ix-flow >=0.3.1 before creating a run.
- Quoin SHALL record canonical repository/config roots and contained targets.
- Quoin SHALL preserve existing runs and their recorded skill paths.
- Quoin SHALL ignore the obsolete IX_SPEC_WORKFLOWS_ROOT override.
- The CLI SHALL NOT embed or extract workflow definitions.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-020-AC-1 | Copied plugin skills launch directly through ix-flow with a relocated native CLI, empty home and no source checkout; matrix resolves no workflow. | Test |
| FR-020-AC-2 | An unknown convenience-launcher workflow is refused before spawning the runner. | Test |
| FR-020-AC-3 | A missing or incompatible ix-flow reports the supported installation command without creating a run. | Test |
| FR-020-AC-4 | A missing explicit skill directory reports how to provide the installed plugin skill, without extracting workflow assets. | Test |
| FR-020-AC-5 | Run context resolves relative documents under the subject repository, including paths with spaces. | Test |

## Dependencies

- Upstream: [StR-004](../stakeholder/StR-004-governed-workflows.md).
- Downstream: [FR-021](./FR-021-launch-ix-flow-runs.md).
