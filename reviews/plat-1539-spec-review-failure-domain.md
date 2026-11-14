---
id: SR-20261114002409
title: "Failure-domain review of PLAT-1539 quoin#893"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-171-security-analysis-emits-typed-controls.md"
review_set: subset
---

## Summary

The new behavior covers unresolved and unavailable evaluation results, but it does not say what the skill must do if profile evaluation fails or emits no valid evaluation document. The applicability boundary should fail closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-171 does not prohibit mapping or artifact emission when profile evaluation exits nonzero or its output is missing or invalid. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:46-48 |

## Scope

- FR-171 Behavior lines 46-58 and AC-8 through AC-14 examined for input, execution, output, and failure paths.
