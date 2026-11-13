# V11 - Cryptography

**Area**: Cryptography  
**Focus**: Cryptography

## When to Apply

**V11 is PRIMARY for cryptography.** Apply when your component:
- Encrypts, hashes or signs data
- Manages keys
- Generates random values

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 11.1.1 | Cryptographic Inventory and Documentation |  | ✓ | ✓ |
| 11.1.2 | Cryptographic Inventory and Documentation |  | ✓ | ✓ |
| 11.1.3 | Cryptographic Inventory and Documentation |  |  | ✓ |
| 11.1.4 | Cryptographic Inventory and Documentation |  |  | ✓ |
| 11.2.1 | Secure Cryptography Implementation |  | ✓ | ✓ |
| 11.2.2 | Secure Cryptography Implementation |  | ✓ | ✓ |
| 11.2.3 | Secure Cryptography Implementation |  | ✓ | ✓ |
| 11.2.4 | Secure Cryptography Implementation |  |  | ✓ |
| 11.2.5 | Secure Cryptography Implementation |  |  | ✓ |
| 11.3.1 | Encryption Algorithms | ✓ | ✓ | ✓ |
| 11.3.2 | Encryption Algorithms | ✓ | ✓ | ✓ |
| 11.3.3 | Encryption Algorithms |  | ✓ | ✓ |
| 11.3.4 | Encryption Algorithms |  |  | ✓ |
| 11.3.5 | Encryption Algorithms |  |  | ✓ |
| 11.4.1 | Hashing and Hash-based Functions | ✓ | ✓ | ✓ |
| 11.4.2 | Hashing and Hash-based Functions |  | ✓ | ✓ |
| 11.4.3 | Hashing and Hash-based Functions |  | ✓ | ✓ |
| 11.4.4 | Hashing and Hash-based Functions |  | ✓ | ✓ |
| 11.5.1 | Random Values |  | ✓ | ✓ |
| 11.5.2 | Random Values |  |  | ✓ |
| 11.6.1 | Public Key Cryptography |  | ✓ | ✓ |
| 11.6.2 | Public Key Cryptography |  |  | ✓ |
| 11.7.1 | In-Use Data Cryptography |  |  | ✓ |
| 11.7.2 | In-Use Data Cryptography |  |  | ✓ |

## Mapping Guidance

```
FR-XXX (Encryption) → 11.3.1, 11.3.2, 11.3.3
FR-XXX (Key Management) → 11.1.1, 11.1.2
FR-XXX (Password Storage) → 11.4.2
FR-XXX (Secure Random) → 11.5.1
NFR-XXX (Algorithm Standards) → 11.2.1, 11.2.3, 11.4.1
NFR-XXX (Nonce Handling) → 11.3.4
```
