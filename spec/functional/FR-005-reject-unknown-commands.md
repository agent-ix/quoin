---
id: FR-005
title: "Reject unknown commands and subcommands"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/NFR-003"
    type: "traces_to"
---

# FR-005: Reject unknown commands and subcommands

## Description

The CLI SHALL reject an unknown command, and an unknown `catalog` or `plugin`
subcommand, by raising an error that includes the relevant usage text, and SHALL
exit with a non-zero status.

Rejection is performed by the oclif runner; see
[FR-026](./FR-026-dispatch-through-oclif-runner.md). The requirement below
states the observable outcome, not the mechanism.

## Inputs

- An unrecognized command, or an unrecognized `catalog`/`plugin` subcommand.

## Outputs

- A raised error carrying usage text, and a non-zero exit status.

## Behavior

- The CLI SHALL raise an error naming the unknown command and including the root
  usage.
- The CLI SHALL raise an error naming an unknown `catalog`/`plugin` subcommand.
- An uncaught error SHALL surface as a non-zero process exit.

## Acceptance Criteria

| ID          | Criteria                                               | Verification       |
| ----------- | ------------------------------------------------------ | ------------------ |
| FR-005-AC-1 | An unknown command raises an error that includes usage | Test (cli.test.ts) |
| FR-005-AC-2 | An unknown `catalog` subcommand raises an error        | Test (cli.test.ts) |
| FR-005-AC-3 | An unknown `plugin` subcommand raises an error         | Test (cli.test.ts) |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: this is one source of the actionable-failure guarantee in
  [NFR-003](../non-functional/NFR-003-actionable-errors.md).
