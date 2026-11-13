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
SBOM a second time and does not generate one, and FR-041 is not changed: the command uses only the
component symbol that adapter emits, which is the purl where the SBOM states one and `name@version` otherwise.

A record's `name` and `version` are the percent-decoded name and version segments of its purl. `supplier` is not promoted:
the FR-041 adapter does not expose it, and the record leaves it absent for a reviewer to state.

Promotion supersedes the PLAT-1515 criterion that the record count equals the SBOM component count. A component
with no purl is skipped and a repeated purl is one record, so the count that holds is the accounting of FR-141-AC-2.

## Behavior

- When a user runs `quoin soup promote` over an SBOM document, the command SHALL read the components through the FR-041 adapter.
- When a component symbol is a purl, the command SHALL write one `soup_record` for that purl, using the skeleton and schema of the installed spec-objects-security module.
- If the module that owns `soup_record` is not installed, then the command SHALL exit non-zero naming the missing module.
- If the module that owns `soup_record` is not installed, the command SHALL write nothing.
- If a component symbol is not a purl, then the command SHALL skip the component.
- If a component symbol is not a purl, the command SHALL list it in the report by its FR-041 symbol, which is `name@version`, or `name` where no version is stated.
- If two components of one document carry the same purl, then the command SHALL write one record for that purl.
- If two components of one document carry the same purl, the command SHALL count the repeat as a merged duplicate in the report.
- The command SHALL write `name` and `version` from the percent-decoded purl segments, and `version` only where the purl carries one.
- If a decoded name or version contains a control character, then the command SHALL skip the component and list it in the report.
- The command SHALL give each new record the id `SOUP-<n>`, with `<n>` of at least three digits, numbering on from the highest `SOUP-` id among the records in the `--records` and `--out` directories, in ascending purl order.
- The command SHALL leave `supplier` absent on a record it creates.
- The command SHALL NOT write `serves`, `known_anomalies`, `support_status` or `end_of_support` on a record it creates.
- Where `--discharges <obligation-id>` is not given, the command SHALL NOT write `disposition_clause` on a record it creates.
- Where `--discharges <obligation-id>` is given and an obligation states that id, the command SHALL write the id verbatim as `disposition_clause`.
- If `--discharges` names an id that no obligation states, then the command SHALL report it as `unmatched` as FR-041-AC-8 does.
- If `--discharges` names an id that no obligation states, the command SHALL write no `disposition_clause`.
- The command SHALL write each new record under `--out <dir>` (default `spec/soup/` of the repository) in a file named by the purl without its qualifiers and subpath, with every byte outside `A-Za-z0-9._-` replaced by `%` and its two uppercase hex digits, followed by `.md`.
- If the file a new record would be written to is the `soup_record` of the same purl, then the command SHALL count the component unchanged.
- If the file a new record would be written to exists and is not the `soup_record` of the same purl, then the command SHALL exit non-zero naming the file.
- If the file a new record would be written to exists and is not the `soup_record` of the same purl, the command SHALL write nothing.
- If the file name of a new record exceeds 255 bytes, then the command SHALL exit non-zero naming the file and write nothing.
- If a `.md` file below the records directory cannot be read in full or exceeds 1 MiB, then the command SHALL exit non-zero naming the file and write nothing.
- The command SHALL look up existing records in `--records <dir>` (default: the `--out` directory), reading every `soup_record` below it.
- When a record for the purl exists in the records directory, the command SHALL NOT modify that file.
- If two records in the records directory carry one purl, then the command SHALL exit non-zero naming both files.
- When the same document is promoted twice, the second run SHALL change no file.
- Where a new record's purl, without version, qualifiers and subpath, equals that of an existing record whose version segment differs as written, the command SHALL report the existing record in the records directory as a superseded candidate.
- The command SHALL NOT delete or edit a superseded candidate.
- Where `--module <dir>` is given, the command SHALL look up the owning module only in the given module roots.
- The command SHALL print a deterministic report listing records created, records unchanged, superseded candidates, merged duplicates and components skipped.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-141-AC-1 | A CycloneDX and an SPDX fixture that list the same three components with purls each promote to three `soup_record` files that validate against the owning schema, each carrying `name` and `version` equal to its purl's segments and no `supplier`. | Test |
| FR-141-AC-2 | For a fixture with purl-bearing components, a repeated purl and a component with no purl, the report states components read, records created, records unchanged, merged duplicates and components skipped, and created + unchanged + merged + skipped equals components read. | Test |
| FR-141-AC-3 | A component with no purl writes no record and appears by name and version in the report; a component with neither purl nor name is dropped by the FR-041 adapter and is not reported as skipped. | Test |
| FR-141-AC-4 | A newly created record carries none of `serves`, `known_anomalies`, `support_status`, `end_of_support`, `supplier`; with `--discharges OB-1` naming a stated obligation it carries `disposition_clause: OB-1`, and without it no `disposition_clause` key. | Test |
| FR-141-AC-5 | Given an existing record whose reviewer-authored fields are populated, promoting the same document leaves the file byte-identical and the report lists it as unchanged. | Test |
| FR-141-AC-6 | After promoting a document whose component version was bumped, a new record exists for the new purl, the old record is byte-identical, and the report lists the old record as a superseded candidate. | Test |
| FR-141-AC-7 | Promoting the same document twice leaves the `--out` and `--records` directories byte-identical after the second run. | Test |
| FR-141-AC-8 | The command with the owning module absent exits non-zero naming the module and creates no file. | Test |
| FR-141-AC-9 | A document that is neither CycloneDX nor SPDX is refused with the FR-041-AC-4 error and writes no record. | Test |
| FR-141-AC-10 | A `--discharges` id no obligation states is reported `unmatched`, and the written records carry no `disposition_clause`. | Test |
| FR-141-AC-11 | With `--records` naming a directory that already holds a record for a purl in the document and `--out` naming another directory, no second record for that purl is written. | Test |
| FR-141-AC-12 | Two records in the records directory carrying one purl make the command exit non-zero naming both files and write nothing. | Test |
| FR-141-AC-13 | With no `--out`, new records land in `spec/soup/` of the repository, and the file name of a purl containing `/` and `@` contains no `/` and is distinct from the file name of every other purl in the fixture. | Test |
| FR-141-AC-14 | New records receive the ids `SOUP-001`, `SOUP-002` and so on in ascending purl order, numbering on from the highest `SOUP-` id already present in the records and `--out` directories. | Test |
| FR-141-AC-15 | A file already at the destination of a new record that is not the `soup_record` of that purl makes the command exit non-zero naming the file and write no record. | Test |
| FR-141-AC-16 | A component whose decoded name contains a control character is skipped, listed in the report, and writes no record. | Test |
| FR-141-AC-17 | The file name of a purl carrying qualifiers contains neither the qualifiers nor the subpath, and a record file that is not valid UTF-8 or exceeds 1 MiB below the records directory makes the command exit non-zero naming it. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-141-CON-1 | The command SHALL NOT contain an SBOM reader of its own; the reading is the FR-041 adapter's. | Design | Inspection |
| FR-141-CON-2 | The command SHALL NOT define a record type; the schema is the owning module's. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-041](./FR-041-sbom-inventory-evidence.md), [FR-046](./FR-046-clause-discharge.md); `agent-ix/spec-objects-security` FR-009, FR-010, FR-011.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
