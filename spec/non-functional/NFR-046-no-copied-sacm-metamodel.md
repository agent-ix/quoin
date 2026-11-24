---
id: NFR-046
title: "Keep the SACM metamodel external to quoin"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-226"
    type: "constrains"
---

# NFR-046: Keep the SACM metamodel external to quoin

## Statement

The quoin repository SHALL contain zero copies of the SACM metamodel.

## Scope

- Applies to: the SACM exporter and its local tests.
- Operational context: a developer checkout or released quoin package.

## Rationale

The normative model remains maintained and published by OMG; quoin carries
only its own SACM output contract.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Copies of the SACM metamodel in the quoin repository | 0 | 0 | Inspection |
| New runtime dependencies outside the workspace other than the selected XML writer | 0 | 0 | Inspection |

## Verification

Inspect repository contents for a copied SACM model and inspect runtime
dependencies. If an XML writer is required, the only permitted external writer
is `quick-xml` under MIT; the workspace currently has no XML writer. Its exact
version is selected in the Cargo manifest and lockfile.
