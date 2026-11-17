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
| FND-003 | medium | A classification on any delivering connection suppresses the missing-classification gap for every covered sink, including sinks that connection does not deliver to | rust/crates/quoin-dossier/src/records_of_processing.rs:531-537 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Current `transfer_entry` maps each `region` member independently and compares each member to the authored `home_regions` values. The updated TC-617 fixture covers both a mixed matching/non-matching list and an all-matching multi-label list. |
| FND-002 | still-open | `FlowFact` and `FlowDocumentRef` carry an optional repository identity, but `facts_by_id` and `fact_for` discard it and key only by id. With multiple selected modules that contain the same artifact id, an edge can resolve to a different repository member's fact and render the wrong record. |
| FND-002 | fixed | `facts_by_id` now keys facts by `(repo, id)` and `fact_for` looks up the target's `(repo, id)`. The regression test creates duplicate ids in two repositories and asserts that an edge with `repo: selected/repo` resolves the selected repository's processor value. |
| FND-003 | still-open | For each sink, `delivered_classification` checks whether any connection delivering to any covered sink carries a classification. With sinks A and B and a classified connection delivering only to A, that boolean is true for B too, so B gets no missing-category gap. The test/code should associate the classification with the specific sink reached by the carrying connection. |
| FND-003 | fixed | The code now checks that the carrying connection's resolved `delivers_to` target matches the current sink by `(repo, id)`. `delivered_classification_only_suppresses_gap_for_its_sink` verifies SINK-A has no classification gap while SINK-B retains both the rendered row and structured gap. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A classification on any delivering connection suppresses the missing-classification gap for every covered sink, including sinks that connection does not deliver to | rust/crates/quoin-dossier/src/records_of_processing.rs:531-537 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-207 AC-1 and location-label comparison | examined | FND-001 fixed; FND-002 remains open for multi-repository model identities |
| FR-207 source boundary (`quire.flow-report/v1`) | examined | FND-002 |

