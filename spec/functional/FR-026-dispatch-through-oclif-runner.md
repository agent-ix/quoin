---
id: FR-026
title: "Dispatch commands through the oclif runner"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-001"
    type: "specifies"
  - target: "ix://agent-ix/quoin/FR-003"
    type: "specifies"
  - target: "ix://agent-ix/quoin/FR-005"
    type: "specifies"
---

# FR-026: Dispatch commands through the oclif runner

## Description

`quoin` SHALL dispatch every command through the oclif runner provided by
`@agent-ix/ix-cli-core` ([FR-025](ix://agent-ix/ix-cli-core/FR-025)), rather
than through a hand-rolled argument parser of its own.

Each command SHALL be a `BaseCommand` subclass under the package's
`oclif.commands` directory, discovered by oclif rather than registered by
quoin. Argument parsing, help rendering, and unknown-command rejection —
specified as observable behaviour by
[FR-001](./FR-001-parse-command-line.md),
[FR-003](./FR-003-print-usage-and-help.md), and
[FR-005](./FR-005-reject-unknown-commands.md) — SHALL be provided by the runner,
so quoin owns the command surface but not the mechanism that serves it.

## Inputs

- The argument vector, and the package's `oclif` configuration (`commands`
  directory, `plugins` list, `topicSeparator`).

## Outputs

- The dispatched command's output, or an error from the runner.

## Behavior

- The runner SHALL discover every command quoin ships, with no bespoke registry
  or manifest loader in quoin.
- Subcommands SHALL be addressed space-separated (`quoin catalog list`), per the
  configured `topicSeparator`.
- `quoin` SHALL NOT retain a hand-rolled argument dispatcher.
- The bare version forms (`--version`, `-v`, `version`) SHALL be answered before
  the runner is invoked, so the output stays the bare version string rather than
  oclif's decorated form ([FR-002](./FR-002-print-package-version.md)).
- Every other argument vector SHALL be handed to the runner, and an error it
  raises SHALL propagate to the caller rather than being swallowed.
- A command declared by a package listed in `oclif.plugins` SHALL be dispatched
  as though quoin declared it.

## Acceptance Criteria

| ID          | Criteria                                                                                     | Verification         |
| ----------- | ---------------------------------------------------------------------------------------------- | -------------------- |
| FR-026-AC-1 | The runner discovers every command quoin ships                                                 | Test (cli.test.ts)   |
| FR-026-AC-2 | Subcommands are addressed space-separated                                                      | Test (cli.test.ts)   |
| FR-026-AC-3 | No hand-rolled argument dispatcher remains in `src/cli.ts`                                     | Test (cli.test.ts)   |
| FR-026-AC-4 | An unknown command, and an unknown subcommand, are each rejected by the runner                 | Test (cli.test.ts)   |
| FR-026-AC-5 | The bare version forms are answered before dispatch; every other argv is handed to the runner  | Test (cli.test.ts)   |
| FR-026-AC-6 | An error raised by the runner propagates to the caller                                         | Test (cli.test.ts)   |
| FR-026-AC-7 | A package listed in `oclif.plugins` is discovered as a core plugin and its command is dispatched, with no runtime install step | Test (it-005-sync-discovery.test.ts) |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone
  CLI. Consumes the runner specified by
  [ix-cli-core FR-025](ix://agent-ix/ix-cli-core/FR-025).
- **Downstream**: supplies the mechanism behind
  [FR-001](./FR-001-parse-command-line.md),
  [FR-003](./FR-003-print-usage-and-help.md), and
  [FR-005](./FR-005-reject-unknown-commands.md).
