---
id: SR-001
title: "Security profile review"
type: SpecReview
analysis: base
scope: "spec/functional/*.md, spec/components/*.md, spec/security/*.md, .ix/source-profile.yaml"
review_set: base
---
## Summary

This review records the binding clauses with no covering requirement and the unresolved profile clauses. The evaluation report is the applicability source; unresolved clauses have no control or exclusion artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Binding clause has no covering requirement. | nist/sp800-53@5.2.0#AC-1 |
| FND-002 | medium | Clause applicability is unresolved because the `data_class` context dimension is missing. | nist/sp800-53@5.2.0#CP-9 |
| FND-003 | medium | Clause applicability is unresolved because the `chapter` context dimension is missing. | owasp/asvs@5.0.0#V1.1.3 |
