---
id: FR-102
title: "Preserve the command surface and retire the oclif shell last"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "requires"
---

# FR-102: Preserve the command surface and retire the oclif shell last

## Description

Quoin SHALL deliver the Rust `quoin` binary after every command capability is
native, preserve the retained command grammar through native fixtures, and
remove the Node/oclif shell without an escape hatch.

## Inputs

- The retained command-help and shell-case fixtures replayed by `quoin-cli`.
- The reviewed Rust crates behind every command.
- The dated disposition for the oclif plugin and hook.

## Outputs

- A `quoin` binary produced from `quoin-cli`.
- A native command-surface fixture replay report per candidate revision.
- GitHub Release archives and an update manifest for supported targets.

## Behavior

- Quoin SHALL treat a retained command-fixture difference as a regression.
- `quoin-cli` SHALL parse the retained command names, subcommands, arguments,
  flags, and exit-status meanings.
- The owner disposition SHALL withdraw the oclif plugin and `command_not_found`
  hook with the shell.
- An unrecognised command SHALL exit non-zero and name that command.
- The cutover SHALL delete the Node entrypoint, oclif configuration, command
  shell, IPC shim, generated TypeScript boundary surface, and their tests.
- Quoin SHALL publish the executable as a GitHub Release asset under the
  native release-manifest contract, and as npm packages repackaged from those
  same release assets (FR-112), and SHALL NOT publish `quoin-cli` to
  crates.io.

## Error Conditions

A fixture difference, an unrecognised command that is accepted, a missing
plugin disposition, or a remaining Node/oclif shell path blocks the cutover.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-102-CON-1 | The Node/oclif shell SHALL NOT be removed before every capability is Rust-native. | Lifecycle | Test |
| FR-102-CON-2 | The extension withdrawal SHALL have a dated owner disposition. | Compatibility | Inspection |
| FR-102-CON-3 | The command surface SHALL NOT change as a side effect of a port stage. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-102-AC-1 | Native fixture tests replay the retained command surface before shell replacement. | Test (TC-1649) |
| FR-102-AC-2 | `quoin-cli` accepts the retained command grammar and preserves exit status. | Property (TC-1650) |
| FR-102-AC-5 | An unrecognised command exits non-zero and names the command. | Test (TC-1653) |
| FR-102-AC-6 | The repository contains no Node/oclif shell or generated TypeScript boundary surface, and the binary is named `quoin`. | Test (TC-1654) |
| FR-102-AC-7 | No first-party crate declares a crates.io publication target. The executable is delivered as GitHub Release assets, and, per FR-112, as npm packages repackaged from those same assets; neither channel rebuilds it. | Test (TC-1703) |

## Dependencies

- **Upstream**: [FR-101](./FR-101-retire-replaced-executable-paths.md).
- **Downstream**: [FR-112](./FR-112-npm-distribution-from-release-assets.md),
  which repackages this requirement's GitHub Release assets into npm
  packages.
