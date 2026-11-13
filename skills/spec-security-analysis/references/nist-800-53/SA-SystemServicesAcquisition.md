# SA - System and Services Acquisition

**Family**: System and Services Acquisition  
**Controls**: 22  
**Focus**: SDLC, supply chain, development security

## When to Apply

**SA is PRIMARY for development practices.** Apply when your component:
- Is under active development
- Uses third-party dependencies
- Requires security in SDLC
- Has supply chain concerns

## Controls

| Control | Name | Description | Software Relevance |
|---------|------|-------------|-------------------|
| SA-1 | Policy and Procedures | Document SA policies | LOW - Docs |
| SA-2 | Allocation of Resources | Allocate security resources | LOW - Planning |
| SA-3 | System Development Life Cycle | Follow SDLC | **HIGH** - Dev process |
| SA-4 | Acquisition Process | Define acquisition | MEDIUM - Procurement |
| SA-5 | System Documentation | Obtain documentation | MEDIUM - Docs |
| SA-8 | Security and Privacy Engineering Principles | Apply principles | **HIGH** - Design |
| SA-9 | External System Services | Control external services | **HIGH** - Dependencies |
| SA-10 | Developer Configuration Management | Dev config mgmt | **HIGH** - SCM |
| SA-11 | Developer Testing and Evaluation | Dev testing | **HIGH** - Testing |
| SA-15 | Development Process, Standards, and Tools | Dev standards | **HIGH** - Standards |
| RA-5(11) | Vulnerability Disclosure | Disclosure program | MEDIUM - Security.txt |
| SA-16 | Developer-Provided Training | Dev training | LOW - Training |
| SA-17 | Developer Security and Privacy Architecture and Design | Secure design | **HIGH** - Architecture |
| SA-21 | Developer Screening | Screen developers | LOW - HR |
| SA-22 | Unsupported System Components | Address unsupported | **HIGH** - EOL tracking |
| SR-3 | Software Supply Chain | Supply chain security | **HIGH** - SBOM |

## Mapping Guidance

### Development (SA-3, SA-8, SA-11, SA-15)
```
NFR-XXX (SDLC) → SA-3
NFR-XXX (Secure Design) → SA-8
NFR-XXX (Testing) → SA-11
NFR-XXX (Standards) → SA-15
```

### Supply Chain (SA-9, SA-22, SR-3)
```
NFR-XXX (Dependencies) → SA-9, SR-3
NFR-XXX (EOL Tracking) → SA-22
```
