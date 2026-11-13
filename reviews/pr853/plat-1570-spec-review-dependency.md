---
id: SR-20261113071105
title: "Dependency review of quoin risk discrepancies (FR-154, FR-155)"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #853 (git diff origin/main...HEAD at 3b00d92c): spec/functional/FR-154, spec/functional/FR-155, spec/functional/index.md, spec/spec.md"
---

# Dependency review of quoin risk discrepancies (FR-154, FR-155)

## Summary

Ticket: PLAT-1570. Checks the frontmatter relationships and the Dependencies sections against
the documents they name, and against quoin's spec-integrity test
`rust/crates/quoin-core/tests/tc_1045_spec_integrity.rs` (tc_118_001). That test treats an id in
a `## Dependencies` line as local unless it is preceded by `quire-rs`, by
"spec-artifacts-process` ", or by an ix://agent-ix/<other-repo>/ link
(`[FR-105](ix://agent-ix/spec-objects-safety/FR-105)`). The form "`agent-ix/spec-objects-safety` FR-105"
is not recognised as foreign. The test was not run, because it needs the workspace build. Its
`local_citations` rule was applied to the two files line by line with a reimplementation.

Enablement: FR-154 depends_on FR-146, and FR-154-AC-8 runs `quoin risk advise`, which is in
open PR #852. FR-154 cannot be built before #852 lands. The edge is declared, so this is
sequencing, not a defect. The traces_to edge to safety StR-001 matches FR-145.

Pre-existing on `origin/main`, outside this diff (surfaced, not counted): by the same rule,
FR-145:67 (FR-305), FR-146:96-97 (FR-101, FR-203, FR-204, FR-305) and FR-148:85 (FR-106,
FR-303), merged in #845, already cite safety ids that resolve to no quoin document. If this
reading is right, tc_118_001 is red on main now.

Examined (role examined): FR-154 and FR-155 frontmatter relationships and Dependencies
sections. Context only: tc_1045_spec_integrity.rs on origin/main; spec-objects-safety
FR-101..106, FR-303; quoin FR-145..148.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | tc_118_001 fails. FR-154:74 cites safety FR-105, and FR-155:88 cites safety FR-101, FR-105 and FR-106, as "`agent-ix/spec-objects-safety` FR-nnn". The test reads these as quoin ids, and quoin has no FR-101, FR-105 or FR-106. The same lines also resolve safety FR-102, FR-103 and FR-104 to unrelated quoin documents. Use the link form [FR-105](ix://agent-ix/spec-objects-safety/FR-105) for every safety id in Dependencies. | FR-154:74, FR-155:88 |
| FND-002 | medium | post-control-estimate-without-decision is spec-objects-safety FR-303's finding: FR-303 owns the relation, the pre_control exclusion, and the rule that an undecided record satisfies it. FR-148 also cites FR-303 for it. FR-155 calls it "an addition to FR-105" and "the token of FR-148", and declares no edge or Dependencies entry to FR-303. Cite FR-303 as its source and add a depends_on relationship. | FR-155:33-35, FR-155:4-17 |
| FND-003 | low | Dependencies and Description cite safety FR-101 (the definition of the acceptable region), but frontmatter relationships have no edge to it, unlike FR-146. Add depends_on ix://agent-ix/spec-objects-safety/FR-101. | FR-155:4-17, FR-155:88 |

## Verdict

Not mergeable: FND-001 turns a repository test red. All three fixes are citation edits inside
this PR. The pre-existing main failures in FR-145/146/148 need their own fix.
