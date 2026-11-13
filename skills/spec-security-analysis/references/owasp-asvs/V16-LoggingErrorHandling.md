# V16 - Security Logging and Error Handling

**Area**: Security Logging and Error Handling  
**Focus**: Security logging and error handling

## When to Apply

**V16 is PRIMARY for audit logging.** Apply when your component:
- Produces logs
- Has error conditions
- Makes security decisions
- Runs in production

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 16.1.1 | Security Logging Documentation |  | ✓ | ✓ |
| 16.2.1 | General Logging |  | ✓ | ✓ |
| 16.2.2 | General Logging |  | ✓ | ✓ |
| 16.2.3 | General Logging |  | ✓ | ✓ |
| 16.2.4 | General Logging |  | ✓ | ✓ |
| 16.2.5 | General Logging |  | ✓ | ✓ |
| 16.3.1 | Security Events |  | ✓ | ✓ |
| 16.3.2 | Security Events |  | ✓ | ✓ |
| 16.3.3 | Security Events |  | ✓ | ✓ |
| 16.3.4 | Security Events |  | ✓ | ✓ |
| 16.4.1 | Log Protection |  | ✓ | ✓ |
| 16.4.2 | Log Protection |  | ✓ | ✓ |
| 16.4.3 | Log Protection |  | ✓ | ✓ |
| 16.5.1 | Error Handling |  | ✓ | ✓ |
| 16.5.2 | Error Handling |  | ✓ | ✓ |
| 16.5.3 | Error Handling |  | ✓ | ✓ |
| 16.5.4 | Error Handling |  |  | ✓ |

## Mapping Guidance

```
FR-013 (Audit Logging) → 16.2.1, 16.3.1, 16.3.2, 16.3.3
FR-013-CON-2 (No Leakage) → 16.2.5
NFR-XXX (Timestamps) → 16.2.2
NFR-XXX (Log Integrity) → 16.4.2
NFR-XXX (Log Injection) → 16.4.1
NFR-XXX (Log Transport) → 16.4.3
NFR-XXX (Error Handling) → 16.5.1, 16.5.3, 16.5.4
```
