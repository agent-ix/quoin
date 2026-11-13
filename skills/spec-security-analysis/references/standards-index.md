# Security Standards Reference

Index of security standards for compliance analysis. Each standard has its own directory with chapter-level detail.

## When to Apply

**ALL standards apply to ALL components.** Apply comprehensive coverage, then scope based on component function.

| Standard | Primary Application | Key Indicator |
|----------|---------------------|---------------|
| [SOC 2](soc2/index.md) | Service organization controls | Any B2B SaaS or service provider |
| [ISO 27001](iso27001/index.md) | Information security management | Enterprise, international, or certified requirements |
| [NIST 800-53](nist-800-53/index.md) | Federal information systems | US government, federal contractors |
| [OWASP ASVS](owasp-asvs/index.md) | Application security verification | Web applications, APIs, mobile |
| [GDPR](gdpr/index.md) | EU data protection | Any EU user data processing |

## Standard Summaries

### SOC 2 (2017 TSC)
Trust Services Criteria for service organizations. Focuses on Security (mandatory), plus optional Availability, Processing Integrity, Confidentiality, and Privacy.

**Apply when**: Building B2B services, cloud services, or handling customer data for other organizations.

### ISO/IEC 27001:2022
International standard for Information Security Management Systems (ISMS). Annex A provides 93 controls across 4 themes.

**Apply when**: International operations, enterprise customers, or seeking certification.

### NIST SP 800-53 Rev 5.2.0
Comprehensive security and privacy controls for federal information systems. 20 control families covering all aspects of security.

**Apply when**: US federal systems, government contractors, or as comprehensive baseline.

### OWASP ASVS 5.0.0
Application Security Verification Standard. 17 verification chapters for testing application security.

**Apply when**: Web applications, APIs, any user-facing software.

### EU GDPR 2016/679
General Data Protection Regulation for EU data protection and privacy.

**Apply when**: Processing personal data of EU residents.

## Quick Reference

For component-type specific guidance, see [component-type-mapping.md](component-type-mapping.md).
