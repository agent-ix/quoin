---
id: SR-2026112016223475846355048
title: "PLAT-1816 spec integrity review"
type: SpecReview
analysis: integrity
scope: "Changed FR-191 document-set clause and AC-31; FR-207 and FR-215 references"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
The new test criterion is falsifiable and directly trace-tagged. It combines related frontmatter assertions in one generator identity contract. FR-191 duplicates the existing producer_identity shape and no-provenance contract from upstream FR-018, so the cross-repository source of truth should remain authoritative there.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-191 duplicates the upstream FR-018 producer identity and exclusion contract. | spec/functional/FR-191-dossier-command.md:106; spec-artifacts-process FR-018 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-191 now references the authoritative FR-018 frontmatter contract and retains only Quoin's exact running-executable version requirement. |

- fnd: FND-001
  outcome: fixed
  after_excerpt: |-
    - The generated files SHALL comply with the `DossierDocument` frontmatter contract defined by spec-artifacts-process [FR-018](ix://agent-ix/spec-artifacts-process/FR-018). Versioned kinds also carry their document `version`, while `records-of-processing` does not. Source-backed RMM and SOUP documents also carry `as_of`.
    - Every generated file SHALL write the exact version string printed by the running Quoin executable to the producer version field defined by spec-artifacts-process FR-018. That producer version is independent of the dossier document's optional `version`.
