---
id: RISK-1
title: Accepted disclosure risk
type: risk
object: risk
decision: accepted
decided_by: A. Person
likelihood: possible
impact: major
relationships:
  - type: arises_from
    target: THREAT-1
---
# Accepted disclosure risk

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| risk_id | UUID | 1..1 | identity |
| likelihood | String | 1..1 | minLength: 1 |
| impact | String | 1..1 | minLength: 1 |
