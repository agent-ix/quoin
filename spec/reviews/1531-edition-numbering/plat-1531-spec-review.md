---
id: SR-20261113143522
title: "Spec review of the FR-171 renumbering amendment (PLAT-1531)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #873 (git diff origin/main...plat-1531-edition-numbering at 1c8cb730): spec/functional/FR-171-security-analysis-emits-typed-controls.md"
---

# Spec review of the FR-171 renumbering amendment (PLAT-1531)

## Summary

Ticket: PLAT-1531. The amendment changes three Behavior bullets and FR-171-AC-4 and FR-171-AC-6.
`quire validate --scope . spec/functional/FR-171-security-analysis-emits-typed-controls.md` exits 0
(quire 0.44.0). It prints one module warning (`semantic.unused-import` in spec-artifacts-app) that does not
come from this file.

The new chapter bullet and AC-4 agree. AC-4 allows exactly the ASVS chapter id and title replacement, and the
skill diff shows no other standard-name change. AC-7 is unaffected, because `SKILL.md` cites only Rev 5.2.0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-171-AC-6 counts a number as wrong when "the catalogue title of that control differs from what the row describes". Read literally, 19 NIST rows on the PR head fail it, the three re-pointed rows among them (RA-5(11) is still titled "Vulnerability Disclosure", catalogue "Public Disclosure Program"). Each names the right control by an older or shortened title, for example AU-2 "Audit Events" (catalogue "Event Logging"), also in the example; AU-12 "Audit Generation"; SC-17 "PKI Certificates". The PR passes them as wording differences. But "leave every other standard name as it is" and AC-4 may also forbid renaming those row titles. Two checkers would score AC-6 differently. Say whether the test is control identity (the same control under a renamed or shortened title passes) or exact title match, with AC-4 then allowing the title change. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:39-41,51,53 |

## Verdict

Testable apart from FND-001. Fixing FND-001 needs only a wording change to AC-6. If it becomes an exact-title
test, the 19 row titles need changing too.
