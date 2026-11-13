---
id: SR-901
title: "Security analysis of the example session gateway"
type: SpecReview
analysis: base
scope: "fixture repo: spec/functional/FR-001..FR-003, spec/components/COMP-001"
review_set: subset
---

# SR-901: Security analysis of the example session gateway

## Summary

The gateway answers eight NIST SP 800-53 Rev 5.2.0 and OWASP ASVS 5.0.0 clauses through three controls and excludes one clause to its platform. Two applicable clauses have no covering requirement or no responsible party. SOC 2 and ISO/IEC 27001 are not mapped, set not installed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Clause SC-28 of nist/sp800-53@5.2.0 applies to stored session data and no requirement covers it. | nist/sp800-53@5.2.0 SC-28 |
| FND-002 | low | Clause CP-9 of nist/sp800-53@5.2.0 is outside the gateway and no responsible party is known, so no scope_exclusion was written. | nist/sp800-53@5.2.0 CP-9 |
| FND-003 | low | SOC 2 and ISO/IEC 27001 were not mapped: not mapped, set not installed. | SOC 2, ISO/IEC 27001 |

## Verdict

**CONDITIONAL**: the gaps in FND-001 and FND-002 need a requirement and a named owner.
