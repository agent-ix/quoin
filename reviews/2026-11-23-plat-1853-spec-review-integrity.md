---
id: SR-17101
title: "Integrity review of the PLAT-1853 customer-artifact retained kind"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md, spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md (PR #1045, git diff origin/main...e0476e21)"
review_set: subset
---

# Integrity review of the PLAT-1853 customer-artifact retained kind

## Summary

Ticket: PLAT-1853. I checked the structure of the three changed FRs: whether
quire mints the new criteria and whether the new text sits under the right
sections. It found two structural defects. `quire validate` exits 0 on all
three files, but `quire matrix --scope .` does not list FR-211-AC-25 to AC-30.
It does list FR-030-AC-36 to AC-41, FR-030-CON-4, FR-030-CON-5 and FR-133-AC-8.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-211-AC-25 to AC-30 are never minted. A blank line (FR-211 line 230) between the AC-24 row and the AC-25 row ends the Acceptance Criteria table, so the six new rows are a header-less fragment and not a table. `quire matrix --scope .` lists FR-211-AC-1 to AC-24 only. The new discharge criteria, which are the core of the decision, are invisible to the matrix and to trace tags. | spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md:229-236 | wrong-requirement |
| FND-002 | medium | The new `### Customer artifacts` heading (FR-030 line 98) is inserted after the first `## Behavior` bullet, with no blank line before it. Every later Behavior bullet now falls under "Customer artifacts". `quire lookup --heading Behavior` returns one bullet, and `--heading "Customer artifacts"` returns 42, of which 36 are unrelated store rules (passing-symbol binding, the writer lock, gc and timestamps). The subsection belongs at the end of Behavior, or its bullets belong after the store rules. | spec/functional/FR-030-evidence-store.md:94-136 | wrong-requirement |

## Verdict

**Changes requested.** FND-001 is high: until the blank line goes, none of the
six FR-211 acceptance criteria exist for the matrix. FND-002 misfiles existing
requirements under the new section.

## Examined

The FR-030 `## Behavior` structure, the new `### Customer artifacts`
subsection, and the Constraints and Acceptance Criteria tables (CON-4, CON-5,
AC-36 to AC-41). The FR-133 Behavior bullet and AC-8 row. The FR-211 Evidence
references and Customer-owned clauses sections, and the Acceptance Criteria
table (AC-24 to AC-30). The minted ids came from `quire matrix --scope .` and
the section contents from `quire lookup`.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The blank line is gone. `quire matrix --scope .` at d64c733b lists FR-211-AC-25 through AC-30. |
| FND-002 | fixed | `### Customer artifacts` now sits at the end of `## Behavior`, before `## Constraints`. `quire lookup --heading Behavior` returns the 36 store bullets, and `--heading "Customer artifacts"` returns its 12 own bullets. |
