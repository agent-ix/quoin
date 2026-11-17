---
id: TC-622
title: "FR-207 AC-6 invalid processing activity scopes relationships"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-622: FR-207 AC-6 invalid processing activity scopes relationships

## Description

Verify FR-207-AC-6: wrong-kind and dangling security-owned `scopes` targets on a `processing_activity` are rejected before a records-of-processing document is generated. Also pass a synthetic `quire.flow-report/v1` fixture directly to the generator with a `covers` edge from an activity to a valid sink and no corresponding `scopes` edge; the ISO edge must not select the sink. This synthetic report tests the consumer boundary and does not claim that the security module permits an authored activity-to-sink `covers` edge.

## Test Procedure

Run `dangling_and_wrong_kind_scopes_fail_validation_before_generation` and `generated_records_of_processing_ignores_iso_covers_edges` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`. The first creates a temporary module schema with security-owned `scopes` links, validates frontmatter relationships with Quire's bundle validator, asserts `dangling-reference` for `MISSING-1` and `disallowed-edge-target` for `QUEUE-1` with target type `queue`, then drives those failures through the built-in dossier registry and `generate`. The second sends a synthetic covers-only flow report through the same registry and verifies both the rendered document and report gaps.

## Expected Results

Wrong-kind and dangling `scopes` target diagnostics are returned as source failures; no generated dossier can claim those records as valid. In the synthetic flow-report case, the sink targeted only by `covers` is absent from scoped sink output and contributes no sink-specific entries or gaps.
