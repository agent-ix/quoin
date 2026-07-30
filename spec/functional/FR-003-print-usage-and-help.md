---
id: FR-003
title: "Print usage and command-scoped help"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-003: Print usage and command-scoped help

## Description

The CLI SHALL print the root usage text when invoked with no command, and SHALL
print command-scoped help in response to `--help` or `-h` for the `write`,
`catalog`, `plugin`, `update`, and workflow commands, falling back to the root
usage for an unrecognized command.

Help is rendered by the oclif runner from each command class's `static`
summary, description and examples; see
[FR-026](./FR-026-dispatch-through-oclif-runner.md). The hand-rolled usage
strings this requirement once described were removed with the dispatcher.

## Inputs

- An empty argument vector, or `--help` / `-h` with an optional command.

## Outputs

- The matching usage or help text on standard output.

## Behavior

- With no command, the CLI SHALL print the root usage describing the command
  surface, the default module set, and the global flags.
- With `--help`/`-h` and a recognized command, the CLI SHALL print that command's
  help text.
- With `--help`/`-h` and an unrecognized command, the CLI SHALL print the root
  usage.

## Acceptance Criteria

| ID          | Criteria                                                                              | Verification                        |
| ----------- | ------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-003-AC-1 | No command prints the root usage                                                      | Test (cli.test.ts)                  |
| FR-003-AC-2 | `--help`/`-h` prints help for `write`, `catalog`, `plugin`, and the workflow commands | Test (cli.test.ts, scripts.test.ts) |
| FR-003-AC-3 | `--help` on an unrecognized command falls back to the root usage                      | Test (cli.test.ts)                  |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI;
  [US-002](../usecase/US-002-discover-authoring-contract.md) discover the contract.
- **Downstream**: none.
