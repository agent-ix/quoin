---
id: FR-021
title: "Launch workflow runs through ix-flow"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-005"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-020"
    type: "references"
---

# FR-021: Launch workflow runs through ix-flow

## Description

On launch, the CLI SHALL spawn
`ix-flow run <flow> --path <skill> --config-root <home> --state-dir <home>/flows`,
appending the optional `--id`, `--json`, and `--target` arguments, SHALL propagate
the `ix-flow` exit code (defaulting to 1 when the child is terminated by a
signal), SHALL surface a spawn error when `ix-flow` cannot be started, and SHALL
hand all subsequent lifecycle control to `ix-flow`.

## Inputs

- The resolved skill path, the config root, and the optional `--id`/`--json`/
  `--target` flags.

## Outputs

- A spawned `ix-flow` run and a propagated exit status.

## Behavior

- The CLI SHALL spawn `ix-flow run` with the flow, skill path, config root, and
  a `<home>/flows` state directory.
- The CLI SHALL append `--id`, `--json`, and each `--target` when provided.
- The CLI SHALL set its exit status to the `ix-flow` exit code, defaulting to 1
  when the child reports no code (signal termination).
- The CLI SHALL surface an error when `ix-flow` cannot be spawned.

## Acceptance Criteria

| ID          | Criteria                                                                                                         | Verification                      |
| ----------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| FR-021-AC-1 | A run spawns `ix-flow run` with the flow, skill, config root, state dir, and the `--id`/`--json`/`--target` args | Test (flows.test.ts, cli.test.ts) |
| FR-021-AC-2 | A non-zero `ix-flow` exit is propagated as the process exit status                                               | Test (flows.test.ts, cli.test.ts) |
| FR-021-AC-3 | A signal-terminated child yields exit status 1                                                                   | Test (flows.test.ts)              |
| FR-021-AC-4 | A failure to spawn `ix-flow` surfaces an error                                                                   | Test (flows.test.ts)              |

## Dependencies

- **Upstream**: [StR-004](../stakeholder/StR-004-governed-workflows.md);
  [US-005](../usecase/US-005-start-gated-spec-workflow.md); the skill from
  [FR-020](./FR-020-resolve-workflow-skills.md).
- **Downstream**: external-process invocation is constrained by
  [NFR-007](../non-functional/NFR-007-external-tool-invocation.md). The launch
  boundary is exercised at the agent-eval layer (`spec/evals.md`, TC-EV-005/TC-EV-013);
  post-launch lifecycle is owned by `ix-flow`.
