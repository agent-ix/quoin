---
id: SR-20261113140011
title: "EARS conformance review of source profile selection (FR-180, FR-181, FR-182, NFR-037)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #870 at 9e8997c3: Behavior and Statement lines of spec/functional/FR-180, FR-181, FR-182, spec/non-functional/NFR-037"
---

# EARS conformance review of source profile selection (FR-180, FR-181, FR-182, NFR-037)

## Summary

Ticket: PLAT-1534. Read every SHALL statement in the four new artifacts. `quire validate` reports no grammar
warning on any of them. That includes the defect below: a statement with no response verb passes the grammar check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-181 Behavior line "Quoin SHALL the module, the selection file and every requirement, writing to the console and `--out-dir` only." has no response verb. Its obligation cannot be read. The intent is probably "SHALL NOT modify", and no AC covers it. Restate it, then add an AC that hashes the modules, the selection files and the spec before and after a run. | spec/functional/FR-181-source-profile-evaluation.md:54 |
| FND-002 | low | "Quoin SHALL use no profile when the selection file is absent" puts the trigger after the response. EARS writes it "When the selection file is absent, Quoin SHALL ...". | spec/functional/FR-180-source-profile-selection.md:48 |

## Verdict

One unreadable statement (FND-001) must be fixed. The rest are singular, use an explicit subject, and can be tested.

Examined (role examined): all Behavior lines of FR-180 (7), FR-181 (7), FR-182 (5) and both NFR-037 Statement
sentences.
