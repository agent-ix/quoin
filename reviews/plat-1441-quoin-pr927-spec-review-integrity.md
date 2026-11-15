---
id: SR-20261115183515
title: "Integrity analysis — PLAT-1441 FR-121 frontmatter-owned record keys"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; PLAT-1441"
review_set: subset
---

## Summary

Ticket: PLAT-1441. Integrity check of the FR-121 statement-to-AC mapping for the
frontmatter-ownership change in quoin PR #927 (`0799418e`). It checks
completeness, atomicity and testability. The ID sequence AC-10..12 is
contiguous, the table has three columns as before, and every new AC is `Test`.

## Verdict

**CONDITIONAL.** Line 86 maps to AC-10, line 87 to AC-11, and the frontmatter-only
and unchanged-key cases to AC-12. All three can fail. Missing: the case where
neither form is present, which carries the never-default rule, and atomicity in
AC-12.

## Coverage

Examined: FR-121 Behavior lines 85-88; FR-121-AC-10, AC-11, AC-12; FR-121-CON-1.
AC-10 and AC-11 have no findings: each names the code, reason, location and
record outcome a fixture can assert.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No AC covers a declared `frontmatter_field`-owned key with neither a frontmatter value nor a section. This is the line 85 rule "an absent frontmatter field leaves the key absent" plus FR-121-CON-1. No test asserts that the key is absent and not defaulted, or what its `availability.recordKeys` state is. | spec/functional/FR-121-record-key-sections-mapping.md:85 |
| FND-002 | low | FR-121-AC-12 bundles three independent behaviours: frontmatter-only extraction with no diagnostic, an other-mapping key (`## Thresholds`) staying as in AC-8, and a lifecycle key (`## Status`) staying as in AC-4. One pass/fail verdict cannot say which one regressed. | spec/functional/FR-121-record-key-sections-mapping.md:142 |

## Dispositions

Round 1, FR-121 at `21db2ff6`. AC-10..AC-16 run contiguously, each AC is `Test`, and the table keeps three columns. The new AC-16 covers the token scope of line 88. No new findings from this method. AC-12 and AC-13 assert no availability state, a gap tracked as code-review FND-005 and not repeated here.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-13 covers a frontmatter-owned key with neither form present: the key is absent, not empty or defaulted, and there is no diagnostic. |
| FND-002 | fixed | AC-12 is split: AC-12 covers frontmatter-only extraction, AC-14 the other-mapping key, and AC-15 the lifecycle key without a locator. Each has its own verdict. |

- FND-001 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:148`): `FR-121-AC-13 | A fixture with a frontmatter_field-owned key, no frontmatter value and no section yields a record without the key, not an empty or default value, and no diagnostic.`
- FND-002 after_excerpt (`spec/functional/FR-121-record-key-sections-mapping.md:147-150`): `FR-121-AC-12 | A fixture with the frontmatter value and no section extracts the frontmatter value with no diagnostic.` / `FR-121-AC-14 | With a frontmatter_field locator declared for one key, a heading that names a key owned by another mapping (## Thresholds) is prose with no diagnostic, as in AC-8.` / `FR-121-AC-15 | ... a lifecycle key that has no such locator (## Status) is read from its section, as in AC-4.`

## New findings (disposition pass 2)

Pass 2, FR-121 at `ab5356be`. AC-10..AC-16 are unchanged and stay contiguous. This finding rides with code-review FND-006, and neither blocks the other.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | No AC asserts line 91's no-section outcome: no `recordSources` entry and no `availability.recordKeys` entry for a frontmatter-owned key. AC-12 and AC-13 assert only the `record` value and "no diagnostic". So a fixture that follows quire-rs FR-081's absent-section rule and reports `not_applicable` would pass every FR-121 AC while violating line 91. Add "no `recordSources` or `availability.recordKeys` entry for the key" to AC-12 and AC-13, as AC-10 already says. | spec/functional/FR-121-record-key-sections-mapping.md:150-151 |
