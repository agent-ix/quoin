---
id: SR-1624
title: "Rust code review — PLAT-1658 records-of-processing generator (Quoin PR #962)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier/src/records_of_processing.rs, spec/functional/FR-207-dossier-records-of-processing-content.md"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-207", type: references}
---

## Summary

Rust code review for PLAT-1658/Quoin PR #962. Rust-specific review is folded into this code-review artifact. On review head `8589b1ab`, `transfer_entry` converted the region list to one comma-separated string before comparing it against individual `home_regions` labels. A sink with multiple exact matching region labels was therefore incorrectly reported as outside those labels. This was recorded as FND-001. The fix round at `2c9be0f2` iterates each authored label and has both mixed-match and all-match fixtures.

## Verdict

FND-001 is fixed on current PR head `2c9be0f241762628715f8f321d3c9ba597105552`. Current code preserves array semantics when comparing and rendering the model location. The new test asserts an outside label alongside a matching label and an all-matching multi-label sink. No additional code-review finding was added in this disposition pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Transfer location comparison joins region arrays before matching home-region labels | rust/crates/quoin-dossier/src/records_of_processing.rs:503-523 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | Flow facts are indexed by artifact id alone, so duplicate ids across selected repository members can resolve to another member's data | rust/crates/quoin-dossier/src/records_of_processing.rs:344-352,391-395 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Current `transfer_entry` maps each `region` member independently and compares each member to the authored `home_regions` values. The updated TC-617 fixture covers both a mixed matching/non-matching list and an all-matching multi-label list. |
| FND-002 | still-open | `FlowFact` and `FlowDocumentRef` carry an optional repository identity, but `facts_by_id` and `fact_for` discard it and key only by id. With multiple selected modules that contain the same artifact id, an edge can resolve to a different repository member's fact and render the wrong record. |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-207 AC-1 and location-label comparison | examined | FND-001 fixed; FND-002 remains open for multi-repository model identities |
| FR-207 source boundary (`quire.flow-report/v1`) | examined | FND-002 |

