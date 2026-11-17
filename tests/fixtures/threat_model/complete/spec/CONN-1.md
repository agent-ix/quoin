---
id: CONN-1
title: API connection
type: connection
object: connection
relationships:
  - type: carries
    target: CLASS-1
  - type: crosses
    target: BOUND-1
  - type: delivers_to
    target: SINK-1
---
# API connection

## Connection

| Source | Source Multiplicity | Target | Target Multiplicity | Direction |
|---|---|---|---|---|
| api_out | 1..1 | database_in | 1..1 | source-to-target |
