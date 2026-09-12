---
id: FR-102
title: "Preserve the command surface and retire the oclif shell last"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "requires"
---

# FR-102: Preserve the command surface and retire the oclif shell last

## Description

Quoin SHALL hold its command surface unchanged until every logic capability
behind it is Rust-native, then replace the oclif command shell with a Rust CLI
that preserves the published command grammar, and SHALL resolve the published
oclif extension contract by a recorded dated disposition before that replacement
lands.

## Inputs

- The 59 declared commands and their flags, arguments and help text.
- The `plugins` array and the `command_not_found` hook declared in
  `package.json`.
- The command-surface snapshots in `tests/command-entries.test.ts` and
  `tests/cli-usage.test.ts`.
- The reviewed Rust crates behind each command.

## Outputs

- A `quoin` binary produced from `quoin-cli`.
- A command-surface snapshot report per candidate revision.
- A dated owner disposition for the oclif extension contract.

## Behavior

- Quoin SHALL keep the command-surface snapshot unchanged through every stage
  before the shell replacement, and SHALL treat any snapshot difference as a
  regression rather than as an accepted consequence of a port.
- Quoin SHALL replace `@oclif/core` only after every logic capability reachable
  from a command is Rust-native and demonstrated at one candidate revision under
  [FR-101](./FR-101-retire-replaced-executable-paths.md).
- `quoin-cli` SHALL parse the same command names, subcommands, arguments and
  flags as the retained shell, and SHALL preserve each command's exit-status
  meaning.
- Quoin SHALL record a dated owner disposition — port, retain behind an escape
  hatch, or drop — for the `plugins` array, for the `command_not_found` hook and
  for the `@agent-ix/filament-plan-sync` plugin, and that disposition SHALL exist
  before the shell replacement lands.
- Where a disposition is to retain, `quoin-cli` SHALL continue to resolve
  declared plugin commands and SHALL continue to invoke the
  `command_not_found` behaviour for an unrecognised command.
- Where a disposition is to drop, Quoin SHALL announce the removal in the release
  notes for the version that removes it and SHALL name the affected extension.
- If an unrecognised command is supplied and no extension resolves it, then
  `quoin-cli` SHALL refuse it with a non-zero status and SHALL name the
  unrecognised command.
- At the shell replacement, `quoin-core` SHALL become `quoin`, and Quoin SHALL
  delete `src/quire/exec.ts` and `src/core/exec.ts` together with the
  TypeScript entry point.
- Quoin SHALL publish the TypeScript package, while it exists, to `npm.ix` and
  SHALL NOT publish it to public npmjs, and SHALL NOT publish `quoin-cli` to
  crates.io.

## Error Conditions

A command-surface snapshot difference before the deliberate replacement, an
unrecognised command that resolves to nothing, a declared plugin that cannot be
resolved, and a shell replacement attempted with an undisposed extension contract
each fail and block the change.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-102-CON-1 | `@oclif/core` SHALL NOT be removed before every capability behind the command surface is Rust-native. | Lifecycle | Test |
| FR-102-CON-2 | The published extension contract SHALL NOT be changed without a dated owner disposition. | Compatibility | Inspection |
| FR-102-CON-3 | The command surface SHALL NOT change as a side effect of a port stage. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-102-AC-1 | `make test` reports the command-entries and CLI-usage snapshots unchanged at every candidate revision before the shell replacement. | Test (TC-1649) |
| FR-102-AC-2 | `quoin-cli` accepts every command name, argument and flag the retained shell accepted, and the two produce the same exit status for each. | Property (TC-1650) |
| FR-102-AC-3 | A dated owner disposition exists for the `plugins` array, the `command_not_found` hook and `@agent-ix/filament-plan-sync`, and the shell-replacement gate fails while any is absent. | Test (TC-1651) |
| FR-102-AC-4 | Where the disposition is retain, a declared plugin command resolves through `quoin-cli` and an unrecognised command invokes the `command_not_found` behaviour. | Test (TC-1652) |
| FR-102-AC-5 | An unrecognised command that no extension resolves exits non-zero and names the command. | Test (TC-1653) |
| FR-102-AC-6 | After the replacement, the repository contains no `@oclif/core` dependency, no `src/quire/exec.ts` and no `src/core/exec.ts`, and the binary is named `quoin`. | Test (TC-1654) |
| FR-102-AC-7 | The package manifest declares `npm.ix` as its publication registry and no first-party crate declares a crates.io publication target. | Inspection |

## Dependencies

- **Upstream**: [FR-099](./FR-099-rust-catalog-and-validation-capability.md), [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) and [FR-101](./FR-101-retire-replaced-executable-paths.md); [FR-001](./FR-001-parse-command-line.md), [FR-003](./FR-003-print-usage-and-help.md), [FR-005](./FR-005-reject-unknown-commands.md) and [FR-026](./FR-026-dispatch-through-oclif-runner.md), whose behaviour it preserves or replaces.
- **Downstream**: none.
