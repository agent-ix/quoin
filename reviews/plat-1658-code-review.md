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

FND-001, FND-002, and FND-003 are fixed in the final implementation. TC-617 exercises list-valued regions, duplicate repository identities, and per-sink missing-classification gaps.

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
| FND-002 | fixed | `facts_by_id` now keys facts by `(repo, id)` and `fact_for` looks up the target's `(repo, id)`. The regression test creates duplicate ids in two repositories and asserts that an edge with `repo: selected/repo` resolves the selected repository's processor value. |
| FND-003 | fixed | Classification suppression is scoped to the sink resolved from each connection's `delivers_to` edge, including the repository identity. The regression test uses the same sink id in two repositories, gives only repo-a's sink a classified delivery, and asserts that the repo-b missing-category gap remains. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A classification on any delivering connection suppresses the missing-classification gap for every covered sink, including sinks that connection does not deliver to | rust/crates/quoin-dossier/src/records_of_processing.rs:531-537 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-207 AC-1 and location-label comparison | examined | FND-001 fixed; FND-002 fixed for multi-repository model identities |
| FR-207 source boundary (`quire.flow-report/v1`) | examined | FND-002 and FND-003 fixed |

## Clean-main baseline

- Command: `git -C /tmp/quoin-plat1658-clean-main diff --check` in a detached worktree at clean `origin/main` commit `e73463dc`.
- Result: exit 0; no whitespace errors.
