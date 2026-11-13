---
id: SR-20261113202802
title: "Spec review of quoin PR 879 changes to FR-171 and FR-172"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0581b950; spec/functional/FR-171-security-analysis-emits-typed-controls.md, spec/functional/FR-172-clause-set-sources-and-licence-boundary.md"
review_set: subset
---

# Spec review of quoin PR 879 changes to FR-171 and FR-172

## Summary

Ticket: PLAT-1530. Base spec review with EARS and integrity checks over the changed lines only:
two new FR-172 Behavior statements (lines 39-40), FR-172-AC-7 and AC-8, and the FR-171 statement
(line 42) and FR-171-AC-4 edits. Both files pass `quire validate`. Tools: quoin 0.30.1, quire
0.44.0 (engine 0.59.1).

Examined and clean: FR-172-AC-8 is atomic, observable and failable (the deleted directories and
links would fail it on the base tree). FR-172 line 40 is a well-formed event-driven statement.
FR-171 edit (question raised by the lead): "among those the skill keeps" does not weaken a live
requirement. The original clause protected unverified SOC 2 and ISO numbers from being "corrected"
without a free source; with the tables deleted no such number remains, so nothing it protected is
lost. The clause is now empty (FND-003).

## Verdict

Acceptable with one medium (FND-001) and three low findings. No requirement contradicts another;
FR-171 is not weakened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-172-AC-7 cannot fail for the defect it targets. The sentinel is a value the test invents, so no shipped file can carry it unless someone pastes it there; the pre-PR tree with every restricted reference table present satisfies AC-7. Installing the pack in a temp `IX_HOME` is not causally connected to the files the criterion inspects. Restate it as an observable absence of restricted content (for example, no SOC 2 or ISO/IEC 27001 control identifier under `skills/spec-security-analysis` or `tests/fixtures/security-analysis`) or delete it in favour of AC-8 | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:54 |
| FND-002 | medium | The new statements at lines 39-40 do not say whether the skill may read the locally installed pack's licensed clause text to judge which clause applies, nor how a pack's set key is recognised as "the" set of SOC 2 or ISO/IEC 27001. With ids only and no text or title, the skill has nothing to decide applicability from, so ticket AC 1 ("maps controls to its clause ids") is underdetermined; two implementers would read it differently. State that pack text may be read locally for applicability but never written, and how a restricted source is matched to an installed set | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:39-40 |
| FND-003 | low | FR-171 line 42 and FR-171-AC-4 now constrain "SOC 2 and ISO/IEC 27001 numbers the skill keeps", an empty set after this PR, and line 41 / AC-4 ("no changed standard name ... in the references") were left as is although this PR removes SOC 2 and ISO rows from the references. Either these FR-171 statements are scoped to the PLAT-1426 change (then the edit was unnecessary) or they are standing (then the standard-name clause is now violated). Delete the SOC 2/ISO clause from the statement and AC-4 rather than qualify it, and scope the statements explicitly to the PLAT-1426 change | spec/functional/FR-171-security-analysis-emits-typed-controls.md:41-42 |
| FND-004 | low | Line 39 is compound: two obligations ("SHALL NOT ship ... and SHALL NOT read one") in one statement, and "read one" has no AC (AC-8 checks shipping and linking only). Split it; the read half is covered by the existing no-text rule or by AC-8's link check | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:39 |
