---
id: SR-20261113080831
title: "EARS conformance review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #855 (git diff origin/main...015a0665): spec/functional/FR-160-per-class-evidence-floors.md, spec/functional/FR-161-per-class-obligations-report.md, spec/non-functional/NFR-034-class-reports-state-and-never-assign.md"
---

# EARS conformance review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)

## Summary

Ticket: PLAT-1526. This review checks the SHALL statements in the NFR-034 Statement and in the FR-160 and FR-161
constraints. The constraints are well-formed ubiquitous statements. `quire validate` raises `ears:non-singular` on
NFR-034 line 19.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The first sentence of the NFR-034 Statement joins two SHALLs: "SHALL read ... and SHALL NOT assign, default, infer, raise or write". `quire validate` reports `ears:non-singular`. Split it into one statement per SHALL. | spec/non-functional/NFR-034-class-reports-state-and-never-assign.md:19 |
| FND-002 | low | FR-160 and FR-161 have no principal SHALL statement. Their core behaviour, what the audit applies and what the command reports, is descriptive prose, and the only SHALLs are prohibitions in the constraints. Add one ubiquitous statement each. For example: "`quoin evidence audit` SHALL apply the class floors of `--class-policy` to each classified obligation". | FR-160; FR-161 |

## Verdict

Minor wording only; no blocking statement-grammar defect.
