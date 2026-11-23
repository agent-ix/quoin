---
id: FR-212
title: "Warn when an installed default module is not at the CLI's pin"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-017"
    type: "references"
---

# FR-212: Warn when an installed default module is not at the CLI's pin

## Description

Lazy reconciliation ([FR-017](./FR-017-reconcile-default-modules.md)) skips a
module that is already installed, so a module installed by an older CLI keeps
its old release after the CLI is upgraded and its content can silently lack
fields the current CLI's pin has. `quoin module doctor` and
`quoin catalog validate` SHALL warn for each default module that is installed
at a release other than, and not newer than, the release the running CLI pins.

## Inputs

- The committed default-module manifest and the install registry.

## Outputs

- One warning diagnostic per such module, naming the module, its installed ref
  and its pinned ref, and stating the two commands that install the pinned
  release: `quoin module remove <name>`, then `quoin module ensure-defaults`.
- Warnings never change an exit status.

## Behavior

- A pin is comparable when its ref parses as a semantic version, with an
  optional leading `v`. A module whose pin is not comparable is not reported.
- When the installed ref also parses as a semantic version and is lower than
  the pin, the warning says the installed release is older than the pin.
- When the installed ref is absent or does not parse as a semantic version (a
  branch, for example) and differs from the pin, the warning says the module is
  not at the pinned release.
- A module that is not installed, or is installed at the pinned or a newer
  release, is not reported.
- `quoin module doctor` SHALL print the warnings.
- `quoin module doctor` SHALL print one summary line stating how many installed
  modules were warned about.
- `quoin module doctor` SHALL exit zero when the only findings are warnings.
- `quoin catalog validate` SHALL print the warnings and keep the exit status it
  has without them.
- When the installed modules cannot be read to make the check, `quoin catalog
  validate` SHALL report that as a warning and keep the exit status it has
  without the check.

## Acceptance Criteria

| ID          | Criteria                                                                                                                                                     | Verification |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------ |
| FR-212-AC-1 | A module installed at a semantic-version ref lower than its pin yields a warning naming the module, installed ref and pinned ref, and the remove-then-ensure-defaults commands | Test         |
| FR-212-AC-2 | A module installed at its pinned or a newer release yields no warning                                                                                        | Test         |
| FR-212-AC-3 | A module installed with a non-version ref or no ref, differing from a version pin, yields a warning naming the module, installed ref and pinned ref, saying it is not at the pinned release | Test         |
| FR-212-AC-4 | A module that is not installed, or whose pin is not a semantic version, yields no warning                                                                    | Test         |
| FR-212-AC-5 | `quoin module doctor` prints the warnings and a one-line summary count, and exits zero                                                                       | Test         |
| FR-212-AC-6 | `quoin catalog validate` prints the warnings and exits zero when the catalog is valid                                                                        | Test         |
| FR-212-AC-7 | When the installed modules cannot be read, the check yields a warning diagnostic in place of the lag warnings and does not change the exit status of `quoin catalog validate` | Test         |

## Dependencies

- **Upstream**: [FR-017](./FR-017-reconcile-default-modules.md).
