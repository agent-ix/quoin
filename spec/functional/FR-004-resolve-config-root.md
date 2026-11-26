---
id: FR-004
title: "Resolve the config root and configure the runtime context"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# FR-004: Resolve the config root and configure the runtime context

## Description

The CLI SHALL resolve a config root from the `--config-root` flag or the
`IX_CONFIG_ROOT` environment variable, defaulting to `~/.ix` (via the `IX_HOME`
environment variable), export the resolved root to
`IX_HOME`, and configure the shared `ix-cli-core` runtime context with the `ix`
namespace and a project config root at `<cwd>/.ix`, treating project config as
enabled unless `--no-project-config` is given.

## Inputs

- The optional `--config-root` flag, the `IX_CONFIG_ROOT` and `IX_HOME`
  environment variables, and the optional `--no-project-config` flag.

## Outputs

- The exported `IX_HOME` value and a configured runtime context.

## Behavior

- The CLI SHALL use `--config-root` when present; in its absence, the selected
  root is the first non-empty value from `IX_CONFIG_ROOT`, `IX_HOME`, or `~/.ix`,
  in that order.
- The CLI SHALL export the resolved root to `IX_HOME` for downstream resolution.
- The CLI SHALL configure the runtime context with namespace `ix` and project
  config root `<cwd>/.ix`.
- The CLI SHALL enable project config by default and disable it when
  `--no-project-config` is given.

## Acceptance Criteria

| ID          | Criteria                                                                   | Verification                        |
| ----------- | -------------------------------------------------------------------------- | ----------------------------------- |
| FR-004-AC-1 | `--config-root` selects the config root for the run                        | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-004-AC-2 | With no `--config-root`, the root falls back to `IX_CONFIG_ROOT`, then `IX_HOME`, then to `~/.ix` | Test |
| FR-004-AC-3 | `--no-project-config` disables the project config root                     | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-004-AC-4 | `IX_CONFIG_ROOT` selects the module home for `quoin module list` and `quoin module ensure-defaults`, ahead of `IX_HOME` | Test |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: the config root locates the shared module store read by
  [FR-007](./FR-007-assemble-module-roots.md) and
  [FR-017](./FR-017-reconcile-default-modules.md). The configurable surface is
  described by [FR-023](./FR-023-runtime-configuration.md).
