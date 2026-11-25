---
id: SR-2026112016223475846355046
title: "PLAT-1816 base spec review"
type: SpecReview
analysis: base
scope: "FR-191 dossier command, FR-207 records-of-processing content, and FR-215 vulnerability-management-plan content changes at PR head 85ffa442c4eb17f102d93ea1503c5502d699e26f"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
Reviewed changed requirement statements, criterion verifiability and trace coverage, and upstream contract consistency. FR-191-AC-31 is testable and bound to a direct executable-level test; the new exact-running-version requirement is distinct and useful. Its added producer shape and identity exclusions duplicate the authoritative spec-artifacts-process FR-018 contract.

## Verdict
FAIL — FR-191 restates the upstream producer identity shape and restrictions; reference FR-018 for that contract and retain only Quoin-specific runtime-version behavior.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-191 redefines producer identity shape and exclusions already specified by FR-018. | spec/functional/FR-191-dossier-command.md:106; spec-artifacts-process FR-018 |

## Coverage
Changed scope checked: FR-191 document-set clause and AC-31; FR-207 and FR-215 cross references. FR-191-AC-31 has a direct `#[trace]`-tagged test that starts the built Quoin executable, compares embedded version to `--version`, distinguishes document version, and checks prohibited fields. No applicable AssuranceProfile was found; inspected AP-201/AP-202 are out of scope. EARS engine output contains no warnings for FR-191. No new dependency edge was introduced.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-191 now references the authoritative FR-018 frontmatter contract and retains only Quoin's exact running-executable version requirement. |

- fnd: FND-001
  outcome: fixed
  after_excerpt: |-
    - The generated files SHALL comply with the `DossierDocument` frontmatter contract defined by spec-artifacts-process [FR-018](ix://agent-ix/spec-artifacts-process/FR-018). Versioned kinds also carry their document `version`, while `records-of-processing` does not. Source-backed RMM and SOUP documents also carry `as_of`.
    - Every generated file SHALL write the exact version string printed by the running Quoin executable to the producer version field defined by spec-artifacts-process FR-018. That producer version is independent of the dossier document's optional `version`.
