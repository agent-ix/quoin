---
id: SR-20261113095720
title: "Base spec review of quoin risk-summary (FR-169, FR-170)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #863 (git diff origin/main...HEAD at a1ece4e9): spec/functional/FR-169, spec/functional/FR-170, spec/functional/index.md, spec/spec.md"
---

# Base spec review of quoin risk-summary (FR-169, FR-170)

## Summary

Ticket: PLAT-1576. Checklist review of the read-only `quoin risk-summary` chain report against
spec-objects-safety FR-101..107, FR-201..205 and FR-301..305 on `origin/main`, and quoin FR-144..148,
FR-154, FR-155 and NFR-031 on `origin/main`.

The read-only, no-verdict contract matches safety FR-104 and FR-305: the command writes nothing,
stores no score, region, finding or verdict, defaults no decision, carries the FR-104 attribution,
and keeps the "accept" stem to the attribution, tokens and copied values. Because NFR-031's Scope
excludes `quoin risk-summary`, those rules are stated inside FR-169 itself, as they must be. No
paywalled standard text or standard name appears in the diff. No hash, digest, pin or tracking
record is introduced. FR-169 and FR-170 are unused on `origin/main`; open PR #862 adds FR-166
(its own clash with `origin/main`), not FR-169 or FR-170. `quire validate` over the two new files
reports no errors and no warnings.

All 32 acceptance criteria (FR-169-AC-1..19, FR-170-AC-1..13) are Test-verified and fixture-runnable;
the testability defects are listed below.

Applicable AssuranceProfiles: spec/assurance/AP-201-finding-quality.md and AP-202-jev-lens-evaluation.md
were inspected; neither scope covers a safety risk report, so neither applies and no enforced run is
required.

Examined (role examined): FR-169 statement, Behavior and FR-169-AC-1..19; FR-170 statement, Behavior,
token table and FR-170-AC-1..13. Context only: spec-objects-safety FR-101..107, FR-201..205,
FR-301..305; quoin FR-144..148, FR-154, FR-155, NFR-031.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The read list admits only FR, NFR, IT and TC documents that link to a control, but a control entry's verifications are "the documents with a verifies edge ... whatever their type", and FR-170 counts a verifies edge "from any document" (safety FR-302, FR-302-AC-4: a retained-evidence record of agent-ix/engineering-assurance). An implementation that follows the read list never sees an evidence record, so a verified control gets control-without-verification and an empty verifications list. Widen the read list to any document with a verifies edge to a control. | FR-169:73, FR-169:105, FR-170:61 |
| FND-002 | medium | Several error paths have no acceptance criterion: an unloadable file below scope is named on stderr while the command still renders (FR-169:81); a module that does not load, an unreadable (as opposed to missing) scope, and a benefit_risk that breaks its contract each exit 2 (FR-169:80). FR-169-AC-16 covers only a missing scope, a residual_risk, a policy and an undeclared type. Add criteria for each. | FR-169:80-81, FR-169-AC-16 |
| FND-003 | medium | Two rules give different Markdown for a null policy_region: "an absent value reads null" (FR-169:122) and "a missing region SHALL read the policy lists no region" (FR-169:124). The second is also false when the null comes from policy-not-found or policy-ambiguous (FR-146), where there is no single policy to list anything, and no criterion tests it (AC-13 checks only resolved regions and null). Keep one rendering, word it so it is true in every FR-146 case, and test it. | FR-169:122, FR-169:124, FR-169-AC-13 |
| FND-004 | low | The Markdown table puts lists and multi-key entries (estimate, control and residual-risk entries, finding objects) into single cells, but the cell rendering is not stated, so FR-169-AC-19's checked-in snapshot fixes whatever the first implementation prints and cannot fail against the spec. State how a list and an entry render in a cell. | FR-169:88, FR-169-AC-19 |
| FND-005 | low | The last clause of FR-170-AC-11 ("a hazard-level finding on neither") cannot fail: both hazards in the fixture have a situation, so hazard-without-situation never applies to them. Drop the clause or test the hazard-level placement on a row that could wrongly carry it. | FR-170-AC-11 |

## Verdict

Not mergeable as written: one high (FND-001, a wrong result for an evidence-verified control) and two
medium findings. All are spec-text fixes inside this PR.
