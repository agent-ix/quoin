---
id: TC-014
title: "Default module manifest uses public modules and released source refs"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-016"
    type: "verifies"
---

# TC-014: Default module manifest uses public modules and released source refs

## Description

Verify that the committed default module manifest is valid, contains exactly
the ten public default modules, and pins the process module to its released
v0.34.0 source containing the `allowed_links` edge type.

## Test Procedure

1. Run the `default_process_module_tracks_the_release_with_allowed_link_types`
   unit test in `rust/crates/quoin-cli/src/module.rs` through
   `/home/peter/dev/worktrees/locked-build.sh`.
2. In a fresh isolated IX home, run `quoin module ensure-defaults` using the
   branch-built binary. Confirm that the process module is installed from the
   v0.34.0 release tag.
3. Run `quire schema --module <installed-process-module> TestMatrix` and
   inspect its exit status and diagnostics.

## Expected Results

The manifest parses as a typed marketplace manifest and contains the ten
public default module names. Its process module source is the
`agent-ix/spec-artifacts-process` `spec_artifacts_process` Git subdirectory at
ref `v0.34.0`. The isolated install resolves that ref to the published release
commit. `quire schema TestMatrix` exits zero without an `UnknownEdgeType`
diagnostic; any separate `UnknownRole` diagnostics remain visible.
