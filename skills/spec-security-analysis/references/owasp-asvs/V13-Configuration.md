# V13 - Configuration

**Area**: Configuration  
**Focus**: Configuration, secrets and information leakage

## When to Apply

**V13 applies to all software.** Apply when your component:
- Is deployed to production
- Holds secrets or calls backend services

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 13.1.1 | Configuration Documentation |  | ✓ | ✓ |
| 13.1.2 | Configuration Documentation |  |  | ✓ |
| 13.1.3 | Configuration Documentation |  |  | ✓ |
| 13.1.4 | Configuration Documentation |  |  | ✓ |
| 13.2.1 | Backend Communication Configuration |  | ✓ | ✓ |
| 13.2.2 | Backend Communication Configuration |  | ✓ | ✓ |
| 13.2.3 | Backend Communication Configuration |  | ✓ | ✓ |
| 13.2.4 | Backend Communication Configuration |  | ✓ | ✓ |
| 13.2.5 | Backend Communication Configuration |  | ✓ | ✓ |
| 13.2.6 | Backend Communication Configuration |  |  | ✓ |
| 13.3.1 | Secret Management |  | ✓ | ✓ |
| 13.3.2 | Secret Management |  | ✓ | ✓ |
| 13.3.3 | Secret Management |  |  | ✓ |
| 13.3.4 | Secret Management |  |  | ✓ |
| 13.4.1 | Unintended Information Leakage | ✓ | ✓ | ✓ |
| 13.4.2 | Unintended Information Leakage |  | ✓ | ✓ |
| 13.4.3 | Unintended Information Leakage |  | ✓ | ✓ |
| 13.4.4 | Unintended Information Leakage |  | ✓ | ✓ |
| 13.4.5 | Unintended Information Leakage |  | ✓ | ✓ |
| 13.4.6 | Unintended Information Leakage |  |  | ✓ |
| 13.4.7 | Unintended Information Leakage |  |  | ✓ |

## Mapping Guidance

```
NFR-XXX (Secrets Management) → 13.3.1, 13.3.2, 13.3.4
NFR-XXX (Backend Communication) → 13.2.1, 13.2.2, 13.2.3
NFR-XXX (Debug Disabled) → 13.4.2
NFR-XXX (Source Metadata Removed) → 13.4.1
NFR-XXX (Outbound Allowlist) → 13.2.4
```
