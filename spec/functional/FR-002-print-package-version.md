---
id: FR-002
title: "Report the installed package version"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# FR-002: Report the installed package version

## Description

The CLI SHALL print its own version in response to the `version` command, the
`--version` flag, or the `-v` flag. A native build SHALL bake a usable version
from `git describe --tags`, removing its leading `v`; when Git metadata is
unavailable or has no usable tag, the build SHALL use Cargo's package version.

## Inputs

- The `version` command, or the `--version` / `-v` flag.

## Outputs

- The version string on standard output, or a raised error when the version is
  unreadable.

## Behavior

- The native build SHALL prefer a non-empty `git describe --tags` result, with a
  leading `v` removed and no dirty marker.
- The native build SHALL use Cargo's package version when Git metadata is
  unavailable or the Git result is empty.
- The CLI SHALL print the resolved version and return before any other dispatch.

## Acceptance Criteria

| ID          | Criteria                                                                                              | Verification                        |
| ----------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-002-AC-1 | `version`, `--version`, and `-v` each print the package version                                       | Test (`quoin-cli` `src/main.rs` :: `tc_373_version_requests_keep_the_bare_native_version_surface`) |
| FR-002-AC-2 | The reported value is a non-empty string                                                              | Test (`quoin-cli` `src/main.rs` :: `tc_373_version_requests_keep_the_bare_native_version_surface`) |
| FR-002-AC-3 | When Git metadata is unavailable or has no usable tag, the native build uses Cargo's package version. | Inspection |
| FR-002-AC-4 | A non-empty `git describe --tags` version is baked without a leading `v` or dirty marker, and the CLI prints that value verbatim. | Test |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: [FR-022](./FR-022-self-update.md) compares this running version
  against the latest published version.
