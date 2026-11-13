# Component Type to Security Standards Mapping

Use this reference to identify which security controls are most relevant based on component type.

> [!IMPORTANT]
> ALL standards apply to ALL components. This guide identifies PRIMARY focus areas - you must still evaluate every standard for comprehensive coverage.

---

## Authorization & Access Control Components

**Examples**: py-permissions, py-auth, auth-service

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC6.1-CC6.3, CC6.6 | Core logical access controls |
| ISO 27001 | A.9.x (all) | Access control is primary function |
| NIST | AC-1 through AC-8, AC-19 | Access enforcement and management |
| OWASP ASVS | V8 (all) | Authorization verification |
| GDPR | Art. 25, 32 | Processing authorization |

### Secondary Focus
- AU controls (audit logging of access decisions)
- CC7.2 (security event monitoring)

---

## Data Services & Storage Components

**Examples**: data-store, pg-data-service, cache services

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC6.7 (transmission), CC7.x | Data protection and operations |
| ISO 27001 | A.9.4.1, A.18.1.4 | Information restriction, privacy |
| NIST | AC-4 (flow), AU-2/AU-9 | Information flow, audit protection |
| OWASP ASVS | V1, V2 | Encoding, input validation |
| GDPR | Art. 17, 20, 30, 32 | Erasure, portability, records |

### Secondary Focus
- Encryption at rest/transit (Art. 32(1)(a))
- Retention policies (CC7.3, A.12.4.1)

---

## API Gateway & BFF Components

**Examples**: gateway, k8s-gateway, BFF services

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC6.1, CC6.6, CC6.7 | Boundary and transmission |
| ISO 27001 | A.9.1.2, A.9.4.1 | Network and information access |
| NIST | AC-3, AC-4, AU-2 | Enforcement, flow, events |
| OWASP ASVS | V8.3.1 (trusted enforcement layer) | Gateway as enforcement |
| GDPR | Art. 32(1)(b) | Confidentiality/integrity |

### Secondary Focus
- Rate limiting and abuse prevention
- Request/response validation (V2)

---

## Worker & Background Processing Services

**Examples**: deploy-worker, scenario-runner, secret-refresh-controller

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC6.1, CC7.2, CC7.3 | Access, monitoring, response |
| ISO 27001 | A.9.2.3, A.12.4.1 | Privileged access, logging |
| NIST | AC-5 (separation), AC-6 (least privilege), AU-x | Duty separation, minimal access |
| OWASP ASVS | V8.2.1, V13.2.2 | Privilege controls |
| GDPR | Art. 32(4) | Authorized processing only |

### Secondary Focus
- Credential handling (A.9.2.4)
- Error handling (V16)

---

## Event & Messaging Components

**Examples**: event-monitor, FastStream consumers/publishers

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC7.2, CC7.3 | Event monitoring, response |
| ISO 27001 | A.12.4.1-4 | Event logging, protection |
| NIST | AU-2 through AU-12 | Full audit family |
| OWASP ASVS | V16 (all) | Error handling and logging |
| GDPR | Art. 30, 32(1)(d) | Processing records, testing |

### Secondary Focus
- Message integrity
- Non-repudiation (AU-10)

---

## Libraries (Non-Service Components)

**Examples**: py-permissions, flowkit, naming-lib

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | Varies by library function | Function-specific |
| ISO 27001 | A.9.4.5 (source code access) | Code security |
| NIST | SA-15, SR-3 | Supply chain (Rev 5.2.0) |
| OWASP ASVS | V2 (if handles input) | Validation |
| GDPR | Art. 25(1) | Privacy by design |

### Key Considerations
- **Fail-Safe Defaults**: NFR for DENY on error
- **Information Leakage**: Error messages must not leak data
- **Auditability**: Must support (not implement) audit logging

---

## Infrastructure & Platform Components

**Examples**: KIND cluster, DNS, Kubernetes orchestration

### Primary Focus

| Standard | Controls | Rationale |
|----------|----------|-----------|
| SOC 2 | CC6.4, CC6.5, CC7.4, CC7.5 | Physical, continuity, recovery |
| ISO 27001 | A.6.2, A.9.1.2 | Teleworking, network access |
| NIST | AC-17, AC-18 | Remote/wireless access |
| OWASP ASVS | Limited applicability | Application-focused |
| GDPR | Art. 32(1)(c) | Availability/resilience |

### Typically Out of Scope
These controls are typically assigned to Infrastructure, not application components.

---

## Control Assignment Matrix

Quick reference for common out-of-scope assignments:

| Control | Description | Typically Assigned To |
|---------|-------------|----------------------|
| CC6.4, CC6.5 | Physical access | Infrastructure |
| A.9.2.1, A.9.2.2 | User provisioning | Auth Service |
| AC-17, AC-18 | Remote/wireless | Network Layer |
| GDPR Art. 17 | Right to erasure | Data Service |
| GDPR Art. 20 | Data portability | Data Service |
| A.6.2 | Mobile/teleworking | HR/IT Policy |
