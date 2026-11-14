---
id: TC-013
title: "Tag attestation preserves Quoin's manual release paths"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-112"
    type: "verifies"
---

# TC-013: Tag attestation preserves Quoin's manual release paths

## Description

Verify Quoin's tag-attestation workflow is additive and the existing manual npm
and native release paths retain their input and publish gates.

## Test Procedure

1. Parse `.github/workflows/release.yml` as YAML and inspect its trigger,
   `publish` input, npm publish action, and post-publish smoke step.
2. Parse `.github/workflows/native-release.yml` and inspect its trigger.
3. Parse `.github/workflows/tag-attestation.yml` and inspect its tag trigger
   and job dependencies.

## Expected Results

1. `release.yml` is triggered by `workflow_dispatch`; its `publish` input is a
   boolean defaulting to `false`; the npm publish action receives
   `${{ inputs.publish }}`; and the post-publish smoke step runs only when
   `inputs.publish` is true.
2. `native-release.yml` remains triggered by `workflow_dispatch`.
3. `tag-attestation.yml` is triggered by pushes of `v*` tags and has no job
   dependency on either manually dispatched release workflow.
