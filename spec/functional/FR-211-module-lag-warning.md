---
id: FR-211
title: "Warn when an installed default module lags the CLI's pin"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-017"
    type: "references"
---

# FR-211: Warn when an installed default module lags the CLI's pin

## Description

Lazy reconciliation ([FR-017](./FR-017-reconcile-default-modules.md)) skips a
module that is already installed, so a module installed by an older CLI keeps
its old release after the CLI is upgraded and its content can silently lack
fields the current CLI's pin has. `quoin module doctor` and
`quoin catalog validate` SHALL warn for each default module whose installed
release is older than the release the running CLI pins.

## Inputs

- The committed default-module manifest and the install registry.

## Outputs

- One warning diagnostic per lagging module, naming the module, its installed
  version and its pinned version. Warnings do not change the exit status.

## Behavior

- A default module lags when its installed `ref` and its pinned `ref` both parse
  as semantic versions (with an optional leading `v`) and the installed one is
  lower.
- A module that is not installed, is installed at the pinned or a newer
  release, or has a ref that is not a semantic version is not reported.
- `quoin module doctor` SHALL report the registry state and exit zero when only
  lag warnings are present.

## Acceptance Criteria

| ID          | Criteria                                                                                                        | Verification |
| ----------- | --------------------------------------------------------------------------------------------------------------- | ------------ |
| FR-211-AC-1 | An installed module older than its pin yields a warning naming the module, installed version and pinned version | Test         |
| FR-211-AC-2 | An installed module at or above its pin yields no warning                                                       | Test         |
| FR-211-AC-3 | `quoin module doctor` and `quoin catalog validate` emit the lag warnings without a failing exit status           | Test         |

## Dependencies

- **Upstream**: [FR-017](./FR-017-reconcile-default-modules.md).
