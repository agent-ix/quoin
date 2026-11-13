# SI - System and Information Integrity

**Family**: System and Information Integrity  
**Controls**: 23  
**Focus**: Integrity, validation, patching

## When to Apply

**SI is PRIMARY for input validation and patching.** Apply when your component:
- Accepts external input
- Requires patching/updates
- Validates data integrity
- Handles errors

## Controls

| Control | Name | Description | Software Relevance |
|---------|------|-------------|-------------------|
| SI-1 | Policy and Procedures | Document SI policies | LOW - Docs |
| SI-2 | Flaw Remediation | Remediate flaws | **HIGH** - Patching |
| SI-2(2) | Automated Patch Verification | Verify patches | **HIGH** - CI/CD |
| SI-3 | Malicious Code Protection | Protect against malware | MEDIUM - Scanning |
| SI-4 | System Monitoring | Monitor systems | **HIGH** - Observability |
| SI-5 | Security Alerts, Advisories, and Directives | Receive alerts | MEDIUM - Security feeds |
| SI-6 | Security Function Verification | Verify security | **HIGH** - Testing |
| SI-7 | Software, Firmware, and Information Integrity | Detect tampering | **HIGH** - Integrity |
| SI-10 | Information Input Validation | Validate input | **HIGH** - Validation |
| SI-11 | Error Handling | Handle errors securely | **HIGH** - Errors |
| SI-12 | Information Management and Retention | Manage retention | MEDIUM - Retention |
| SI-16 | Memory Protection | Protect memory | MEDIUM - Memory safety |
| SI-17 | Fail-Safe Procedures | Fail safely | **HIGH** - Fail-safe |

## Mapping Guidance

### Input/Output (SI-10, SI-11)
```
FR-XXX (Input Validation) → SI-10
NFR-XXX (Error Handling) → SI-11
```

### Integrity (SI-6, SI-7, SI-17)
```
NFR-XXX (Testing) → SI-6
NFR-XXX (Integrity) → SI-7
NFR-XXX (Fail-Safe) → SI-17
```

### Patching (SI-2)
```
NFR-XXX (Dependency Updates) → SI-2
NFR-XXX (Automated Patching) → SI-2(2)
```
