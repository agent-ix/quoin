---
id: SR-17102
title: "EARS conformance review of the PLAT-1853 customer-artifact retained kind"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md, spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md (PR #1045, git diff origin/main...e0476e21)"
review_set: subset
---

# EARS conformance review of the PLAT-1853 customer-artifact retained kind

## Summary

Ticket: PLAT-1853. I checked the requirement statements this PR adds for EARS
form, for one SHALL per bullet, and for pronoun references. The trigger bullet
(`When a person runs ... the store SHALL retain ...`), CON-4 and CON-5 conform.
I found two low-severity wording defects.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Normative FR-030 Customer artifacts bullets lack a SHALL or carry two requirements. The `artifactId` bound with its `case-collision` and `invalid-identifier` refusals, and the append-only bullet, state obligations with no SHALL. The last bullet joins "SHALL validate ... before writing" with "a stored file ... is an error" in one sentence. The `gc` bullet joins a SHALL NOT with the kind definition. | spec/functional/FR-030-evidence-store.md:118-132 | wrong-requirement |
| FND-002 | low | The new text uses "it" for a named subject. In FR-030: "lowercases it in the path" and "who supplied it". In FR-211: "however the customer supplies it" and "digest of it". In FR-133: "any digest of it". Each should name `artifactId` or "the customer artifact". | spec/functional/FR-030-evidence-store.md:113,119; spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md:88,138; spec/functional/FR-133-discharge-facts-in-the-evidence-store.md:62 | wrong-requirement |

## Verdict

**Acceptable with low findings.** Both are wording only, and the meaning is
recoverable from context.

## Examined

The FR-030 Customer artifacts bullets (lines 105-132), CON-4, CON-5 and
AC-36 to AC-41. The FR-133 new Behavior bullet and AC-8. The FR-211 new
Evidence references and Customer-owned clauses bullets, and AC-25 to AC-30.
