---
id: FR-022
title: "Upgrade quoin to the latest published release"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-006"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-002"
    type: "references"
---

# FR-022: Upgrade quoin to the latest published release

## Description

The CLI SHALL expose an `update` command that upgrades the native `quoin`
artifact from the latest stable GitHub Release manifest, supporting a `--check`
mode that reports availability without installing and a `--registry <url>`
option that selects an explicit manifest endpoint. With no `--registry`, the
command defaults to the public GitHub latest-release manifest.

## Inputs

- An `update` invocation with optional `--check` and `--registry <url>` flags.

## Outputs

- An in-place upgrade, or an availability report under `--check`.

## Behavior

- The CLI SHALL compare the running version (see
  [FR-002](./FR-002-print-package-version.md)) against a stable native release
  manifest and safely stage its target-specific artifact before replacement.
- The CLI SHALL report availability without installing when `--check` is given.
- The CLI SHALL pass a `--registry` manifest endpoint through, and SHALL
  otherwise default to GitHub's stable latest-release manifest.
- An npm-installed `quoin` (see [FR-112](./FR-112-npm-distribution-from-release-assets.md))
  is updated by npm rather than by this command's native artifact-replacement
  path: `update` given as the first argument under an npm install defers to
  npm instead of running the behavior above.

## Acceptance Criteria

| ID          | Criteria                                                                                                              | Verification          |
| ----------- | --------------------------------------------------------------------------------------------------------------------- | --------------------- |
| FR-022-AC-1 | `update` selects the running host's native artifact from a stable manifest and verifies it before replacement | Test (Rust delivery tests) |
| FR-022-AC-2 | `--check` reports availability without downloading an archive or mutating the installation | Test (Rust delivery tests) |
| FR-022-AC-3 | `--registry <url>` selects an explicit manifest endpoint; its absence uses the GitHub latest-release manifest | Test (Rust CLI tests) |

## Dependencies

- **Upstream**: [StR-006](../stakeholder/StR-006-current-via-self-update.md).
- **Downstream**: the release producer publishes the documented artifact and
  manifest convention in `docs/native-release.md`.
