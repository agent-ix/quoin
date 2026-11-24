---
id: FR-216
title: "Export person-authored SOUP VEX statements as OpenVEX JSON"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-009"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-022"
    type: "depends_on"
---

# FR-216: Export person-authored SOUP VEX statements as OpenVEX JSON

## Description

The `quoin soup vex export` command SHALL provide an OpenVEX JSON output mode for the optional `vex_statements`
authored on SOUP records. Each output statement represents one source statement for the SOUP record identified by
its canonical package URL. The export SHALL preserve that source and its authored meaning without changing or
reconciling it with advisory scans, vulnerability lifecycle records, or other analysis.

## Inputs

- The caller-selected repository and its SOUP records.
- The OpenVEX document `@id`, author, timestamp, and version; the caller SHALL supply these values to the command.
- The author's role, when supplied.
- An output path, or the output location selected by the command's existing default behavior.

## Outputs

- A UTF-8 JSON OpenVEX 0.2.0 document at the selected output path.
- A located error naming the SOUP record and source field when a selected statement cannot be serialized.

## Behavior

- The command SHALL select OpenVEX 0.2.0 output mode for `quoin soup vex export`.
- The document SHALL contain `@context` set to `https://openvex.dev/ns/v0.2.0`, the caller-supplied `@id`,
  `author`, `timestamp`, and `version`, the supplied `role` when present, and a `statements` list.
- The command SHALL emit one OpenVEX statement for each entry in each selected record's `vex_statements` field.
- The command SHALL set each statement's `vulnerability.name` to the entry's `advisory` value, its `status` to the
  entry's `status`, and its `products` to one product whose `@id` is the record's canonical package URL.
- The command SHALL copy `justification`, `impact_statement`, and `action_statement` to the correspondingly named
  OpenVEX statement fields when those source fields are present.
- The command SHALL NOT emit a statement from a vulnerability lifecycle record, advisory scan, scanner finding,
  reachability result, or other analysis.
- The command SHALL NOT infer, replace, or reconcile a VEX statement's status or justification from any other source.
- When `vex_statements` is absent or empty on every selected record, the document SHALL contain an empty
  `statements` list.
- When a selected statement cannot be represented or fails an OpenVEX-required condition, the command SHALL report
  the record and field and SHALL NOT write a partial document or change an existing output file.
- The command SHALL write deterministic JSON for identical records and document metadata.
- The output SHALL conform to the published [OpenVEX 0.2.0 specification](https://github.com/openvex/spec/blob/main/OPENVEX-SPEC.md)
  and its [JSON schema](https://github.com/openvex/spec/blob/main/openvex_json_schema.json).

### OpenVEX field mapping

| SOUP VEX source | OpenVEX 0.2.0 output | Mapping |
|---|---|---|
| `advisory` | `statements[].vulnerability.name` | Copy the identifier unchanged. |
| Record canonical package URL | `statements[].products[].@id` | Emit one product reference for the SOUP record. |
| `status` | `statements[].status` | Copy one of the four source values unchanged. |
| `justification` | `statements[].justification` | Copy the source value unchanged when present. |
| `impact_statement` | `statements[].impact_statement` | Copy unchanged when present. |
| `action_statement` | `statements[].action_statement` | Copy unchanged when present. |

The exporter SHALL NOT add fields for assessor, assessment date, or evidence references when OpenVEX 0.2.0 has no
corresponding statement field. It SHALL NOT change the statement timestamp to the source `assessed_at` date or
derive a document timestamp from a source record; document metadata is supplied explicitly as an input.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-216-AC-1 | A selected record with two authored VEX statements emits exactly two OpenVEX statements, each with the exact advisory identifier, source status, and canonical package URL. | Test |
| FR-216-AC-2 | Each present source justification, impact statement, and action statement is copied unchanged to the correspondingly named OpenVEX field; absent optional fields remain absent. | Test |
| FR-216-AC-3 | The document contains the OpenVEX 0.2.0 context and the caller-supplied `@id`, author, timestamp, and version, and contains role only when supplied. | Test |
| FR-216-AC-4 | When no selected SOUP record has a VEX statement, the document contains an empty `statements` list even if scans or vulnerability lifecycle records contain entries. | Test |
| FR-216-AC-5 | Given a source VEX statement and conflicting advisory scan or vulnerability lifecycle data, the output retains only the authored VEX statement's values and does not add statements from those other sources. | Test |
| FR-216-AC-6 | A malformed source statement reports its SOUP record and field, writes no partial output, and leaves any existing output byte-unchanged. | Test |
| FR-216-AC-7 | Repeating the export with identical records and document metadata produces byte-identical JSON. | Test |
| FR-216-AC-8 | The generated document validates against the published OpenVEX 0.2.0 schema. | Test |

## Dependencies

- **Upstream**: [FR-009](./FR-009-soup-record-identity-and-promotion.md) defines SOUP record identity; spec-objects-security [FR-022](ix://agent-ix/spec-objects-security/FR-022) defines the authored `vex_statements` fields.
- **Context**: [FR-156](./FR-156-vex-export-for-soup-statements.md) exports those authored statements in CycloneDX JSON; this requirement defines OpenVEX output.
- **Downstream**: PLAT-1842 OpenVEX export.
