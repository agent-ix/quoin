---
id: SR-20261113095722
title: "Dependency review of quoin risk-summary (FR-169, FR-170)"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #863 (git diff origin/main...HEAD at a1ece4e9): spec/functional/FR-169, spec/functional/FR-170"
---

# Dependency review of quoin risk-summary (FR-169, FR-170)

## Summary

Ticket: PLAT-1576. Checks the frontmatter `relationships`, the `## Dependencies` sections, and the
cross-repository citation rule that `rust/crates/quoin-core/tests/tc_1045_spec_integrity.rs` enforces
(another repository's id is cited only as an `ix://agent-ix/<repo>/<id>` link, never bare).

Every relationship target resolves: quoin FR-145, FR-146, FR-147, FR-155, FR-169 and FR-170 exist on the
branch, and the spec-objects-safety targets exist on its `origin/main`. Both `## Dependencies` sections
cite safety ids as `ix://` links only, so tc_118_001 passes. FR-169 `implements` FR-305 and FR-170
`depends_on` FR-169 with FR-169 only `references` FR-170, so there is no dependency cycle. The work is
feature work with no enablement gap: every upstream requirement is on `origin/main`. FR-302's manifest
relations wait on quire-rs PLAT-1674, but FR-170 computes the control tokens with the FR-147 code, so
that ticket does not block this command.

Examined (role examined): FR-169 and FR-170 frontmatter relationships, body citations and Dependencies
sections. Context only: tc_1045_spec_integrity.rs; spec-objects-safety FR-103, FR-204, FR-301..305;
quoin FR-145, FR-147, FR-155.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-170 token table's "Rule from" column cites FR-301, FR-302, FR-303 and FR-105 bare, twelve times. These are safety-module ids; in quoin FR-301..303 and FR-105 resolve to no document. tc_118_001 reads only the Dependencies section, so it stays green, but the rule is ix:// links only. Link each cell. | FR-170:48-59 |
| FND-002 | low | Upstream edges are missing for rules the text relies on: FR-169 cites safety FR-103 for "a hazard's status is no decision" but neither frontmatter nor Dependencies lists it; FR-170 takes its finding shape "as in FR-145" without listing FR-145. Add the two edges. | FR-169:125, FR-170:43 |

## Verdict

Mergeable on this lens once the two low findings are fixed; neither blocks on its own.

## Dispositions

Round 1, against commit 68717d15.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
