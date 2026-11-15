---
id: SR-20261115183514
title: "EARS conformance — PLAT-1441 FR-121 frontmatter-owned record keys"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; PLAT-1441"
review_set: subset
---

## Summary

Ticket: PLAT-1441. EARS review of the four statements added or changed in
FR-121 by quoin PR #927 (`0799418e`). `quoin`'s automated EARS lint could not be
run (the sandbox requires approval for the binary), so this is a manual reading.

## Verdict

**CONDITIONAL.** Lines 86 and 87 are well-formed unwanted-behaviour (`If ... then
Quire SHALL ...`) statements with one response each. Line 80 is a well-formed
negative ubiquitous statement. Line 88 cannot be verified as written, and line 85
has a non-system subject.

## Coverage

Examined: FR-121 Behavior line 80 (frontmatter prohibition with exception),
line 85 (ownership), line 86 (section-only error), line 87 (both-form warning),
line 88 (unchanged behaviour). All are listed in the comment scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Line 88, "Keys owned by any other mapping, and every lifecycle key read from a body section, SHALL keep the behavior stated in this requirement", points back at its own requirement and states no response. It is also ambiguous when a module declares a `frontmatter_field` locator for a lifecycle key such as `status`: is that key "a lifecycle key read from a body section" or frontmatter-owned? | spec/functional/FR-121-record-key-sections-mapping.md:88 |
| FND-002 | low | The subject of line 85 is a locator ("A `frontmatter_field` locator ... SHALL own that key"), not the system, and its second sentence ("An absent frontmatter field leaves the key absent; it does not hand the key to a section") has no SHALL. Rephrased as "Quire SHALL treat a key ... as frontmatter-owned ... and SHALL NOT read it from a section", it could be verified. | spec/functional/FR-121-record-key-sections-mapping.md:85 |

## New findings (disposition pass 1)

Round 1 re-read the seven Frontmatter-owned keys statements at `21db2ff6`, lines 87-93. Lines 87, 88, 90, 91, 92 and 93 have a system subject and one response each. The finding below does not block merge.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | Line 89 joins two obligations with different triggers in one bullet. The first is "If the frontmatter holds no value ... then Quire SHALL leave the key absent". The second is "the absence of a section SHALL NOT erase a value the frontmatter projects", which applies when frontmatter does hold a value. Its "whether or not a section names it" also restates line 90. Split it into two statements. | spec/functional/FR-121-record-key-sections-mapping.md:89 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Line 93 has a system subject and names its scope. A key that a `frontmatter_field` locator owns, lifecycle or not, is frontmatter-owned (line 87). The behaviour of the other sections applies only to keys no such locator owns. |
| FND-002 | fixed | Line 87 is now "Quire SHALL treat ... and SHALL NOT read that key from a section". The absent-field rule is line 89, with a SHALL. |

- FND-001 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:93`): `Quire SHALL keep the behavior stated in the other sections for a key owned by another mapping, and for a key read from a body section, including a lifecycle key, that no frontmatter_field locator owns.`
- FND-002 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:87`): `Quire SHALL treat a record key as frontmatter-owned when the object type declares a frontmatter_field locator under that key in yield_pattern.match (quire-rs FR-069), whether or not the artifact's frontmatter holds the field, and SHALL NOT read that key from a section.`

## Dispositions (pass 2)

Pass 2, FR-121 at `ab5356be`. Lines 89 and 90 each have one trigger and one response. Lines 123 and 124 are well-formed `If ... then Quire SHALL` statements. No new finding from this method. Line 91's self-contradiction is a soundness defect, not a grammar defect, so it is recorded as code-review FND-006 and not repeated here.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | Line 89 is now split into two statements. Line 89 covers no frontmatter value: the key is absent from `record` and not defaulted. Line 90 covers a frontmatter value with no section: the value is kept. Each bullet has a single trigger. |

- FND-003 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:89-90`): `If the frontmatter holds no value for a frontmatter-owned key, then Quire SHALL leave the key absent from record, whether or not a section names it, and SHALL NOT default it.` / `If the frontmatter holds a value for a frontmatter-owned key and no section names the key, then Quire SHALL keep the frontmatter value in record; the absence of a section SHALL NOT erase a value the frontmatter projects.`

## Dispositions (pass 3)

Pass 3, FR-121 at `8b7a934e`. Line 91 was rewritten. It is now a `When ... Quire SHALL` statement with one trigger and one response. The `because` clause gives a reason and does not add a second response. No new findings. FND-001..FND-003 stay fixed: lines 87, 89, 90 and 95 are unchanged since pass 2.
