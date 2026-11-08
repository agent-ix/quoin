---
id: FR-011
title: "List the catalog and show a single type"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-011: List the catalog and show a single type

## Description

The `catalog list` command SHALL print one `name@version root` line per module,
or the full catalog as JSON under `--json`; and the `catalog show <type>` command
SHALL print the resolved entry's kind, name, module, and root, or the entry as
JSON under `--json`, raising an error when the type is unknown or omitted.

## Inputs

- The `catalog list` or `catalog show <type>` invocation, with an optional
  `--json` flag.

## Outputs

- Module lines or entry details on standard output (text or JSON).

## Behavior

- `catalog list` SHALL print one line per module as `name@version root`, rendering
  a missing version as `unknown`.
- `catalog list --json` SHALL print the whole catalog as JSON.
- An omitted subcommand SHALL behave as `list`.
- `catalog show <type>` SHALL print the entry's kind, name, module, and root.
- `catalog show <type> --json` SHALL print the entry as JSON.
- `catalog show` SHALL raise an error when the type is omitted or unknown.

## Acceptance Criteria

| ID          | Criteria                                                                                           | Verification       |
| ----------- | -------------------------------------------------------------------------------------------------- | ------------------ |
| FR-011-AC-1 | `catalog list` prints one `name@version root` line per module; a missing version renders `unknown` | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-011-AC-2 | An omitted subcommand behaves as `list`, and `--json` prints the whole catalog                     | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-011-AC-3 | `catalog show <type>` prints the entry as text or JSON                                             | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-011-AC-4 | `catalog show` raises an error for an omitted or unknown type                                      | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md). Reads entries from
  [FR-009](./FR-009-read-module-manifest.md).
- **Downstream**: none.
