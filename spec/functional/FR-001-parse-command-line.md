---
id: FR-001
title: "Parse the command line into command, flags, and positionals"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# FR-001: Parse the command line into command, flags, and positionals

## Description

The CLI SHALL parse its argument vector into a leading command, an optional
subcommand for the `catalog` and `plugin` commands, long flags in both
`--flag=value` and `--flag value` forms, single-dash boolean flags, and bare
positionals, accumulating repeated `--types` and `--target` flags into ordered
lists.

This behaviour is provided by the oclif runner rather than a parser quoin
maintains; see [FR-026](./FR-026-dispatch-through-oclif-runner.md). The
requirement below states what an invocation observes, not how it is parsed.

## Inputs

- The process argument vector (an ordered list of strings).

## Outputs

- A structured parse result carrying the command, optional subcommand, the
  ordered positionals, and a map of flag values (string, boolean, or ordered
  list).

## Behavior

- The first bareword SHALL become the command. For `catalog` and `plugin`, the
  next bareword SHALL become the subcommand; any further barewords SHALL become
  positionals.
- A `--flag=value` token SHALL bind the value after the `=`.
- A `--flag value` token SHALL bind the following token when it is not itself a
  flag; otherwise the flag SHALL be recorded as boolean `true`.
- A single-dash token such as `-h` SHALL be recorded as a boolean flag.
- Repeated `--types` and `--target` flags SHALL accumulate their values, in
  order, into a list.

## Acceptance Criteria

| ID          | Criteria                                                             | Verification                        |
| ----------- | -------------------------------------------------------------------- | ----------------------------------- |
| FR-001-AC-1 | Both `--flag=value` and `--flag value` bind the value                | Test (cli.test.ts)                  |
| FR-001-AC-2 | A bareword after `write` is parsed as a positional, not a subcommand | Test (cli.test.ts)                  |
| FR-001-AC-3 | A long flag with no following value is recorded as boolean `true`    | Test (cli.test.ts)                  |
| FR-001-AC-4 | Repeated `--types`/`--target` accumulate into an ordered list        | Test (write.test.ts, index.test.ts) |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: every command handler consumes this parse result, including
  [FR-013](./FR-013-resolve-requested-types.md) and
  [FR-021](./FR-021-launch-ix-flow-runs.md), which read the accumulated
  `--types`/`--target` lists.
