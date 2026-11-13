# V1 - Encoding and Sanitization

**Area**: Encoding and Sanitization  
**Focus**: Encoding, injection prevention, sanitization, safe parsing

## When to Apply

**V1 is PRIMARY for injection defence.** Apply when your component:
- Builds queries, commands, URLs, HTML or scripts from untrusted data
- Parses XML, JSON or serialized objects
- Handles memory or strings in unmanaged code

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 1.1.1 | Encoding and Sanitization Architecture |  | ✓ | ✓ |
| 1.1.2 | Encoding and Sanitization Architecture |  | ✓ | ✓ |
| 1.2.1 | Injection Prevention | ✓ | ✓ | ✓ |
| 1.2.2 | Injection Prevention | ✓ | ✓ | ✓ |
| 1.2.3 | Injection Prevention | ✓ | ✓ | ✓ |
| 1.2.4 | Injection Prevention | ✓ | ✓ | ✓ |
| 1.2.5 | Injection Prevention | ✓ | ✓ | ✓ |
| 1.2.6 | Injection Prevention |  | ✓ | ✓ |
| 1.2.7 | Injection Prevention |  | ✓ | ✓ |
| 1.2.8 | Injection Prevention |  | ✓ | ✓ |
| 1.2.9 | Injection Prevention |  | ✓ | ✓ |
| 1.2.10 | Injection Prevention |  |  | ✓ |
| 1.3.1 | Sanitization | ✓ | ✓ | ✓ |
| 1.3.2 | Sanitization | ✓ | ✓ | ✓ |
| 1.3.3 | Sanitization |  | ✓ | ✓ |
| 1.3.4 | Sanitization |  | ✓ | ✓ |
| 1.3.5 | Sanitization |  | ✓ | ✓ |
| 1.3.6 | Sanitization |  | ✓ | ✓ |
| 1.3.7 | Sanitization |  | ✓ | ✓ |
| 1.3.8 | Sanitization |  | ✓ | ✓ |
| 1.3.9 | Sanitization |  | ✓ | ✓ |
| 1.3.10 | Sanitization |  | ✓ | ✓ |
| 1.3.11 | Sanitization |  | ✓ | ✓ |
| 1.3.12 | Sanitization |  |  | ✓ |
| 1.4.1 | Memory, String, and Unmanaged Code |  | ✓ | ✓ |
| 1.4.2 | Memory, String, and Unmanaged Code |  | ✓ | ✓ |
| 1.4.3 | Memory, String, and Unmanaged Code |  | ✓ | ✓ |
| 1.5.1 | Safe Deserialization | ✓ | ✓ | ✓ |
| 1.5.2 | Safe Deserialization |  | ✓ | ✓ |
| 1.5.3 | Safe Deserialization |  |  | ✓ |

## Mapping Guidance

```
NFR-XXX (Output Encoding) → 1.2.1, 1.2.2, 1.2.3
NFR-XXX (Injection Prevention) → 1.2.4, 1.2.5
NFR-XXX (HTML Sanitization) → 1.3.1
NFR-XXX (SSRF Protection) → 1.3.6
NFR-XXX (XML Parsing) → 1.5.1
NFR-XXX (Deserialization) → 1.5.2
NFR-XXX (Memory Safety) → 1.4.1
```
