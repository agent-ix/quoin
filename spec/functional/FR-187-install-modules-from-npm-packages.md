---
id: FR-187
title: "Install modules from npm packages"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-003"
    type: "implements"
---

# FR-187: Install modules from npm packages

## Description

When a user installs a package source, the Quoin module installer SHALL resolve
the selected npm package release and install its module through the same module
registry and validation flow as other hosted sources.

## Inputs

- A `package:<name>` or `package:<name>@<version>` install source.

## Outputs

- An installed module record and materialized module on success.
- A named install error on metadata, download, integrity, archive, or module
  validation failure.

## Behavior

- The installer SHALL resolve package metadata and tarballs from the public npm
  registry over HTTPS. It SHALL reject a configured registry that is not the
  public registry.
- When the source names an exact version, the installer SHALL resolve that
  version. When no version is named, the installer SHALL resolve the `latest`
  dist-tag from the registry metadata.
- The installer SHALL obtain the selected release's tarball URL and declared
  integrity value from registry metadata, verify the downloaded tarball against
  that integrity value, and install the module rooted in the package archive.
- The installer SHALL reject archive entries that escape the extraction root
  and SHALL NOT execute package scripts.
- The installer SHALL reject a redirect that changes the HTTPS scheme or
  leaves the public registry origin.
- If metadata resolution, download, integrity verification, archive extraction,
  or module validation fails, the install SHALL leave the module registry and
  any previously installed module unchanged.
- When installation succeeds, the installer SHALL record the package source
  and selected version in the module registry.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-187-CON-1 | The installer SHALL send requests only over HTTPS to `registry.npmjs.org` | Security | Test |
| FR-187-CON-2 | The installer SHALL reject redirects that change the HTTPS scheme or leave the public registry origin | Security | Test |
| FR-187-CON-3 | The installer SHALL limit package metadata to 16 MiB, a compressed tarball to 64 MiB, extracted content to 512 MiB, and archive entries to 10,000 | Security | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-187-AC-1 | Given a package source with an exact version, the installer resolves that release, installs its module, and records the package source and selected version | Test |
| FR-187-AC-2 | Given a package source without a version, the installer resolves the registry's `latest` dist-tag and records its selected version | Test |
| FR-187-AC-3 | Given a tarball whose bytes do not match the registry integrity value, the installer rejects it without changing the registry or an existing installation | Test |
| FR-187-AC-4 | Given an archive containing a path that escapes the extraction root, the installer rejects it without writing outside the extraction root | Test |
| FR-187-AC-5 | A package containing an install script does not cause the installer to execute that script | Test |
| FR-187-AC-6 | Given a package source that resolves to a module failing existing module validation, the installer rejects it and preserves the prior registry record and installed module | Test |

## Dependencies

- **Upstream**: source parsing from [FR-018](./FR-018-map-plugin-sources.md),
  install and registry behavior from
  [FR-019](./FR-019-manage-plugin-registry.md), and package-module use from
  [US-003](../usecase/US-003-install-community-module.md).
- **Downstream**: installed package modules join the active catalog through
  [FR-007](./FR-007-assemble-module-roots.md).
