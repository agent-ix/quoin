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
the requested npm release for that invocation and install its module through
the same registry and validation flow as other hosted sources.

## Inputs

- A `package:<name>` or `package:<name>@<version>` install source.

## Outputs

- An installed module record and materialized module on success.
- A named install error on metadata, download, archive, timeout, or module
  validation failure.

## Behavior

- The installer SHALL resolve package metadata and tarballs for the package
  source. Registry and redirect rules are defined by
  [FR-187-CON-1](#fr-187-con-1) and [FR-187-CON-2](#fr-187-con-2).
- When the source names an exact version, the installer SHALL resolve that
  version for the current invocation.
- When the source has no version, the installer SHALL resolve the registry's
  `latest` dist-tag for the current invocation.
- The requested version and any version selected from registry metadata SHALL
  remain transient to that install invocation.
- The registry record SHALL store the package name with its source version
  unset.
- The installer SHALL install the module rooted in the resolved package
  archive.
- The installer SHALL reject archive entries that escape the extraction root.
- The installer SHALL NOT execute package scripts.
- If metadata resolution, download, archive extraction, or module validation
  fails, the install SHALL leave the module registry and any previously
  installed module unchanged.
- When a registry metadata or tarball request reaches its deadline in
  [FR-187-CON-7](#fr-187-con-7), the installer SHALL return
  `QM027_NPM_TIMEOUT` and leave the module registry and any previously
  installed module unchanged.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-187-CON-1 | The installer SHALL make registry and tarball requests only to the HTTPS origin `https://registry.npmjs.org` | Security | unit-testing |
| FR-187-CON-2 | The installer SHALL reject redirects that change the HTTPS scheme or leave the public registry origin | Security | integration-testing |
| FR-187-CON-3 | The installer SHALL limit package metadata to 16 MiB | Security | integration-testing |
| FR-187-CON-4 | The installer SHALL limit a compressed package tarball to 64 MiB | Security | integration-testing |
| FR-187-CON-5 | The installer SHALL limit extracted package content to 512 MiB | Security | integration-testing |
| FR-187-CON-6 | The installer SHALL limit package archives to 10,000 entries | Security | integration-testing |
| FR-187-CON-7 | Each registry metadata or tarball request SHALL terminate within 300 seconds | Reliability | fault-injection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-187-AC-1 | Given `package:<name>@<version>`, a local HTTPS registry fixture records the exact requested version and serves that release, the installer installs its module, and the registry source retains the package name with its version unset | integration-testing |
| FR-187-AC-2 | Given `package:<name>` and a registry fixture whose `latest` dist-tag names a release, the installer requests and installs that release while the registry source retains the package name with its version unset | integration-testing |
| FR-187-AC-3 | End to end, given a scoped source `package:@scope/name@<version>`, a local HTTPS registry fixture records the scoped package and exact version request and serves that release, and the installer installs its module with the scoped package name intact and its version unset in the registry source | integration-testing |
| FR-187-AC-4 | Given a registry override whose scheme is HTTP or whose origin is not `https://registry.npmjs.org`, the installer refuses the source before making a request and leaves the registry and installed modules unchanged | unit-testing |
| FR-187-AC-5 | Given a registry fixture that redirects to HTTP or a different origin, the installer rejects the redirect and leaves the registry and installed modules unchanged | integration-testing |
| FR-187-AC-6 | Given a metadata response body of 16 MiB plus one byte, the installer reports `QM016_RESOURCE_BOUND_EXCEEDED` without changing the registry or installed modules | integration-testing |
| FR-187-AC-7 | Given a compressed tarball of 64 MiB plus one byte, the installer reports `QM016_RESOURCE_BOUND_EXCEEDED` without changing the registry or installed modules | integration-testing |
| FR-187-AC-8 | Given an archive whose extracted content is 512 MiB plus one byte, the installer reports `QM016_RESOURCE_BOUND_EXCEEDED` without writing outside the extraction root or changing installed state | integration-testing |
| FR-187-AC-9 | Given an archive with 10,001 entries, the installer reports `QM016_RESOURCE_BOUND_EXCEEDED` without changing the registry or installed modules | integration-testing |
| FR-187-AC-10 | Given an archive entry whose path escapes the extraction root, the installer rejects the archive without writing outside the extraction root | integration-testing |
| FR-187-AC-11 | Given a package containing an install script, the installer does not execute that script | integration-testing |
| FR-187-AC-12 | Given a package whose module fails existing module validation, the installer rejects it and preserves the prior registry record and installed module | integration-testing |
| FR-187-AC-13 | Given a registry metadata or tarball request that exceeds 300 seconds, the installer returns `QM027_NPM_TIMEOUT` and preserves the prior registry record and installed module | fault-injection |

## Dependencies

- **Upstream**: source parsing from [FR-018](./FR-018-map-plugin-sources.md),
  install and registry behavior from
  [FR-019](./FR-019-manage-plugin-registry.md), and package-module use from
  [US-003](../usecase/US-003-install-community-module.md).
- **Downstream**: installed package modules join the active catalog through
  [FR-007](./FR-007-assemble-module-roots.md).
