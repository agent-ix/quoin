---
id: FR-141
title: "Promote ingested SBOM components to SOUP records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-009"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-010"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-011"
    type: "depends_on"
---

# FR-141: Promote ingested SBOM components to SOUP records

## Description

The `soup_record` object type belongs to `agent-ix/spec-objects-security` (FR-009 to FR-011 there).
`quoin` adds no record type for inventories (FR-041-CON-2); this requirement adds a command that writes
the other module's record from the components the FR-041 adapter already reads. `quoin` does not parse an
SBOM a second time and does not generate one.

## Behavior

- When a user runs `quoin soup promote` over an SBOM document, the command SHALL read the components through the FR-041 adapter.
- When a component carries a purl, the command SHALL write one `soup_record` for that purl, using the skeleton and schema of the installed spec-objects-security module.
- If the module that owns `soup_record` is not installed, then the command SHALL exit non-zero naming the missing module.
- If the module that owns `soup_record` is not installed, the command SHALL write nothing.
- If a component carries no purl, then the command SHALL skip it.
- If a component carries no purl, the command SHALL list it by name in the report.
- If two components of one document carry the same purl, then the command SHALL write one record for that purl.
- Where the SBOM states `name`, `version` or `supplier`, the command SHALL copy the stated value into the record.
- If the SBOM does not state `name`, `version` or `supplier`, then the command SHALL leave the key absent or write the literal `unknown`, as the owning schema admits.
- If the SBOM does not state `name`, `version` or `supplier`, the command SHALL NOT write any other default.
- The command SHALL NOT write `serves`, `known_anomalies`, `support_status`, `end_of_support` or `disposition_clause` on a record it creates, other than the `disposition_clause` of the next requirement.
- Where `--discharges <obligation-id>` is given, the command SHALL populate `disposition_clause` with that id.
- When a record for the purl already exists, the command SHALL leave `serves`, `known_anomalies`, `support_status`, `end_of_support` and `disposition_clause` byte-for-byte unchanged.
- When a record for the purl already exists and the SBOM states a different `version` or `supplier`, the command SHALL report the difference.
- When a record for the purl already exists and the SBOM states a different `version` or `supplier`, the command SHALL NOT overwrite the record.
- When the same document is promoted twice, the second run SHALL change no file.
- If `--discharges` names an obligation that no obligation states, then the command SHALL report it as `unmatched` as FR-041-AC-8 does.
- If `--discharges` names an obligation that no obligation states, the command SHALL write no `disposition_clause`.
- The command SHALL print a deterministic report listing records created, records left unchanged, differences and components skipped.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-141-AC-1 | A CycloneDX and an SPDX fixture that list the same three components with purls each promote to three `soup_record` files that validate against the owning schema. | Test |
| FR-141-AC-2 | A component with no purl writes no record and appears by name in the report; a component with neither purl nor name is dropped by the FR-041 adapter and is not reported as a skipped name. | Test |
| FR-141-AC-3 | A component with no supplier yields a record with no `supplier` key or `supplier: unknown`, never an empty string or a name copied from elsewhere. | Test |
| FR-141-AC-4 | A newly created record carries none of `serves`, `known_anomalies`, `support_status`, `end_of_support`; with `--discharges OB-1` it carries `disposition_clause: OB-1`, and without it no `disposition_clause` key. | Test |
| FR-141-AC-5 | Given an existing record whose five reviewer-authored fields are populated, promoting the same document leaves the file byte-identical and the report lists it as unchanged. | Test |
| FR-141-AC-6 | Promoting a document whose component version differs from the existing record reports the difference and leaves the record unchanged. | Test |
| FR-141-AC-7 | Promoting the same document twice leaves the directory byte-identical after the second run. | Test |
| FR-141-AC-8 | The command with the owning module absent exits non-zero naming the module and creates no file. | Test |
| FR-141-AC-9 | A document that is neither CycloneDX nor SPDX is refused with the FR-041-AC-4 error and writes no record. | Test |
| FR-141-AC-10 | A `--discharges` id no obligation states is reported `unmatched`, and the written records carry no `disposition_clause`. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-141-CON-1 | The command SHALL NOT contain an SBOM reader of its own; the reading is the FR-041 adapter's. | Design | Inspection |
| FR-141-CON-2 | The command SHALL NOT define a record type; the schema is the owning module's. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-041](./FR-041-sbom-inventory-evidence.md), [FR-046](./FR-046-clause-discharge.md); `agent-ix/spec-objects-security` FR-009, FR-010, FR-011.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
