---
id: SR-20261113071106
title: "EARS conformance review of quoin risk discrepancies (FR-154, FR-155)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #853 (git diff origin/main...HEAD at 3b00d92c): spec/functional/FR-154, spec/functional/FR-155"
---

# EARS conformance review of quoin risk discrepancies (FR-154, FR-155)

## Summary

Ticket: PLAT-1570. Each SHALL statement of FR-154 and FR-155 was checked for EARS form and for a
single testable obligation. `quire validate` over the two files reports no errors and four
warnings: quality:ambiguous-term `acceptable` at FR-155 lines 40-43. Every statement is a single
obligation, and the unwanted-behaviour (If/then) and event (When) forms are used correctly.

On the `acceptable` warnings: the phrase is not ambiguous in meaning, because FR-155:28-29 cites
FR-101's definition, so these are not correctness defects. But unlike FR-146, where the word sits
in a quoted output string, here it sits in the requirement conditions. FR-105 states the same
conditions without the term, using the field literal `outside_acceptable: true` / `false`. Using
that literal removes the warnings and the false-positive paragraph (FND-001).

Examined (role examined): FR-154 Behavior lines 43-56; FR-155 Behavior lines 39-66.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The conditions say "its region is outside/inside the acceptable region" where safety FR-105 says "its region has outside_acceptable: true/false". This trips quality:ambiguous-term four times and needs a lint-disclaimer paragraph. Use FR-105's field-literal wording and delete the disclaimer sentence. | FR-155:40-43, FR-155:31 |
| FND-002 | low | "With --strict" and "Without --strict" are not an EARS form. A flag is an optional feature: "Where --strict is given, the command SHALL ...". | FR-154:52-54 |

## Verdict

Mergeable on EARS grounds. Both findings are low wording fixes.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The fix made the exit-zero rule unconditional: "When the command renders, it SHALL exit zero." With --strict and findings the command renders, so it must exit zero and exit 1 at once. The old "Without --strict" condition is gone. Restore it: "Where --strict is not given, the command SHALL exit zero whenever it renders." | FR-154:63-64, FR-154-AC-5 |

## Dispositions

Round 1, against commit 678a797b.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
