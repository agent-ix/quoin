---
id: SR-20261123190538
title: "Integrity review of SOUP promotion keeping SBOM supplier and licence"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-141-promote-sbom-components-to-soup-records.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/spec.md"
review_set: subset
---

# Integrity review of SOUP promotion keeping SBOM supplier and licence

## Summary

Ticket: PLAT-1812 (PR agent-ix/quoin#1030). Consistency, completeness and cross-reference check of the changed
requirements against each other and the owning module. Statements, ACs and constraints agree in substance; the
defects are traceability edges and one redundant criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-141 Dependencies now lists FR-150 as upstream, but the frontmatter `relationships` has no FR-150 edge, and FR-141 uses nothing FR-150 defines (the licences come from the FR-041 reader, FR-041-AC-10). Either drop FR-150 from Dependencies or add the edge with a stated reason. | FR-141 Dependencies, FR-141 frontmatter |
| FND-002 | low | FR-041 now has a stated caller in FR-141 (its supplier section links FR-141), but FR-041's Dependencies still reads "Downstream: none" and its frontmatter references FR-150 only, so the reverse edge is missing. | FR-041 Dependencies, FR-041 frontmatter |
| FND-003 | low | FR-141-AC-23 repeats FR-141-AC-4's `license_fact_ref` absence clause; the "both `license` and `license_fact_ref`" half follows from it. One of the two clauses is redundant. | FR-141-AC-23, FR-141-AC-4 |

## Verdict

No integrity defect above low. The new statements, AC-18..23 and CON-3/CON-4 cover each other; CON-1's rewording
keeps a single reader; FR-150's sentence matches the reader contract; spec/spec.md's summary matches FR-141.

## Examined

FR-141 frontmatter, Description, Behavior, AC-1..AC-23, CON-1..CON-4, Dependencies; FR-041 frontmatter, supplier
section, AC-11, Dependencies; FR-150 changed paragraph; spec/spec.md FR-141 line.
