---
id: SR-20261117175244
title: "VER-76 failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-209-agent-design-gate.md"
review_set: subset
---

## Summary

The FR explicitly distinguishes evaluator unavailability from design denial, protects human acceptance, and treats the proposer as untrusted. The external scripted proposer's own launch and response failures are not assigned a terminal or recoverable outcome.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Define the failure behavior when the scripted design agent cannot launch or returns an unusable/no revision. The current contract covers evaluator failures but not proposer failures, so the workflow's termination and human recovery path are undefined. | FR-209 Inputs; FR-209 Behavior: Loop and human acceptance |

## Dispositions

Round 1, fix commit 1fe35c20.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Agent launch/no-usable-edit failure now stops in `proposing` or `revising` with `design_agent_unavailable`; an operator may resume or end the run, without treating it as a denial or consuming a retry. |
