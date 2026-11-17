---
id: TC-624
title: "FR-207 AC-8 deterministic records-of-processing output"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-624: FR-207 AC-8 deterministic records-of-processing output

## Description

Verify FR-207-AC-8: the same authored activity records and flow report generate byte-identical dossier Markdown and deterministic JSON output, including ordered gap rows, without requiring a commit or pre-generation check.

## Test Procedure

Run `tc_1658_controller_lists_classification_recipients_transfer_gaps_and_safeguards` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`. The test renders the same fixture twice, compares the Markdown body and ordered gaps, then serializes each run's body and gaps as JSON bytes and compares those bytes.

## Expected Results

Two in-memory generations have identical rendered Markdown, ordered gap values, and byte-identical JSON serialization of the generated body and gaps.
