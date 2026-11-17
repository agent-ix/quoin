---
id: TC-625
title: "FR-207 AC-3 sink-specific delivered classification"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-625: FR-207 AC-3 sink-specific delivered classification

## Description

Verify FR-207-AC-3: a classified connection delivering to one sink does not suppress the missing-category gap for another sink, including when both sinks share an artifact id in different repositories.

## Test Procedure

From `rust/`, run `cargo test -p quoin-dossier delivered_classification_only_suppresses_gap_for_its_sink` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The repository-a sink reached by the classified connection has no personal-data category gap. The repository-b sink with the same artifact id retains one Markdown gap row and one typed `personal_data_categories_unstated` gap.
