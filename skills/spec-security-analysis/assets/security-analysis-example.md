# Example: py-permissions Security Compliance Traceability

This example shows the complete Section 19 from the py-permissions library specification.

---

## 19. Compliance Standards Traceability

This specification targets alignment with the following industry security and privacy standards.

### 19.1 Targeted Standards

| Standard | Version | Scope | Region |
|----------|---------|-------|--------|
| AICPA SOC 2 | 2017 TSC | CC6 (Logical Access Controls) | Global |
| ISO/IEC 27001 | 2022 | Annex A.9 (Access Control) | Global |
| NIST SP 800-53 | Rev 5.2.0 | AC Family (Access Control) | US Federal |
| OWASP ASVS | 5.0.0 | V8 (Authorization) | Global |
| EU GDPR | 2016/679 | Articles 25, 32 (Security of Processing) | EU/EEA |

### 19.2 Control Mapping

| Control ID | Standard | Control Name | py-permissions Requirement |
|------------|----------|--------------|---------------------------|
| CC6.1 | SOC2 | Logical Access Security | FR-001, FR-002, FR-008, FR-009 |
| CC6.2 | SOC2 | User Registration/Authorization | FR-014 (Role Schema) |
| CC6.3 | SOC2 | Access Modification/Removal | FR-008 (RBAC), FR-014 |
| CC6.6 | SOC2 | Boundary Protection | FR-007 (Org Tenancy) |
| CC7.2 | SOC2 | Security Event Monitoring | FR-013 (Audit Logging) |
| A.9.1.1 | ISO 27001 | Access Control Policy | StR-001 |
| A.9.2.3 | ISO 27001 | Privileged Access Management | FR-008, FR-010 |
| A.9.2.5 | ISO 27001 | Review of User Access Rights | FR-015 (Permission Query) |
| A.9.4.1 | ISO 27001 | Information Access Restriction | FR-005, FR-003 |
| A.12.4.1 | ISO 27001 | Event Logging | FR-013 |
| AC-2 | NIST | Account Management | FR-014 |
| AC-3 | NIST | Access Enforcement | FR-001, FR-002 |
| AC-5 | NIST | Separation of Duties | FR-008 (RBAC) |
| AC-6 | NIST | Least Privilege | FR-005, FR-008 |
| AU-2 | NIST | Audit Events | FR-013 |
| 8.1.1 | ASVS | Access Control Model | FR-008, FR-009 |
| 8.2.2 | ASVS | Sensitive Data Access Control | FR-005, FR-007 |

### 19.3 GDPR Compliance

| GDPR Article | Requirement | py-permissions Coverage |
|--------------|-------------|------------------------|
| Art. 25(1) | Data Protection by Design | FR-008 (RBAC), FR-009 (ABAC), NFR-002 (Fail-Safe) |
| Art. 25(2) | Data Protection by Default | FR-005 (Resource-Level), FR-003 (List Filtering) |
| Art. 32(1)(b) | Confidentiality/Integrity | FR-007 (Org Tenancy), FR-002 (Enforcement) |
| Art. 32(1)(d) | Regular Testing | NFR-001, NFR-003, NFR-004 (Verification) |
| Art. 32(4) | Authorized Processing Only | FR-001, FR-002, FR-013 (Audit) |

Key GDPR principles addressed:
- **Least Privilege**: FR-005, FR-008 enforce minimum necessary access
- **Access Logging**: FR-013 provides audit trail for Art. 30 records
- **Multi-Tenancy**: FR-007 ensures organization isolation (data controller boundaries)

### 19.4 Audit Support Controls

| Control Requirement | Standard Reference | py-permissions Coverage |
|--------------------|-------------------|------------------------|
| Audit Logging | SOC2 CC7.2, ISO A.12.4, NIST AU-2 | FR-013 |
| Log Integrity | NIST AU-9 | FR-013-CON-2 |
| Fail-Safe Access | NIST AC-7, ISO A.9.4.2, GDPR Art. 32 | NFR-002 |
| Retention Compatibility | SOC2 CC7.3, ISO A.12.4.1, GDPR Art. 30 | NFR-005 |

### 19.5 Out of Scope

The following controls are outside py-permissions scope:

| Control | Description | Responsible Component |
|---------|-------------|----------------------|
| CC6.4, CC6.5 | Physical Access/Asset Disposal | Infrastructure |
| A.9.2.1, A.9.2.2 | User Registration/Provisioning | Auth Service |
| AC-17, AC-18 | Remote/Wireless Access | Network Layer |
| GDPR Art. 17 | Right to Erasure | Data Service Layer |
| GDPR Art. 20 | Data Portability | Data Service Layer |

---

## Key Observations

This example demonstrates:

1. **Comprehensive Coverage**: All 5 mandatory standards included with specific scopes
2. **Bidirectional Traceability**: Each control maps to specific FR/NFR/StR identifiers
3. **Overlapping Controls**: Multiple standards covering same area (e.g., Audit Logging) strengthens coverage
4. **Clear Boundaries**: Out-of-scope controls explicitly assigned to responsible components
5. **GDPR Principles**: Extracted and documented for EU compliance visibility
