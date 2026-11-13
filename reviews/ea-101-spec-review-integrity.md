---
id: SR-20261113092141
title: "Integrity review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #861 (git diff origin/main...ea-101-snapshot-spec at 4325f1fb): spec/functional/FR-166, spec/functional/FR-167, spec/functional/FR-168, spec/non-functional/NFR-036, spec/functional/index.md, spec/non-functional/index.md"
---

# Integrity review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)

## Summary

Ticket: EA-101. I checked IDs, links, index lines, relationships, and whether each acceptance criterion can fail.
`quire validate --scope .` over the six changed files exits 0. The only warning is the module's existing
`semantic.unused-import`. There are no ID collisions, and all four index lines are present. The `ix://` targets
for EA FR-049, FR-051, FR-052 and NFR-007 exist. One relative link is broken, which quire did not catch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-168 links `./FR-063-change-assurance-record.md` twice, in Description and in Dependencies. The file is `FR-063-change-assurance-record-integrity.md`. | spec/functional/FR-168-release-snapshot-change-records.md:21, :70 |
| FND-002 | low | Every other refusal in FR-166 and FR-168 names a code. The "missing, unreadable or fails its own format" refusal names none, so FR-166-AC-5 cannot assert which refusal fired. | spec/functional/FR-166-release-snapshot-command.md (Behavior); FR-166-AC-5 |
| FND-003 | low | FR-167-AC-8 (Demonstration: "a plain JSON reader ... reads every section by member name") cannot fail once FR-167-AC-1's schema check passes. It adds no evidence. Delete it or give it a distinct oracle. | FR-167-AC-8 |
| FND-004 | low | FR-167-AC-2 requires the five `evidence_freshness` keys "in a fixed order", but no order is stated. `protocol::canonical_json` orders keys itself, so the clause is either redundant or undefined. | FR-167-AC-2 |

## Verdict

Minor. FND-001 is a broken link to fix. FND-002 to FND-004 are testability tightenings.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new codes overlap. Under FR-166, an input that "fails its own format" is refused with `release_snapshot_input_invalid`. Under FR-168, a history file with the wrong protocol or a missing member is refused with `release_snapshot_history_invalid`. A malformed history file matches both rules, so implementers can choose different codes. State that FR-168's code takes precedence for `--history`, or exclude `--history` from the FR-166 rule. | spec/functional/FR-166-release-snapshot-command.md:85-86; spec/functional/FR-168-release-snapshot-change-records.md (Behavior) |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Both FR-063 links now point to `./FR-063-change-assurance-record-integrity.md`. |
| FND-002 | fixed | The refusal is split into `release_snapshot_input_unreadable` and `release_snapshot_input_invalid`, and AC-5 asserts each code. The overlap this creates is FND-005. |
| FND-003 | fixed | FR-167-AC-8 is deleted. |
| FND-004 | fixed | AC-2 now reads "exactly the five keys and no other", with no order clause. |
| FND-005 | fixed | FR-166 excludes `--history` from `release_snapshot_input_invalid` and refuses a malformed `--history` with `release_snapshot_history_invalid`, as FR-168 does. FR-166-AC-9 asserts this. |
