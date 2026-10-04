---
name: spec-security-analysis
description: Analyze specifications and implementations to identify applicable security standards and generate compliance traceability documentation.
---

# Security Compliance Analysis

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Analyze a component's specification and implementation to generate comprehensive security compliance traceability. Apply ALL security standards for maximum coverage - overlapping controls strengthen rather than conflict.

## Mandatory Standards

Apply ALL of these standards to EVERY analysis (scope varies by component function):

| Standard | Version | Scope |
|----------|---------|-------|
| AICPA SOC 2 | 2017 TSC | Trust Services Criteria |
| ISO/IEC 27001 | 2022 | Annex A Controls |
| NIST SP 800-53 | Rev 5.2.0 | Security & Privacy Controls |
| OWASP ASVS | 5.0.0 | Application Security Verification |
| EU GDPR | 2016/679 | Data Protection Articles |

## Process

1. **Gather Context**
   - Read `spec/spec.md` (if exists)
   - Scan implementation for security-relevant patterns (auth, data, API, logging)
   - Identify component type and function

2. **Identify Applicable Controls**
   - Read [references/standards-index.md](references/standards-index.md) for standard selection
   - Select relevant chapters from each standard's index:
     - [soc2/index.md](references/soc2/index.md) → CC1-CC9 categories
     - [iso27001/index.md](references/iso27001/index.md) → A5-A8 themes
     - [nist-800-53/index.md](references/nist-800-53/index.md) → Control families
     - [owasp-asvs/index.md](references/owasp-asvs/index.md) → V1-V14 areas
     - [gdpr/index.md](references/gdpr/index.md) → Chapters II-V
   - Use index decision guides to choose specific chapters to investigate
   - Reference [component-type-mapping.md](references/component-type-mapping.md) for scope guidance
   - **[REQUIRED]** Verify identified controls against official FREE sources:
     - NIST 800-53: https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final (US Gov - fully open)
     - OWASP ASVS: https://github.com/OWASP/ASVS (CC BY-SA 4.0 - fully open)
     - GDPR: https://eur-lex.europa.eu/eli/reg/2016/679/oj (EU Law - fully open)
     - SOC 2 TSC: https://en.wikipedia.org/wiki/SOC_2 (reference summary - no login)
     - ISO 27001: https://en.wikipedia.org/wiki/ISO/IEC_27001 (control list - no login)
   - Local references are summaries; use web search for specific control details

3. **Map Controls to Requirements**
   - Link each applicable control to existing FR/NFR/StR requirements
   - Identify gaps where controls lack requirement coverage
   - Document requirement-to-control bidirectional traceability

4. **Define Scope Boundaries**
   - Identify controls outside component scope
   - Assign responsible component for out-of-scope controls
   - Document explicitly what is NOT covered and why

5. **Generate Output**
   - Use template: [assets/security-analysis-template.md](assets/security-analysis-template.md)
   - Output as Section 19 "Compliance Standards Traceability" in `spec/spec.md`
   - Example: [assets/security-analysis-example.md](assets/security-analysis-example.md)

## Output Sections

- **19.1 Targeted Standards**: All 5 mandatory standards with version and scope
- **19.2 Control Mapping**: Control ID → Standard → Name → Component Requirement
- **19.3 GDPR Compliance**: Specific GDPR article coverage
- **19.4 Audit Support Controls**: Cross-standard audit and logging controls
- **19.5 Out of Scope**: Controls outside component boundary with responsible component

## Rules

- Apply ALL standards - comprehensive coverage trumps minimal compliance
- Every control mapping MUST reference specific FR/NFR/StR identifiers
- Overlapping controls from multiple standards reinforce coverage
- Out-of-scope controls MUST identify the responsible component
- Gap identification is required - missing control coverage is a finding

## References

- Template: [assets/security-analysis-template.md](assets/security-analysis-template.md)
- Example: [assets/security-analysis-example.md](assets/security-analysis-example.md)
- Standards Index: [references/standards-index.md](references/standards-index.md)
- Component Mapping: [references/component-type-mapping.md](references/component-type-mapping.md)

