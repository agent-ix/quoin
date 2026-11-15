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

## New findings (disposition pass 1)

Round 1 re-read FR-121 at `21db2ff6` and quire-rs FR-069 and FR-081 at quire-rs#688 head `7b43672f`. Neither finding below blocks merge. Each is a one-clause clarification. In each case the ACs, line 78 ("governed by Frontmatter-owned keys") and quire-rs#688 already fix one reading.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The general Availability rule (line 121, "If any header, row, block, or section of a key carries an error, then Quire SHALL report `availability.recordKeys` for that key `unavailable`") has no frontmatter-owned exception. Line 90 says a section-only frontmatter-owned key's error adds no `availability.recordKeys` entry. Line 78 sends such keys to Frontmatter-owned keys, so the specific rule wins. But the Availability rule is the one general rule round 1 did not qualify. | spec/functional/FR-121-record-key-sections-mapping.md:121 |
| FND-005 | low | For a frontmatter-owned key with no section (AC-12, frontmatter value; AC-13, no value), FR-121 states no `recordSources` and no `availability.recordKeys` outcome. Lines 90-91 say "no entry" only when a section names the key, and line 83 now excludes the key from `not_applicable`. quire-rs FR-081's unamended absent-section rule ("omit a key whose section is absent, reporting it `not_applicable`") would give `not_applicable`. The golden fixtures (FR-121-CON-2) will therefore take their expected availability from FR-081, not FR-121. | spec/functional/FR-121-record-key-sections-mapping.md:83 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Line 78 adds `frontmatter_field`-owned keys to the owned set and routes them to Frontmatter-owned keys. Line 79 and AC-8 except them from the owned-key-as-prose rule. This matches quire-rs#688 FR-081. |
| FND-002 | fixed | Line 90 and AC-10 now say no `record` key, no `recordSources` entry and no `availability.recordKeys` entry. This matches quire-rs#688 FR-081 ("adds no `recordSources` entry and no `availability.recordKeys` value"). |
| FND-003 | fixed | Line 83 excludes frontmatter-owned keys from the absent-section `not_applicable` rule. Line 89 says an absent section does not erase a projected value. The availability state of a no-section frontmatter-owned key is still unstated; that residual is tracked separately as FND-005 (low). |

- FND-001 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:78-79`): `A key a frontmatter_field locator owns is also owned, and is governed by Frontmatter-owned keys.` / `A section whose heading names no readable record key, including a heading that names an owned key other than a frontmatter_field-owned key, SHALL be prose to this mapping.`
- FND-002 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:90`): `If a section heading names a frontmatter-owned key and the frontmatter holds no value for the field, then Quire SHALL emit the error semantic.feature-not-extractable at the heading with reason frontmatter-owned, extract no value from the section, and add no key to record, no recordSources entry and no availability.recordKeys entry for it.`
- FND-003 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:83, 89`): `When the artifact has no section naming a key that is not frontmatter_field-owned, Quire SHALL omit the key and report availability.recordKeys for it not_applicable.` / `the absence of a section SHALL NOT erase a value the frontmatter projects.`

## New findings (disposition pass 2)

Pass 2 re-read FR-121 at `ab5356be` and quire-rs FR-081 at quire-rs#688 head `221478ab`. The lead reports that `quire validate` passes on this exact head, with only the existing unused-import warning. That was not re-run here.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Line 91 contradicts itself. It says Quire "SHALL report the section mapping `not_applicable` for the absent section and SHALL add no ... `availability.recordKeys` entry for the key". `availability.recordKeys` is the only availability output this mapping has (Outputs line 67), and "section mapping" appears nowhere else in the spec, so the first clause has no channel and cannot be verified. An implementer reading `not_applicable` as an availability state emits the entry that the second clause forbids. quire-rs FR-081 at #688 `221478ab` does exactly that: it keeps the unqualified rule "omit a key whose section is absent, reporting it `not_applicable`", and its Outputs make `availability.recordKeys` present when a section "fails to name a key". Fix: delete "report the section mapping `not_applicable` for the absent section and". quire-rs#688 should then except frontmatter-owned keys from FR-081's absent-section rule. | spec/functional/FR-121-record-key-sections-mapping.md:91 |

## Dispositions (pass 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | Line 123 limits the general Availability rule to keys that are not frontmatter-owned. Line 124 sends a frontmatter-owned key's section error to Frontmatter-owned keys and adds no `record`, `recordSources` or `availability.recordKeys` entry from the section. |
| FND-005 | fixed | Line 91 now states the no-section outcome: no `recordSources` entry, no `availability.recordKeys` entry, and the `record` value comes from the `frontmatter_field` locator alone. Lines 89-90 cover the `record` outcome with and without a frontmatter value. The new wording has a defect of its own, recorded as FND-006. |

- FND-004 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:123-124`): `If any header, row, block, or section of a key that is not frontmatter-owned carries an error, then Quire SHALL report availability.recordKeys for that key unavailable ...` / `If a section of a frontmatter-owned key carries an error, then Quire SHALL apply the rules of Frontmatter-owned keys instead and SHALL NOT add a record, recordSources or availability.recordKeys entry for the key from the section.`
- FND-005 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:91`): `When no section names a frontmatter-owned key, Quire SHALL report the section mapping not_applicable for the absent section and SHALL add no recordSources or availability.recordKeys entry for the key; the independent frontmatter_field locator alone supplies the record value when the frontmatter holds one.`

Pass-2 verdict: **CHANGES REQUESTED**, on FND-006 only. It is a one-clause deletion. Everything else in the change is closed.

## Dispositions (pass 3)

Pass 3 re-read FR-121 at `8b7a934e`. The lead reports that `quire validate` passes on this exact head, with only the existing unused-import warning. That was not re-run here. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The `not_applicable` clause is gone from line 91. The line now says only that Quire adds no `recordSources` or `availability.recordKeys` entry for a frontmatter-owned key with no section. That matches lines 92-93 and 124, and AC-12 and AC-13 now assert it. Line 83 keeps `not_applicable` for keys that are not frontmatter-owned, so the two absent-section rules do not overlap. |

- FND-006 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:91`): `When no section names a frontmatter-owned key, Quire SHALL add no recordSources or availability.recordKeys entry for the key, because the frontmatter projection is independent of the section mapping; the frontmatter_field locator alone supplies the record value when the frontmatter holds one.`

FND-001..FND-005 were re-checked at this head and stay fixed. Lines 78-79, 83, 87-90, 92 and 123-124 are unchanged since pass 2. Follow-up outside this PR: quire-rs#688 should except frontmatter-owned keys from FR-081's absent-section `not_applicable` rule, so the engine spec agrees with line 91.

Pass-3 verdict: **APPROVED.** No finding is open.
