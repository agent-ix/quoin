# V7 - Session Management

**Area**: Session Management  
**Focus**: Session management

## When to Apply

Apply when your component:
- Creates and tracks user sessions
- Issues session tokens

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 7.1.1 | Session Management Documentation |  | ✓ | ✓ |
| 7.1.2 | Session Management Documentation |  | ✓ | ✓ |
| 7.1.3 | Session Management Documentation |  | ✓ | ✓ |
| 7.2.1 | Fundamental Session Management Security | ✓ | ✓ | ✓ |
| 7.2.2 | Fundamental Session Management Security | ✓ | ✓ | ✓ |
| 7.2.3 | Fundamental Session Management Security | ✓ | ✓ | ✓ |
| 7.2.4 | Fundamental Session Management Security | ✓ | ✓ | ✓ |
| 7.3.1 | Session Timeout |  | ✓ | ✓ |
| 7.3.2 | Session Timeout |  | ✓ | ✓ |
| 7.4.1 | Session Termination | ✓ | ✓ | ✓ |
| 7.4.2 | Session Termination | ✓ | ✓ | ✓ |
| 7.4.3 | Session Termination |  | ✓ | ✓ |
| 7.4.4 | Session Termination |  | ✓ | ✓ |
| 7.4.5 | Session Termination |  | ✓ | ✓ |
| 7.5.1 | Defenses Against Session Abuse |  | ✓ | ✓ |
| 7.5.2 | Defenses Against Session Abuse |  | ✓ | ✓ |
| 7.5.3 | Defenses Against Session Abuse |  |  | ✓ |
| 7.6.1 | Federated Re-authentication |  | ✓ | ✓ |
| 7.6.2 | Federated Re-authentication |  | ✓ | ✓ |

## Mapping Guidance

```
FR-XXX (Session Creation) → 7.2.1, 7.2.2, 7.2.4
FR-XXX (Logout) → 7.4.1
FR-XXX (Timeout) → 7.3.1, 7.3.2
NFR-XXX (Token Randomness) → 7.2.3
NFR-XXX (Admin Termination) → 7.4.5
```
