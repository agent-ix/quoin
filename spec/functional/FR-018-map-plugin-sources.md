---
id: FR-018
title: "Map plugin source arguments to typed sources"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-003"
    type: "implements"
---

# FR-018: Map plugin source arguments to typed sources

## Description

The CLI SHALL map a plugin source argument to a typed `ts-plugin-kit` source:
`path:<dir>` to a path source; `github:<owner>/<repo>[@<ref>]` to a GitHub
source; `github:<owner>/<repo>//<subdir>[@<ref>]` to a git-subdir source;
`package:<pkg>[@<ver>]` to an npm source; and a bare argument to a path source.

## Inputs

- A single plugin source argument string.

## Outputs

- A typed source descriptor for `ts-plugin-kit`.

## Behavior

- A `path:` prefix SHALL produce a path source for the given directory.
- A `github:` prefix SHALL produce a GitHub source, carrying the `@<ref>` when
  present; a `//<subdir>` segment SHALL instead produce a git-subdir source with
  the repo, subdirectory, and optional ref.
- A `package:` prefix SHALL produce an npm source, preserving a leading scope `@`
  and splitting the version on the final `@`. The npm source mapping is in place
  for when `ts-plugin-kit` adds npm install support.
- A bare argument SHALL produce a path source.

## Acceptance Criteria

| ID          | Criteria                                                                                          | Verification                          |
| ----------- | ------------------------------------------------------------------------------------------------- | ------------------------------------- |
| FR-018-AC-1 | `path:<dir>` maps to a path source                                                                | Test (ts_oracle.rs, tc_446_source_arg_properties.rs, index.test.ts) |
| FR-018-AC-2 | `github:<owner>/<repo>[@<ref>]` maps to a GitHub source with optional ref                         | Test (ts_oracle.rs, tc_446_source_arg_properties.rs) |
| FR-018-AC-3 | `github:<owner>/<repo>//<subdir>[@<ref>]` maps to a git-subdir source                             | Test (ts_oracle.rs, tc_446_source_arg_properties.rs) |
| FR-018-AC-4 | `package:<pkg>[@<ver>]`, including a scoped package, maps to an npm source split on the final `@` | Test (ts_oracle.rs, tc_446_source_arg_properties.rs) |
| FR-018-AC-5 | A bare argument maps to a path source                                                             | Test (ts_oracle.rs, tc_446_source_arg_properties.rs, index.test.ts) |

## Dependencies

- **Upstream**: [StR-002](../stakeholder/StR-002-extensible-vocabulary.md);
  [US-003](../usecase/US-003-install-community-module.md).
- **Downstream**: the typed source is installed and recorded by
  [FR-019](./FR-019-manage-plugin-registry.md); live GitHub install is verified
  by [IT-002](../integration/IT-002-github-plugin-install.md).
