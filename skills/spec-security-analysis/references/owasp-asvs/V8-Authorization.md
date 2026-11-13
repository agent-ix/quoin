# V8 - Authorization

**Area**: Authorization  
**Focus**: Authorization and privilege management

## When to Apply

**V8 is PRIMARY for authorization.** Apply when your component:
- Controls access to resources
- Implements RBAC or ABAC
- Has admin functionality
- Serves multiple tenants

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 8.1.1 | Authorization Documentation | ✓ | ✓ | ✓ |
| 8.1.2 | Authorization Documentation |  | ✓ | ✓ |
| 8.1.3 | Authorization Documentation |  |  | ✓ |
| 8.1.4 | Authorization Documentation |  |  | ✓ |
| 8.2.1 | General Authorization Design | ✓ | ✓ | ✓ |
| 8.2.2 | General Authorization Design | ✓ | ✓ | ✓ |
| 8.2.3 | General Authorization Design |  | ✓ | ✓ |
| 8.2.4 | General Authorization Design |  |  | ✓ |
| 8.3.1 | Operation Level Authorization | ✓ | ✓ | ✓ |
| 8.3.2 | Operation Level Authorization |  |  | ✓ |
| 8.3.3 | Operation Level Authorization |  |  | ✓ |
| 8.4.1 | Other Authorization Considerations |  | ✓ | ✓ |
| 8.4.2 | Other Authorization Considerations |  |  | ✓ |

## Mapping Guidance

```
FR-001 (Permission Check) → 8.2.1, 8.2.2, 8.3.1
FR-002 (Enforcement) → 8.3.1
FR-005 (Resource-Level) → 8.2.2, 8.2.3
FR-007 (Org Tenancy) → 8.4.1
FR-008 (RBAC) → 8.1.1, 8.2.1
FR-009 (ABAC) → 8.1.3, 8.2.4
FR-010 (Admin) → 8.4.2
NFR-002 (Fail-Safe) → 8.2.1, 8.3.1
```
