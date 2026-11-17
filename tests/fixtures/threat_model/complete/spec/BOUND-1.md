---
id: BOUND-1
title: Trusted service boundary
type: trust_boundary
object: trust_boundary
trust_level: trusted
---
# Trusted service boundary

## Boundary

```mermaid
flowchart LR
  api[API service] --> db[(Tenant database)]
```

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| boundary_id | UUID | 1..1 | identity |
| trust_level | String | 1..1 | minLength: 1 |

## Invariants

The trusted service boundary limits access to the tenant database.

### TenantAccess

```ocl
context TrustedServiceBoundary
inv TenantAccess: true
```
