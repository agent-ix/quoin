---
id: FR-156
title: "Export person-authored SOUP VEX statements as CycloneDX JSON"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-142"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-030"
    type: "constrained_by"
  - target: "ix://agent-ix/spec-objects-security/FR-009"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-022"
    type: "depends_on"
---
# FR-156: Export person-authored SOUP VEX statements as CycloneDX JSON

## Description

The `quoin soup vex export` command SHALL serialize each person-authored entry in `vex_statements` in a `soup_record` as one CycloneDX 1.7 JSON vulnerability entry associated with that record's component purl. The command SHALL export only statements stored in SOUP records. The command SHALL NOT create a statement from a scan, an advisory disposition, reachability result or other analysis.

## Inputs

- A repository and its SOUP record directory.
- A Syft CycloneDX SBOM path used to place the VEX document alongside that SBOM.
- The `soup_record` files in the selected records directory.

## Outputs

- A CycloneDX 1.7 JSON VEX document at the requested output path or, when omitted, next to the Syft SBOM with the suffix `.vex.cdx.json`.
- A validation error naming the record and field when a statement is invalid or cannot be represented.

## Behavior

- The command SHALL read `vex_statements` from each record and emit one `vulnerabilities[]` entry for each statement.
- The command SHALL use the Syft SBOM path only to choose the default output location.
- The command SHALL NOT read or modify the Syft SBOM.
- The command SHALL emit a component with a stable `bom-ref` and that record's purl for each component referenced by an exported statement.
- Each vulnerability entry's `affects[].ref` SHALL reference its component.
- The command SHALL copy the statement's advisory identifier to the CycloneDX vulnerability `id`.
- The command SHALL map each stored status and the affected component version status as specified in the table below.
- The command SHALL preserve the source status in the named property so consumers can distinguish the authored VEX vocabulary from CycloneDX's analysis vocabulary.
- The command SHALL preserve source fields that CycloneDX does not represent with an equivalent native field as named vulnerability properties.
- The command SHALL produce one property for each evidence reference.
- The command SHALL include only `analysis.justification` mappings with an equivalent CycloneDX 1.7 justification.
- The command SHALL retain every stored source justification in `quoin:vex:justification`, including values without a native CycloneDX equivalent.
- When the current read of every applicable advisory scan no longer reports the statement's advisory for its record, the command SHALL label the exported statement as stale with `quoin:vex:stale=true`.
- When an applicable scan is missing, unordered or vacuous, the command SHALL NOT infer staleness.
- The command SHALL omit `vulnerabilities` when there are no statements to export.
- The command SHALL NOT emit a vulnerability entry for an advisory that has no matching person-authored statement.
- The command SHALL write a deterministic JSON document for identical record, SBOM and scan inputs.

### CycloneDX 1.7 mapping

The JSON document SHALL use the published [CycloneDX 1.7 JSON schema](https://cyclonedx.org/schema/bom-1.7.schema.json) and the [CycloneDX 1.7 JSON reference](https://cyclonedx.org/docs/1.7/json/). CI SHALL retrieve the schema as a validation input. The repository SHALL NOT contain a downloaded schema copy.

| SOUP statement field | CycloneDX 1.7 output | Mapping |
|---|---|---|
| `advisory` | `vulnerabilities[].id` | Copy the identifier unchanged. |
| Record purl | `components[].purl`; referenced by `vulnerabilities[].affects[].ref` | Use the purl as the component's `bom-ref`; emit the record version under `affects[].versions[].version`. |
| `status=affected` | `vulnerabilities[].analysis.state=exploitable`; `affects[].versions[].status=affected` | Direct VEX meaning to CycloneDX analysis and affected-version vocabularies. |
| `status=not_affected` | `analysis.state=not_affected`; `affects[].versions[].status=unaffected` | Direct mapping. |
| `status=under_investigation` | `analysis.state=in_triage`; `affects[].versions[].status=unknown` | Direct mapping to CycloneDX investigation and unspecified version state. |
| `status=fixed` | `analysis.state=resolved`; `affects[].versions[].status=unaffected` | Direct mapping to CycloneDX remediated state and unaffected version. |
| `status` | `vulnerabilities[].properties[]` with `name=quoin:vex:source-status` and `value` equal to the source value | Also retain the exact source value. |
| `justification=vulnerable_code_not_present` | `analysis.justification=code_not_present` | Equivalent native justification. |
| `justification=vulnerable_code_not_in_execute_path` | `analysis.justification=code_not_reachable` | Equivalent native justification. |
| `justification=inline_mitigations_already_exist` | `analysis.justification=protected_by_mitigating_control` | Equivalent native justification. |
| `justification=component_not_present` | `vulnerabilities[].properties[]` with `name=quoin:vex:justification` and `value` equal to the source value | CycloneDX 1.7 has no equivalent justification enum; preserve the source value without mapping it to a different assertion. |
| `justification=vulnerable_code_cannot_be_controlled_by_adversary` | `vulnerabilities[].properties[]` with `name=quoin:vex:justification` and `value` equal to the source value | No equivalent value in the CycloneDX 1.7 justification enum; preserve the source value without substituting a different assertion. |
| `impact_statement` | `analysis.detail` | Copy unchanged. |
| `action_statement` | `vulnerabilities[].recommendation` | Copy unchanged when present. |
| `assessed_by` | `vulnerabilities[].properties[]` with `name=quoin:vex:assessed-by` and `value` equal to the named assessor | Copy the named assessor unchanged. |
| `assessed_at` | `vulnerabilities[].properties[]` with `name=quoin:vex:assessed-at` and `value` equal to the ISO 8601 date | Copy the date without adding time precision. |
| Each `evidence_refs[]` item | `vulnerabilities[].properties[]` with `name=quoin:vex:evidence-ref` and `value` equal to that reference | Emit one property per reference, preserving the reference value. |
| A stale statement | `vulnerabilities[].properties[]` with `name=quoin:vex:stale`, `value="true"` | Emit only when the stale condition above is established. |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-156-CON-1 | The export SHALL conform to the published CycloneDX 1.7 JSON schema. | Interface | Test |
| FR-156-CON-2 | The repository SHALL NOT contain a downloaded schema copy. | Interface | Inspection |
| FR-156-CON-3 | The export SHALL NOT infer a VEX status or create an entry without a stored person-authored statement. | Reliability | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-156-AC-1 | A fixture containing one statement for each of the four statuses exports exactly four vulnerability entries, each carrying the expected CycloneDX 1.7 `analysis.state`, affected-version status, source status property and component purl reference. | Test |
| FR-156-AC-2 | For every exported entry, `advisory`, assessor, assessment date, impact statement, optional action statement and each evidence reference equal the source statement; each statement in a record produces exactly one entry, and the same advisory id on two records produces two entries. | Test |
| FR-156-AC-3 | Statements with all five allowed justifications export the exact source value; `vulnerable_code_not_present`, `vulnerable_code_not_in_execute_path` and `inline_mitigations_already_exist` map to their equivalent CycloneDX justifications, while `component_not_present` and `vulnerable_code_cannot_be_controlled_by_adversary` preserve the source value and omit `analysis.justification`. | Test |
| FR-156-AC-4 | The output validates against the published CycloneDX 1.7 JSON schema fetched as a CI input; the schema is absent from the repository. | Test |
| FR-156-AC-5 | With no `vex_statement`, the output contains no `vulnerabilities[]` entry, including when the scan reports advisories; every output entry with an advisory has exactly one matching stored person-authored statement. | Test |
| FR-156-AC-6 | A statement absent from all current applicable scans is exported with `quoin:vex:stale=true`; an unavailable, unordered or vacuous applicable scan does not produce that property. | Test |
| FR-156-AC-7 | Repeating export with identical records, SBOM and scan inputs yields byte-identical JSON at the same output path. | Test |

## Dependencies

- **Upstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md), `agent-ix/spec-objects-security` [FR-009](ix://agent-ix/spec-objects-security/FR-009) and [FR-022](ix://agent-ix/spec-objects-security/FR-022).
