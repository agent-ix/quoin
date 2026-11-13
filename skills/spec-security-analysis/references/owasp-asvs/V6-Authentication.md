# V6 - Authentication

**Area**: Authentication  
**Focus**: Authentication

## When to Apply

**V6 is PRIMARY for authentication.** Apply when your component:
- Authenticates users or services
- Stores or verifies credentials
- Offers multi-factor or federated login

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 6.1.1 | Authentication Documentation | ✓ | ✓ | ✓ |
| 6.1.2 | Authentication Documentation |  | ✓ | ✓ |
| 6.1.3 | Authentication Documentation |  | ✓ | ✓ |
| 6.2.1 | Password Security | ✓ | ✓ | ✓ |
| 6.2.2 | Password Security | ✓ | ✓ | ✓ |
| 6.2.3 | Password Security | ✓ | ✓ | ✓ |
| 6.2.4 | Password Security | ✓ | ✓ | ✓ |
| 6.2.5 | Password Security | ✓ | ✓ | ✓ |
| 6.2.6 | Password Security | ✓ | ✓ | ✓ |
| 6.2.7 | Password Security | ✓ | ✓ | ✓ |
| 6.2.8 | Password Security | ✓ | ✓ | ✓ |
| 6.2.9 | Password Security |  | ✓ | ✓ |
| 6.2.10 | Password Security |  | ✓ | ✓ |
| 6.2.11 | Password Security |  | ✓ | ✓ |
| 6.2.12 | Password Security |  | ✓ | ✓ |
| 6.3.1 | General Authentication Security | ✓ | ✓ | ✓ |
| 6.3.2 | General Authentication Security | ✓ | ✓ | ✓ |
| 6.3.3 | General Authentication Security |  | ✓ | ✓ |
| 6.3.4 | General Authentication Security |  | ✓ | ✓ |
| 6.3.5 | General Authentication Security |  |  | ✓ |
| 6.3.6 | General Authentication Security |  |  | ✓ |
| 6.3.7 | General Authentication Security |  |  | ✓ |
| 6.3.8 | General Authentication Security |  |  | ✓ |
| 6.4.1 | Authentication Factor Lifecycle and Recovery | ✓ | ✓ | ✓ |
| 6.4.2 | Authentication Factor Lifecycle and Recovery | ✓ | ✓ | ✓ |
| 6.4.3 | Authentication Factor Lifecycle and Recovery |  | ✓ | ✓ |
| 6.4.4 | Authentication Factor Lifecycle and Recovery |  | ✓ | ✓ |
| 6.4.5 | Authentication Factor Lifecycle and Recovery |  |  | ✓ |
| 6.4.6 | Authentication Factor Lifecycle and Recovery |  |  | ✓ |
| 6.5.1 | General Multi-factor authentication requirements |  | ✓ | ✓ |
| 6.5.2 | General Multi-factor authentication requirements |  | ✓ | ✓ |
| 6.5.3 | General Multi-factor authentication requirements |  | ✓ | ✓ |
| 6.5.4 | General Multi-factor authentication requirements |  | ✓ | ✓ |
| 6.5.5 | General Multi-factor authentication requirements |  | ✓ | ✓ |
| 6.5.6 | General Multi-factor authentication requirements |  |  | ✓ |
| 6.5.7 | General Multi-factor authentication requirements |  |  | ✓ |
| 6.5.8 | General Multi-factor authentication requirements |  |  | ✓ |
| 6.6.1 | Out-of-Band authentication mechanisms |  | ✓ | ✓ |
| 6.6.2 | Out-of-Band authentication mechanisms |  | ✓ | ✓ |
| 6.6.3 | Out-of-Band authentication mechanisms |  | ✓ | ✓ |
| 6.6.4 | Out-of-Band authentication mechanisms |  |  | ✓ |
| 6.7.1 | Cryptographic authentication mechanism |  |  | ✓ |
| 6.7.2 | Cryptographic authentication mechanism |  |  | ✓ |
| 6.8.1 | Authentication with an Identity Provider |  | ✓ | ✓ |
| 6.8.2 | Authentication with an Identity Provider |  | ✓ | ✓ |
| 6.8.3 | Authentication with an Identity Provider |  | ✓ | ✓ |
| 6.8.4 | Authentication with an Identity Provider |  | ✓ | ✓ |

## Mapping Guidance

```
FR-XXX (Login) → 6.2.1, 6.2.5, 6.3.1
FR-XXX (Password Change) → 6.2.2, 6.2.3
FR-XXX (Breached Passwords) → 6.2.12
FR-XXX (MFA) → 6.3.3, 6.5.1
FR-XXX (Password Reset) → 6.4.3
NFR-XXX (Rate Limiting) → 6.3.1
NFR-XXX (One-time Code Lifetime) → 6.5.5
NFR-XXX (Federated Login) → 6.8.1, 6.8.2
```
