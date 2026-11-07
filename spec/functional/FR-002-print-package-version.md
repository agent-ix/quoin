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
`--version` flag, or the `-v` flag. The version SHALL be the value baked at build
time from `git describe` — a bare tag for a clean release (e.g. `0.5.2`), a
commits-ahead suffix otherwise (e.g. `0.5.1-3-gabc123`) — and SHALL fall
back to the package's `package.json` `version` field when no build-time value is
present (dev, test, or no-git builds), raising an error when that fallback version
is missing or not a string.

## Inputs

- The `version` command, or the `--version` / `-v` flag.

## Outputs

- The version string on standard output, or a raised error when the version is
  unreadable.

## Behavior

- The CLI SHALL prefer the build-time baked version (`git describe --tags`, leading
  `v` stripped) when it is a non-empty string. It SHALL NOT use `--dirty`, since the
  release pipeline stamps `package.json` before building and would otherwise mislabel
  a clean release as `-dirty`.
- The CLI SHALL otherwise resolve its `package.json` relative to the installed
  package and read the `version` field.
- The CLI SHALL print the resolved version and return before any other dispatch.
- The CLI SHALL raise an explicit error when, on the fallback path, the `version`
  field is absent or is not a string.

## Acceptance Criteria

| ID          | Criteria                                                                                              | Verification                        |
| ----------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-002-AC-1 | `version`, `--version`, and `-v` each print the package version                                       | Test (`quoin-cli` `src/main.rs` :: `tc_373_version_requests_keep_the_bare_native_version_surface`) |
| FR-002-AC-2 | The reported value is a non-empty string                                                              | Test (`quoin-cli` `src/main.rs` :: `tc_373_version_requests_keep_the_bare_native_version_surface`) |
| FR-002-AC-3 | A missing or non-string version field raises an error                                                 | No native test cited (PLAT-1275)  |
| FR-002-AC-4 | A non-empty baked version is reported verbatim; an empty one falls back to the `package.json` version | No native test cited (PLAT-1275)              |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: [FR-022](./FR-022-self-update.md) compares this running version
  against the latest published version.
