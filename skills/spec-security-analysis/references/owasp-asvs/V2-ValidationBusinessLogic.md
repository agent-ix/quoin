# V2 - Validation and Business Logic

**Area**: Validation and Business Logic  
**Focus**: Input validation and business logic

## When to Apply

**V2 is PRIMARY for input validation and business rules.** Apply when your component:
- Accepts external input
- Enforces multi-step workflows or limits
- Needs protection from automated abuse

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 2.1.1 | Validation and Business Logic Documentation | ✓ | ✓ | ✓ |
| 2.1.2 | Validation and Business Logic Documentation |  | ✓ | ✓ |
| 2.1.3 | Validation and Business Logic Documentation |  | ✓ | ✓ |
| 2.2.1 | Input Validation | ✓ | ✓ | ✓ |
| 2.2.2 | Input Validation | ✓ | ✓ | ✓ |
| 2.2.3 | Input Validation |  | ✓ | ✓ |
| 2.3.1 | Business Logic Security | ✓ | ✓ | ✓ |
| 2.3.2 | Business Logic Security |  | ✓ | ✓ |
| 2.3.3 | Business Logic Security |  | ✓ | ✓ |
| 2.3.4 | Business Logic Security |  | ✓ | ✓ |
| 2.3.5 | Business Logic Security |  |  | ✓ |
| 2.4.1 | Anti-automation |  | ✓ | ✓ |
| 2.4.2 | Anti-automation |  |  | ✓ |

## Mapping Guidance

```
FR-XXX (Input Validation) → 2.1.1, 2.2.1, 2.2.2
FR-XXX (Workflow) → 2.3.1
FR-XXX (Business Limits) → 2.1.3, 2.3.2
NFR-XXX (Transactions) → 2.3.3
NFR-XXX (Concurrency) → 2.3.4
NFR-XXX (Rate Limiting) → 2.4.1
```
