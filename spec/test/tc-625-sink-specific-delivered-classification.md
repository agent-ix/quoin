---
id: TC-625
title: "FR-207 AC-9 cross-repository sink identity"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-625: FR-207 AC-9 cross-repository sink identity

## Description

Verify FR-207-AC-9: covered sinks with the same id in different repositories remain distinct, and a classification delivered to repo-a does not suppress repo-b's missing-category gap.

## Test Procedure

From `rust/`, run `cargo test -p quoin-dossier duplicate_sink_ids_in_different_repos_are_retained_and_gapped_per_repo` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

Both sinks are retained as separate rendered entries, identified by their repository-specific processor fields. The repo-a sink has no `personal_data_categories_unstated` gap; repo-b retains exactly one Markdown gap row and one typed gap with the shared sink id.
