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
source beside their agent plugin skills. The design-gate workflow in
[FR-209](./FR-209-agent-design-gate.md) SHALL also be launchable through
Quoin's convenience CLI. Each skill SHALL declare its relative workflows
directory in metadata. ix-flow SHALL load these ordinary files from an explicit
skill directory. The CLI SHALL supply the native invariant provider. The
workflow SHALL require no archived provider package or private checkout.

## Inputs

- The workflow name, explicit skill directory, config root, repository and targets.
- Installed Quoin and ix-flow executables, with the ix-flow host API available
  from the installed package for design-gate.

## Outputs

- An ix-flow run referencing the supplied skill directory and repository context.
- For design-gate, one host-seeded, system-owned `quoin_context` item.
- An actionable refusal when a skill path or supported runner is unavailable.

## Behavior

- Quoin SHALL recognize review, to-plan, and design-gate launchers and retain
  matrix as a computed view.
- The plugin SHALL ship SKILL.md and workflows/<name>/def.yaml for each workflow.
- For review and to-plan, the agent SHALL launch the installed skill through
  `ix-flow run --path`. For design-gate, the agent SHALL use
  `quoin design-gate --path` so Quoin seeds its protected context.
- The convenience CLI SHALL accept --path and initialize one quoin_context item.
- When `quoin design-gate` launches a run, the host SHALL seed exactly one
  system-owned `quoin_context` before returning control to the proposer.
- The design-gate host SHALL set `quoin_context.id` to the engine run ID and
  derive `repo_root`, `config_root`, and canonical contained targets from the
  launch environment; the proposer SHALL NOT supply or replace these values.
- When a review or to-plan run is requested, the CLI SHALL require ix-flow
  >=0.3.1.
- When a design-gate run is requested, the CLI SHALL verify that the installed
  ix-flow package exposes the host seeding API before creating the run.
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
| FR-020-AC-6 | Given a compatible ix-flow host API, `quoin design-gate` invokes its system-owned item seeding API once for `quoin_context`; the item ID equals the engine run ID, and its repository root, config root, and targets equal the canonical launch paths (an empty target list when none are supplied). | Test |
| FR-020-AC-7 | Given an ix-flow package without the host seeding API, `quoin design-gate` returns an actionable error before creating a run. | Test |

The ix-flow engine owns refusal of proposer add, replacement, and update attempts against a
system-owned item and leaves that item unchanged. Quoin verifies that it declares
`quoin_context` as system-owned and seeds it only through ix-flow's host API; mutation refusal
belongs to ix-flow's own conformance tests.

## Dependencies

- Upstream: [StR-004](../stakeholder/StR-004-governed-workflows.md).
- Downstream: [FR-021](./FR-021-launch-ix-flow-runs.md).
- Related: [FR-209](./FR-209-agent-design-gate.md) defines the workflow launched here.
