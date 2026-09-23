---
id: FR-112
title: "Distribute the native quoin binary through npm from GitHub Release assets"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-006"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-102"
    type: "extends"
---

# FR-112: Distribute the native quoin binary through npm from GitHub Release assets

## Description

In addition to the GitHub Release assets FR-102 already publishes, quoin
SHALL publish the native `quoin` binary to public npmjs as a launcher
package, `@agent-ix/quoin`, plus one platform package per supported
target — `@agent-ix/quoin-linux-x64`, `@agent-ix/quoin-linux-arm64`,
`@agent-ix/quoin-darwin-arm64` and `@agent-ix/quoin-win32-x64` — the way
`quire-cli` already publishes itself. The npm packages SHALL be built only
from the GitHub Release assets of an existing tag, after each asset's
SHA-256 is checked against that release's `quoin-update-manifest.json`;
quoin SHALL NOT rebuild the binary to publish it to npm. Packaging is
performed by the shared `agent-ix/nodejs-actions/publish-native-npm` action,
called from `.github/workflows/release.yml`; quoin's own workspace carries
no packaging logic and no JavaScript source of its own. Under an npm
install, `quoin update` SHALL decline to modify the installation and SHALL
name the npm command that manages it instead, since npm — not the native
updater — owns the files it placed under `node_modules`.

## Rationale

Owner ruling, 2026-09-23: quoin's npm distribution was withdrawn during the
Rust burn-down (StR-009) as a deliberate, dated exemption while the port
ran (ADR-0003 "Registries"), and the owner has now reinstated it in the
shape `quire-cli` already ships — a thin launcher plus per-platform
optional dependencies, repackaged from artifacts that are already built and
already smoked rather than rebuilt for a second channel. Rebuilding for npm
would let the two distribution channels drift from what GitHub Release
delivery actually shipped and smoked; reading the same manifest FR-102's
release already carries removes that risk instead of adding a second build
and a second version to track.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-112-AC-1 | `npm install -g @agent-ix/quoin@<v>` installs the native `<v>` binary for `linux-x64`, `linux-arm64`, `darwin-arm64` and `win32-x64`, and the installed `quoin --version` reports `<v>` on each. | Test (TC-1885) |
| FR-112-AC-2 | The npm packages published for tag `v<v>` are built from that tag's GitHub Release assets after each artifact's SHA-256 is verified against `quoin-update-manifest.json`; a checksum mismatch fails the workflow before anything is published, and no artifact is rebuilt from source for npm. | Test (TC-1886) |
| FR-112-AC-3 | Under an npm install, `quoin update` given as the first argument makes no change to the installation, exits non-zero, and its stderr names the npm command that manages it (`npm install -g @agent-ix/quoin@latest`) and states that quoin was installed with npm. | Test (TC-1887) |
| FR-112-AC-4 | Every Linux platform package (`@agent-ix/quoin-linux-x64`, `@agent-ix/quoin-linux-arm64`) declares `libc: ["glibc"]` in `package.json`, matching the glibc floor the archived binary was built against. | Test (TC-1888) |

## Constraints

- **FR-112-CON-1**: No packaging, archive-extraction or npm-publish logic
  lives in this repository; `release.yml` verifies and extracts the release
  assets and calls `agent-ix/nodejs-actions/publish-native-npm@main`, which
  owns the package layout and the `npm publish` calls.
- **FR-112-CON-2**: The published npm version always equals the dispatched
  tag with its leading `v` removed; there is no independent npm version
  counter.
- **FR-112-CON-3**: Publishing to npm never publishes a Rust crate to
  crates.io; FR-102-AC-7's prohibition is unaffected.
- **FR-112-CON-4**: The npm launcher intercepts `update` only when it is the
  first argument; an invocation such as `quoin --no-project-config update`
  is not recognised by the launcher and reaches the native updater, which
  does not itself detect an npm install and would overwrite the binary
  `node_modules` placed on disk. Closing that gap requires npm-install
  detection inside the native updater itself, tracked as PLAT-1013.

## Dependencies

- **Upstream**: [StR-006](../stakeholder/StR-006-current-via-self-update.md),
  the operator need this satisfies for an npm install; and
  [FR-102](./FR-102-command-surface-and-oclif-retirement.md), which defines
  the GitHub Release asset contract this requirement repackages.
- **Downstream**: none. `agent-ix/nodejs-actions`'s `publish-native-npm`
  action owns the packaging and publish implementation, and is verified by
  its own test suite in that repository.
