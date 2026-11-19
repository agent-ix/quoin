---
id: TC-014
title: "Default module manifest pins all ten public modules to release refs"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-016"
    type: "verifies"
---

# TC-014: Default module manifest pins all ten public modules to release refs

## Description

Verify that the committed default module manifest is valid, contains exactly
the ten public default modules, and declares each Git source at its expected
release tag. The process module must use v0.34.0, which contains the
`allowed_links` edge type required by `TestMatrix`.

## Test Procedure

1. Run the `default_modules_use_the_expected_public_release_pins`
   unit test in `rust/crates/quoin-cli/src/module.rs` through
   `/home/peter/dev/worktrees/locked-build.sh`.
2. Confirm that each of the ten repository/ref pairs in Expected Results is a
   published GitHub release tag by inspecting the release record. Then, in a
   fresh isolated IX home, run `quoin module ensure-defaults` using the
   branch-built binary and confirm that all ten declared refs resolve.
3. Run `quire schema --module <installed-process-module> TestMatrix` and
   inspect its exit status and diagnostics.

## Expected Results

The manifest parses as a typed marketplace manifest and contains exactly these
ten public Git-subdirectory module refs:

| Module | Repository | Module path | Release ref |
| --- | --- | --- | --- |
| engineering-assurance | agent-ix/engineering-assurance | engineering_assurance | v0.10.1 |
| spec-artifacts-iso | agent-ix/spec-artifacts-iso | spec_artifacts_iso | v0.27.0 |
| spec-artifacts-app | agent-ix/spec-artifacts-app | spec_artifacts_app | v0.10.0 |
| spec-artifacts-process | agent-ix/spec-artifacts-process | spec_artifacts_process | v0.34.0 |
| spec-objects-business | agent-ix/spec-objects-business | spec_objects_business | v0.10.0 |
| spec-objects-architecture | agent-ix/spec-objects-architecture | spec_objects_architecture | v0.11.0 |
| spec-objects-enterprise | agent-ix/spec-objects-enterprise | spec_objects_enterprise | v0.9.0 |
| spec-objects-operational | agent-ix/spec-objects-operational | spec_objects_operational | v0.10.0 |
| spec-objects-security | agent-ix/spec-objects-security | spec_objects_security | v0.11.0 |
| spec-objects-safety | agent-ix/spec-objects-safety | spec_objects_safety | v0.8.0 |

The isolated install resolves each ref to its source release commit. The
process module's resolved ref is v0.34.0. `quire schema TestMatrix` exits zero
without an `UnknownEdgeType` diagnostic; any separate `UnknownRole`
diagnostics remain visible.
