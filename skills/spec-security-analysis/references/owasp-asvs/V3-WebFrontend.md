# V3 - Web Frontend Security

**Area**: Web Frontend Security  
**Focus**: Browser-facing security: cookies, headers, origins

## When to Apply

Apply when your component:
- Serves pages or scripts to browsers
- Sets cookies
- Handles cross-origin requests or redirects

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 3.1.1 | Web Frontend Security Documentation |  |  | ✓ |
| 3.2.1 | Unintended Content Interpretation | ✓ | ✓ | ✓ |
| 3.2.2 | Unintended Content Interpretation | ✓ | ✓ | ✓ |
| 3.2.3 | Unintended Content Interpretation |  |  | ✓ |
| 3.3.1 | Cookie Setup | ✓ | ✓ | ✓ |
| 3.3.2 | Cookie Setup |  | ✓ | ✓ |
| 3.3.3 | Cookie Setup |  | ✓ | ✓ |
| 3.3.4 | Cookie Setup |  | ✓ | ✓ |
| 3.3.5 | Cookie Setup |  |  | ✓ |
| 3.4.1 | Browser Security Mechanism Headers | ✓ | ✓ | ✓ |
| 3.4.2 | Browser Security Mechanism Headers | ✓ | ✓ | ✓ |
| 3.4.3 | Browser Security Mechanism Headers |  | ✓ | ✓ |
| 3.4.4 | Browser Security Mechanism Headers |  | ✓ | ✓ |
| 3.4.5 | Browser Security Mechanism Headers |  | ✓ | ✓ |
| 3.4.6 | Browser Security Mechanism Headers |  | ✓ | ✓ |
| 3.4.7 | Browser Security Mechanism Headers |  |  | ✓ |
| 3.4.8 | Browser Security Mechanism Headers |  |  | ✓ |
| 3.5.1 | Browser Origin Separation | ✓ | ✓ | ✓ |
| 3.5.2 | Browser Origin Separation | ✓ | ✓ | ✓ |
| 3.5.3 | Browser Origin Separation | ✓ | ✓ | ✓ |
| 3.5.4 | Browser Origin Separation |  | ✓ | ✓ |
| 3.5.5 | Browser Origin Separation |  | ✓ | ✓ |
| 3.5.6 | Browser Origin Separation |  |  | ✓ |
| 3.5.7 | Browser Origin Separation |  |  | ✓ |
| 3.5.8 | Browser Origin Separation |  |  | ✓ |
| 3.6.1 | External Resource Integrity |  |  | ✓ |
| 3.7.1 | Other Browser Security Considerations |  | ✓ | ✓ |
| 3.7.2 | Other Browser Security Considerations |  | ✓ | ✓ |
| 3.7.3 | Other Browser Security Considerations |  |  | ✓ |
| 3.7.4 | Other Browser Security Considerations |  |  | ✓ |
| 3.7.5 | Other Browser Security Considerations |  |  | ✓ |

## Mapping Guidance

```
NFR-XXX (Cookie Security) → 3.3.1, 3.3.3, 3.3.4
NFR-XXX (HSTS) → 3.4.1
NFR-XXX (CORS) → 3.4.2, 3.5.1, 3.5.2
NFR-XXX (CSP) → 3.4.3
NFR-XXX (Redirects) → 3.7.2
```
