---
id: FR-052
title: "Inventory the complete default-module corpus"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-051"
    type: "depends_on"
---

# FR-052: Inventory the complete default-module corpus

## Description

Given the frozen audit snapshot, the audit SHALL inventory every default module, declared semantic
type, contract surface, and Markdown document without silently excluding malformed or untyped input.

## Rationale

Type-fit conclusions are useful only when their denominator is visible and closed. Counting only
documents Quire accepts or only types with schemas would hide the exact gaps this audit exists to find.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-1 | The module denominator equals the ordered entries in `default-modules.yaml`; duplicates and unresolved entries remain separately visible and prevent a clean verdict. | Test (TC-1162) |
| FR-052-AC-2 | For every module, the inventory records every declared artifact and object type exactly once per declaration, including duplicate names across or within modules. | Test (TC-1163) |
| FR-052-AC-3 | Each declared type records its structural kind, module identity, declaration location, schema, skeleton, relations, mappings, projections, and each surface's presence or explicit absence. | Test (TC-1164) |
| FR-052-AC-4 | Every Markdown file under each pinned module corpus is retained with a module-relative path and one parse state from `parsed`, `invalid`, `untyped`, `excluded`, or `io-error`; non-parsed states include a reason. | Test (TC-1165) |
| FR-052-AC-5 | Parsed documents record their declared type, stable document identity when present, and definition/occurrence signals including run, result, evidence, timestamp, lifecycle, relationship, and provenance fields. | Test (TC-1166) |
| FR-052-AC-6 | Every declared type records zero or more representative document instances, and a type with none remains in the inventory with an explicit `no-instance` observation. | Test (TC-1167) |
| FR-052-AC-7 | The inventory publishes denominator reconciliation proving enumerated modules, declarations, contract surfaces, documents, and parse states equal their respective source counts. | Test (TC-1168) |

## Constraints

- Files explicitly excluded by a module contract remain in the document denominator with the reason.
- A parser failure is evidence; it SHALL NOT abort enumeration of sibling files.

## Dependencies

- **Upstream**: [FR-051](./FR-051-snapshot-semantic-audit-scope.md)
