---
id: SR-20261113143523
title: "Gap analysis of FR-171 renumbering criteria against the PLAT-1531 skill change"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quoin; PR #873 (git diff origin/main...plat-1531-edition-numbering at 1c8cb730): FR-171-AC-4, FR-171-AC-6, FR-171-AC-7 against skills/spec-security-analysis"
---

# Gap analysis of FR-171 renumbering criteria against the PLAT-1531 skill change

## Summary

Ticket: PLAT-1531. Plan completion: not assessed. The PR changes skill content and a spec, not code or tests.
The criteria it bears on use Inspection or Analysis, so no tagged test is expected.

- FR-171-AC-4 (Inspection): the skill diff changes ASVS chapter ids and titles, ASVS requirement ids, three
  NIST numbers and the example's NIST revision label. It changes no other standard name and no SOC 2 or ISO
  number. Holds.
- FR-171-AC-6 (Analysis): re-run independently against the OSCAL Rev 5.2.0 catalogue and the ASVS 5.0.0 CSV.
  No cited id is missing or withdrawn. The three NIST numbers whose catalogue control differed from the row
  are corrected and listed in the PR. Holds under the control-identity reading. The title-wording ambiguity
  is recorded once, as FND-001 of SR-20261113143522, and not repeated here.
- FR-171-AC-7 (Inspection): `SKILL.md` names only Rev 5.2.0. Holds.
- FR-171-AC-1, AC-2, AC-3 and AC-5 (typed control emission) belong to PLAT-1427. This PR does not change
  them. Context only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

No gap unique to this method.
