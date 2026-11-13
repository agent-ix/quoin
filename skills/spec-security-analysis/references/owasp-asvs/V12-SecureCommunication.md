# V12 - Secure Communication

**Area**: Secure Communication  
**Focus**: TLS and service-to-service communication

## When to Apply

Apply when your component:
- Communicates over a network
- Calls or hosts internal services

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 12.1.1 | General TLS Security Guidance | ✓ | ✓ | ✓ |
| 12.1.2 | General TLS Security Guidance |  | ✓ | ✓ |
| 12.1.3 | General TLS Security Guidance |  | ✓ | ✓ |
| 12.1.4 | General TLS Security Guidance |  |  | ✓ |
| 12.1.5 | General TLS Security Guidance |  |  | ✓ |
| 12.2.1 | HTTPS Communication with External Facing Services | ✓ | ✓ | ✓ |
| 12.2.2 | HTTPS Communication with External Facing Services | ✓ | ✓ | ✓ |
| 12.3.1 | General Service to Service Communication Security |  | ✓ | ✓ |
| 12.3.2 | General Service to Service Communication Security |  | ✓ | ✓ |
| 12.3.3 | General Service to Service Communication Security |  | ✓ | ✓ |
| 12.3.4 | General Service to Service Communication Security |  | ✓ | ✓ |
| 12.3.5 | General Service to Service Communication Security |  |  | ✓ |

## Mapping Guidance

```
FR-XXX (TLS) → 12.1.1, 12.2.1
FR-XXX (Certificate Validation) → 12.3.2
NFR-XXX (Internal TLS) → 12.3.1, 12.3.3, 12.3.4
NFR-XXX (Public Certificates) → 12.2.2
```
