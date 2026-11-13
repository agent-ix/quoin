# V15 - Secure Coding and Architecture

**Area**: Secure Coding and Architecture  
**Focus**: Secure coding, architecture and dependencies

## When to Apply

Apply when your component:
- Uses third-party libraries
- Runs concurrent code
- Maps request data onto objects

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 15.1.1 | Secure Coding and Architecture Documentation | ✓ | ✓ | ✓ |
| 15.1.2 | Secure Coding and Architecture Documentation |  | ✓ | ✓ |
| 15.1.3 | Secure Coding and Architecture Documentation |  | ✓ | ✓ |
| 15.1.4 | Secure Coding and Architecture Documentation |  |  | ✓ |
| 15.1.5 | Secure Coding and Architecture Documentation |  |  | ✓ |
| 15.2.1 | Security Architecture and Dependencies | ✓ | ✓ | ✓ |
| 15.2.2 | Security Architecture and Dependencies |  | ✓ | ✓ |
| 15.2.3 | Security Architecture and Dependencies |  | ✓ | ✓ |
| 15.2.4 | Security Architecture and Dependencies |  |  | ✓ |
| 15.2.5 | Security Architecture and Dependencies |  |  | ✓ |
| 15.3.1 | Defensive Coding | ✓ | ✓ | ✓ |
| 15.3.2 | Defensive Coding |  | ✓ | ✓ |
| 15.3.3 | Defensive Coding |  | ✓ | ✓ |
| 15.3.4 | Defensive Coding |  | ✓ | ✓ |
| 15.3.5 | Defensive Coding |  | ✓ | ✓ |
| 15.3.6 | Defensive Coding |  | ✓ | ✓ |
| 15.3.7 | Defensive Coding |  | ✓ | ✓ |
| 15.4.1 | Safe Concurrency |  |  | ✓ |
| 15.4.2 | Safe Concurrency |  |  | ✓ |
| 15.4.3 | Safe Concurrency |  |  | ✓ |
| 15.4.4 | Safe Concurrency |  |  | ✓ |

## Mapping Guidance

```
NFR-XXX (Dependency Inventory) → 15.1.2
NFR-XXX (Dependency Remediation) → 15.1.1, 15.2.1
NFR-XXX (Dependency Source) → 15.2.4
NFR-XXX (Mass Assignment) → 15.3.3
NFR-XXX (Concurrency) → 15.4.1, 15.4.2
```
