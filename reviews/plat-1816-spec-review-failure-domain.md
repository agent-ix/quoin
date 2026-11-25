---
id: SR-2026112016223475846355050
title: "PLAT-1816 failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "FR-191 producer identity implementation and FR-191-AC-31"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
The producer version is sourced from Quoin's single build-version function, serialized with the shared YAML quoting helper, and exercised through the actual CLI executable. The new frontmatter contains no subject revision, hash, digest, generation timestamp, Quire version, Engineering Assurance version, or generic provenance field.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | -------------------------------- | --- |
| FND-001 | low | No issues found (placeholder) | - |
