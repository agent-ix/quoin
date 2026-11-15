# Security Standards Reference

Index of security standards for compliance analysis. Each standard has its own directory with chapter-level detail.

## Selection

This index is secondary guidance for interpreting selected sources. The repository's source profile and its
`source-profile-evaluation-v1` document determine which clause sets are in scope and which clauses are binding.
Do not select a source or override an evaluation outcome based on this index.

| Standard | Context when selected | Key Indicator |
|----------|---------------------|---------------|
| [NIST 800-53](nist-800-53/index.md) | Federal information systems | US government, federal contractors |
| [OWASP ASVS](owasp-asvs/index.md) | Application security verification | Web applications, APIs, mobile |
| [GDPR](gdpr/index.md) | Selected clauses from Articles 25, 30, 32, and 35 in [`eu/gdpr-2016-679@2016-05-04`](https://github.com/agent-ix/spec-clauses-eu-law/blob/main/spec_clauses_eu_law/clauses/eu-gdpr-2016-679-2016-05-04.json) | Only when this exact set is in the source-profile evaluation |

## Standard Summaries

### NIST SP 800-53 Rev 5.2.0
Comprehensive security and privacy controls for federal information systems. 20 control families covering all aspects of security.

**Context**: US federal systems, government contractors, or a broad security baseline.

### OWASP ASVS 5.0.0
Application Security Verification Standard. 17 verification chapters for testing application security.

**Context**: Web applications, APIs, and other user-facing software.

### EU GDPR 2016/679
Use only the selected set [`eu/gdpr-2016-679@2016-05-04`](https://github.com/agent-ix/spec-clauses-eu-law/blob/main/spec_clauses_eu_law/clauses/eu-gdpr-2016-679-2016-05-04.json) when its exact key appears in the source-profile evaluation. It covers selected clauses from Articles 25, 30, 32, and 35; other GDPR topics, including retained Chapters II, III, and V summaries, are untyped context and are not evaluated. Applicability uses the set’s person-supplied dimensions.

## Licence-restricted sources

Licence-restricted sources have no reference table here. They are mapped only from a locally installed pack, by clause id; with no pack installed the source is reported "not mapped, set not installed".

## Quick Reference

For component-type specific guidance, see [component-type-mapping.md](component-type-mapping.md).
