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
