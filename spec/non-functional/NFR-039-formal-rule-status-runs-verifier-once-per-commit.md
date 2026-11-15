---
id: NFR-039
title: "Formal-rule status runs the verifier once per candidate commit"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: "constrains"
---

# NFR-039: Formal-rule status runs the verifier once per candidate commit

## Statement

`quoin formal-rules status` SHALL run engineering-assurance `signature-verify`
once per candidate commit, not once per rule.

## Scope

- Applies to: `quoin formal-rules status` ([FR-188](../functional/FR-188-formal-rule-approval-state-from-signed-commits.md)).

## Rationale

One approval commit may name many rules; verifying per rule would repeat the
same signature check.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| `signature-verify` invocations for N candidate commits naming M rules | N | N | Test |

## Verification

A fixture with one signed commit carrying several `Formal-rule:` trailers is
run against a counting stand-in for the verifier; the count equals the number
of candidate commits.
