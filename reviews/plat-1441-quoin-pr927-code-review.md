---
id: SR-20261115183512
title: "Code review — PLAT-1441 FR-121 frontmatter-owned record keys (quoin#927)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; PLAT-1441"
review_set: subset
---

## Summary

Ticket: PLAT-1441. Reviewed quoin PR #927 at `0799418e` against `origin/main`: a
Markdown-only change to FR-121 that adds a `### Frontmatter-owned keys`
subsection, relaxes the frontmatter prohibition, adds FR-121-AC-10..12, and adds
`depends_on` edges to quire-rs FR-069 and FR-081. The sibling engine spec change
is quire-rs PR #688 (open), which was read for cross-repository consistency.

`quire validate` and `quoin` could not be run in this session (the sandbox
requires approval for both binaries), so frontmatter and table structure were
checked by reading. The two `ix://agent-ix/quire-rs/FR-069` and `FR-081` targets
exist on quire-rs `main`.

## Verdict

**CHANGES REQUESTED.** The new subsection states the intended behaviour
(section-only error, both-form warning with frontmatter winning, absent field
leaves the key absent) and matches the ticket. But the general rules it overrides
were not amended, and its availability rule contradicts the engine spec it
depends on.

## Coverage

Examined: FR-121 Behavior lines 76-81 (owned-key set, owned-key-as-prose,
frontmatter prohibition, absent-section rule), the new lines 85-88, Outputs
line 64 (`recordSources`), FR-121-AC-8, AC-10, AC-11, AC-12, the new
relationships and Dependencies bullet. Context: quire-rs FR-081 on `main` and as
amended in quire-rs#688, and quire-rs FR-069 as amended in quire-rs#688.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The closed owned-key set (line 76) and the rule that a heading naming an owned key or no key "SHALL be prose" (line 77, with AC-8's "no diagnostic") were not amended. A frontmatter-owned key is neither listed as owned nor excepted, so line 76 reads its section as a record key and line 77 or line 86 apply in conflict. quire-rs#688 adds exactly this exception to FR-081; FR-121 does not. | spec/functional/FR-121-record-key-sections-mapping.md:76-77 |
| FND-002 | high | FR-121 line 86 and AC-10 require `availability.recordKeys` `unavailable` for a section-only frontmatter-owned key, but quire-rs FR-081 as amended in quire-rs#688 says the section "adds no `recordSources` entry and no `availability.recordKeys` value for that key". The fixtures (FR-121-CON-2, owned by quire-fixtures) cannot satisfy both specs. | spec/functional/FR-121-record-key-sections-mapping.md:86 |
| FND-003 | medium | Line 81 still says that when no section names a key, Quire SHALL omit the key and report it `not_applicable`, with no exception. This contradicts line 87 and AC-12, where a frontmatter-owned key with a frontmatter value and no section is extracted. Neither the `recordSources` entry (Outputs line 64 spans "the key's section") nor the availability state of a frontmatter-sourced key is stated. | spec/functional/FR-121-record-key-sections-mapping.md:81 |
