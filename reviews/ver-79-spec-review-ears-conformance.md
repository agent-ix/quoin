---
id: SR-20261113055213
title: "EARS conformance review of the quoin differential report v2 adapter (FR-150, NFR-032)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #848 (git diff origin/main...HEAD at dfd3b6b7): spec/functional/FR-150, spec/non-functional/NFR-032, spec/functional/FR-069 (edited lines)"
---

# EARS conformance review of the quoin differential report v2 adapter (FR-150, NFR-032)

## Summary

Ticket: VER-79. Checked each SHALL statement of FR-150 and NFR-032 for EARS form and for one
testable obligation. `quire validate` (quire 0.44.0) reports no warning on FR-150, NFR-032 or
TC-001. The FR-069 warnings it reports (lines 22, 43-47, 62, 78) are on lines this PR did not
change. FR-150's Behavior bullets are clean ubiquitous and unwanted-behaviour statements. Each
names the adapter or `quoin evidence record` as its subject.

Examined (role examined): the 22 FR-150 Behavior statements, FR-150-CON-1, FR-150-CON-2 and the
NFR-032 statement. Context only: the FR-069 Inputs edit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The NFR statement holds three obligations under one SHALL: bound work by input size, refuse input over 16 MiB unparsed, and stay deterministic. "Bound the work ... by the input's size" has no metric in the Measurement table. Split it into one statement per obligation, or drop the unmeasured clause. | spec/non-functional/NFR-032-differential-report-intake-is-bounded-and-deterministic.md:17-18 |
| FND-002 | low | One unwanted-behaviour statement carries two responses, "write no run" and "exit non-zero with an error naming the reason". Split it, or make them one conjunct that one test asserts. | spec/functional/FR-150-differential-report-v2-adapter.md:109-110 |

## Verdict

Mergeable on EARS grounds after two low wording fixes. The substantive defects are in the base
review (SR-20261113055212).
