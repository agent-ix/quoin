---
id: TC-626
title: "Direct approval references remain optional direct-fact metadata"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-046-AC-9"
    type: "verifies"
---

# TC-626: Direct approval references remain optional direct-fact metadata

## Description

Verify that a direct evidence fact rejects an empty or whitespace-only
`approvalRef`, retains a valid supplied reference through persistence, reading
and report generation, and remains direct evidence. Verify that omitting the
reference remains valid.

## Test Procedure

1. Run `direct_approval_ref_is_validated_retained_and_stays_in_the_direct_population`
   in `quoin-assurance`'s store unit tests.

## Expected Results

1. Empty and whitespace-only references are rejected before any store write.
2. A nonempty reference is preserved verbatim in the stored fact, read fact,
   and report's direct population; the clause remains classified as direct.
3. A fact without `approvalRef` is stored, read and reported as direct, with the
   optional field omitted from serialized output.
