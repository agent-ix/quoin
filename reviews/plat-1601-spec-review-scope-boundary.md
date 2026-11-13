---
id: SR-20261113052928
title: "Scope and boundary review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quoin; PR #881; FR-142, FR-144, FR-149, and engineering-assurance FR-044 as a referenced producer"
review_set: all
---

# Scope and boundary review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

The quoin adapter consumes the report produced through engineering-assurance `producer-execute`; it does not launch the scanner, read an SBOM, fetch a database, or open the network. FR-142 owns purl joins and FR-144 owns rendering. The boundary and responsibility split are explicit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Boundary assessment

External report production and advisory data remain with the referenced producer and source databases. Quoin owns only parsing the retained report, recording a `FindingRecord`, joining findings onto SOUP records, and rendering them. This review did not treat unsupported OS-package purls as an acceptable ticket outcome; that remains in the base finding.
