---
id: THREAT-1
title: Data disclosure
type: threat
object: threat
stride_category: information_disclosure
vector: Disclosure through an exposed API request
relationships:
  - type: threatens
    target: CONN-1
  - type: threatens
    target: SINK-1
---
# Data disclosure

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| threat_id | UUID | 1..1 | identity |
| stride_category | String | 1..1 | minLength: 1 |
| vector | String | 1..1 | minLength: 1 |
