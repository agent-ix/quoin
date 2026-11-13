---
id: SR-20261113095723
title: "EARS conformance review of quoin risk-summary (FR-169, FR-170)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #863 (git diff origin/main...HEAD at a1ece4e9): spec/functional/FR-169, spec/functional/FR-170"
---

# EARS conformance review of quoin risk-summary (FR-169, FR-170)

## Summary

Ticket: PLAT-1576. Each Behavior statement of FR-169 and FR-170 was checked for EARS form and for one
testable obligation. `quire validate` over the two files reports no errors and no warnings. The event
(When), unwanted-behaviour (If/then) and state forms are used correctly for the command surface, the
exit codes and the multi-estimate rule.

Examined (role examined): every Behavior bullet of FR-169 (Surface and reading, Exit codes and
diagnostics, Output contract, Ordering, No verdict) and of FR-170.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "A control is verified, and implemented, exactly as FR-147 defines ... The command tests that the edge exists and never reads whether a verification passed" carries no SHALL, so the prohibition on reading a verification outcome binds nothing, and no FR-170 criterion tests it (FR-147-AC-3 has the equivalent). Write it as a SHALL NOT and add the failing-verification criterion. | FR-170:61 |
| FND-002 | low | Obligations hide in non-SHALL sentences inside SHALL bullets: "--format defaults to markdown" (FR-169:71), "In Markdown an absent value reads null" (FR-169:122), and the definitions of requirements and verifications (FR-169:105). Split each into its own SHALL statement so each has one obligation. | FR-169:71, FR-169:105, FR-169:122 |

## Verdict

Mergeable on this lens once the two low findings are fixed.

## Dispositions

Round 1, against commit 68717d15.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
