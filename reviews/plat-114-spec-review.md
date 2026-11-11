---
id: SR-20261111190503
title: "Spec review of quoin FR-132 (paired-run metrics)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@e29f1c25; spec/functional/FR-132-paired-run-metrics.md, spec/functional/index.md, spec/spec.md, spec/matrix.md"
review_set: subset
---

# Spec review of quoin FR-132

## Summary

Ticket: PLAT-114. Read against engineering-assurance FR-034 and its
`measurement-plan-frontmatter.schema.json` `paired_run` definition at 4e8df92.
`quire validate` on the changed files reports only the pre-existing TC-1943/TC-1944
tombstone rows in `spec/matrix.md`, which are on `origin/main` and outside this diff.

## Verdict

**PASS WITH FINDINGS (one medium, one low).** FR-132 states the mechanism, the inclusive
band, the inconclusive and incomparable outcomes and no verdict, and claims no NFR is
measured. It disagrees with FR-034 on two member rules without saying so.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-132-AC-7 refuses a `paired_run` that lacks `minimum_pairs`, but EA FR-034 makes `minimum_pairs` optional with its default left to the consumer. FR-132 neither states quoin's default nor says that it deliberately refuses an EA-valid plan. State a default that can never make zero comparisons pass, or state the refusal as deliberate | spec/functional/FR-132-paired-run-metrics.md:49 |
| FND-002 | low | The FR-132 description allows a "non-negative" `delta`, and AC-7 does not refuse members other than `relation`, `minimum_pairs` and `delta`. FR-034 requires `delta` greater than 0 and forbids extra members | spec/functional/FR-132-paired-run-metrics.md:19-21 |
