# NIST SP 800-53 Rev 5.2.0

**Version**: Revision 5.2.0  
**Scope**: Security and privacy controls for information systems  
**Region**: US Federal (widely adopted globally)

## When to Apply

Apply NIST 800-53 when your component:
- Is part of a US federal system
- Supports federal contractors (FedRAMP)
- Needs a comprehensive, well-documented control baseline
- Requires defense-in-depth security

## Family Selection Guide

**Use this to decide which family to investigate:**

| If your component... | Read Family |
|---------------------|-------------|
| Enforces authorization/RBAC/ABAC | [AC-AccessControl](AC-AccessControl.md) |
| Authenticates users or services | [IA-IdentificationAuthentication](IA-IdentificationAuthentication.md) |
| Produces audit logs | [AU-AuditAccountability](AU-AuditAccountability.md) |
| Uses encryption or TLS | [SC-SystemCommunications](SC-SystemCommunications.md) |
| Validates input or handles errors | [SI-SystemInformationIntegrity](SI-SystemInformationIntegrity.md) |
| Has configuration or dependencies | [CM-ConfigurationManagement](CM-ConfigurationManagement.md) |
| Is under active development | [SA-SystemServicesAcquisition](SA-SystemServicesAcquisition.md) |

## Control Families

20 control families organized by function:

| Family | Code | Primary For | Skip If |
|--------|------|-------------|---------|
| [AC-AccessControl](AC-AccessControl.md) | AC | Authorization components | No access control logic |
| [AU-AuditAccountability](AU-AuditAccountability.md) | AU | Logging/observability | No security events |
| [CM-ConfigurationManagement](CM-ConfigurationManagement.md) | CM | Config, SBOM | Stateless utility |
| [IA-IdentificationAuthentication](IA-IdentificationAuthentication.md) | IA | Auth services | No auth handling |
| [SA-SystemServicesAcquisition](SA-SystemServicesAcquisition.md) | SA | SDLC, supply chain | All software |
| [SC-SystemCommunications](SC-SystemCommunications.md) | SC | Crypto, network | No network/crypto |
| [SI-SystemInformationIntegrity](SI-SystemInformationIntegrity.md) | SI | Validation, patching | No external input |

### Typically Infrastructure (Lower Priority for Software)

| Family | Code | Assign To |
|--------|------|-----------|
| AT (Awareness/Training) | AT | HR/Training |
| CA (Assessment) | CA | Compliance |
| CP (Contingency) | CP | Infrastructure |
| IR (Incident Response) | IR | Operations |
| MA (Maintenance) | MA | Infrastructure |
| MP (Media) | MP | Infrastructure |
| PE (Physical) | PE | Facilities |
| PL (Planning) | PL | Management |
| PM (Program) | PM | Management |
| PS (Personnel) | PS | HR |
| RA (Risk) | RA | Risk team |
| SR (Supply Chain) | SR | Covered by SA |
