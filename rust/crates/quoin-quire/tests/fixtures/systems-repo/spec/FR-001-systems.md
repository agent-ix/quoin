---
id: FR-001
type: FR
title: A document that declares systems tables
---

## Parts

| Owner | Declared Type | Multiplicity |
|-------|---------------|--------------|
| PART-A | Engine | 1 |

## Ports

| Owner | Direction | Interface | Multiplicity |
|-------|-----------|-----------|--------------|
| PART-A | out | Telemetry | 1 |

## Connections

| Source | Source Multiplicity | Target | Target Multiplicity | Direction |
|--------|---------------------|--------|---------------------|-----------|
| PART-A.out | 1 | PART-B.in | 0..* | forward |

## Allocations

| Source | Target |
|--------|--------|
| FR-001 | PART-A |
