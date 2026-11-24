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
the other module's record from the components the FR-041 reader already returns. `quoin` does not parse an
SBOM a second time and does not generate one. The command uses the component symbol, which is the purl where the
SBOM states one and `name@version` otherwise, and the supplier and declared licences that the reader exposes
(FR-041-AC-10, FR-041-AC-11, FR-041-AC-12).

A record's `name` and `version` are the percent-decoded name and version segments of its purl. Its `supplier` and
`license` are what the SBOM states, as written. They are facts the SBOM supplies, not reviewer statements, so a
created record carries them and a person amends them afterwards.

**Supplier.** The record's `supplier` is the component's declared supplier as the reader exposes it. Where the SBOM
states none, the record leaves `supplier` absent (not stated); the command does not invent the literal `unknown`. Where the SBOM itself states `unknown`, that is a stated value and is written as written.

**Licence.** The record's `license` is a single string, so it carries a licence only where the component declares exactly
one readable licence. The text is written as the document wrote it, whether the reader classed it as an `id`, a `name` or an
`expression`; nothing is parsed, normalised or mapped. Where the component declares none, `license` is absent. Where it
declares more than one, `license` is absent and the component is listed in the report with the declared texts, because
joining them would state an operator the SBOM did not. A readable licence beside an unreadable one counts as more than one; the report lists the readable texts and the placeholder `unreadable` for each unreadable entry. Where its only licence is `unreadable` (FR-041-AC-12), `license`
is absent and the component is listed in the report, so no SBOM fragment becomes licence text. A created record never carries `license_fact_ref`, so the
owning module's exclusivity of `license` and `license_fact_ref` (FR-009 there) holds; relating a record to an integrator
licence fact stays a person's statement.

**Duplicates.** Where two components of one document carry one purl, the record carries the supplier and licence of the
first occurrence in document order. Where a later occurrence states a different supplier or licence, the report lists the
conflict.

**Existing records.** Promotion never overwrites a field of an existing record. The supplier the SBOM states for a purl is
the exposed supplier, or none; the licence it states is the single readable licence, or none (where the component
declares none, several, an unreadable licence or a licence with a control character). Where an existing record's
`supplier` or `license` differs from that, including a field the record sets while the SBOM states none and a field the
SBOM states while the record sets none, the command reports the difference and leaves the file untouched.

Promotion does not hold that the record count equals the SBOM component count. A component
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
- The command SHALL write `supplier` on a record it creates as the declared supplier text the FR-041 reader exposes, as written.
- Where the reader exposes no supplier for a component, the command SHALL leave `supplier` absent on the record it creates.
- Where the reader exposes exactly one declared licence for a component and its kind is `id`, `name` or `expression`, the command SHALL write `license` on the record it creates as that text, as written, for each of the three kinds.
- Where the reader exposes no declared licence for a component, the command SHALL leave `license` absent on the record it creates.
- Where the reader exposes more than one declared licence for a component, the command SHALL leave `license` absent on the record it creates.
- Where the reader exposes more than one declared licence for a component, the command SHALL list the component in the report with its readable licence texts and the placeholder `unreadable` for each unreadable entry.
- If a supplier text contains a control character, then the command SHALL omit `supplier` from the record it creates.
- If a licence text contains a control character, then the command SHALL omit `license` from the record it creates.
- If a supplier or licence text contains a control character, then the command SHALL list the component and field in the report.
- Where the only licence a component declares is `unreadable`, the command SHALL leave `license` absent on the record it creates.
- Where the only licence a component declares is `unreadable`, the command SHALL list the component in the report.
- If two components of one document carry one purl, then the record SHALL carry the supplier and licence of the first occurrence in document order.
- If a later occurrence of a purl states a supplier or licence different from the first occurrence's, then the command SHALL list the conflict in the report.
- The command SHALL NOT write `license_fact_ref` on a record it creates.
- Where the SBOM states no supplier or licence, the command SHALL NOT write the literal `unknown` for it.
- The command SHALL NOT write `serves`, `known_anomalies`, `support_status` or `end_of_support` on a record it creates.
- Where `--discharges <obligation-id>` is not given, the command SHALL NOT write `disposition_clause` on a record it creates.
- Where `--discharges <obligation-id>` is given and an obligation states that id, the command SHALL write the id verbatim as `disposition_clause`.
- If `--discharges` is given and the promotion plans to create more than one new `soup_record`, then the command SHALL refuse before writing any record; it SHALL name the multi-component promotion refusal and SHALL NOT accept a mapping format.
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
- Where an existing record's `supplier` differs from the supplier the SBOM states for its purl, the command SHALL list the record and field in the report as a difference.
- Where an existing record states a `supplier` and the SBOM states none for its purl, the command SHALL list the record and field in the report as a difference.
- Where the SBOM states a supplier for a purl and the existing record states none, the command SHALL list the record and field in the report as a difference.
- Where an existing record's `license` differs from the licence the SBOM states for its purl, the command SHALL list the record and field in the report as a difference.
- Where an existing record states a `license` and the SBOM states none for its purl, the command SHALL list the record and field in the report as a difference.
- Where the SBOM states a licence for a purl and the existing record states none, the command SHALL list the record and field in the report as a difference.
- Where an existing record carries `license_fact_ref`, the command SHALL NOT report a licence difference for it.
- If two records in the records directory carry one purl, then the command SHALL exit non-zero naming both files.
- When the same document is promoted twice, the second run SHALL change no file.
- Where a new record's purl, without version, qualifiers and subpath, equals that of an existing record whose version segment differs as written, the command SHALL report the existing record in the records directory as a superseded candidate.
- The command SHALL NOT delete or edit a superseded candidate.
- Where `--module <dir>` is given, the command SHALL look up the owning module only in the given module roots.
- The command SHALL print a deterministic report listing records created, records unchanged, superseded candidates, merged duplicates, components skipped, field differences, components with multiple or unreadable licences, duplicate conflicts and omitted fields.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-141-AC-1 | A CycloneDX and an SPDX fixture that list the same three components with purls each promote to three `soup_record` files that validate against the owning schema, each carrying `name` and `version` equal to its purl's segments. | Test |
| FR-141-AC-2 | For a fixture with purl-bearing components, a repeated purl and a component with no purl, the report states components read, records created, records unchanged, merged duplicates and components skipped, and created + unchanged + merged + skipped equals components read. | Test |
| FR-141-AC-3 | A component with no purl writes no record and appears by name and version in the report; a component with neither purl nor name is dropped by the FR-041 adapter and is not reported as skipped. | Test |
| FR-141-AC-4 | A newly created record carries none of `serves`, `known_anomalies`, `support_status`, `end_of_support`, `license_fact_ref`; with `--discharges OB-1` naming a stated obligation it carries `disposition_clause: OB-1`, and without it no `disposition_clause` key. | Test |
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
| FR-141-AC-18 | A CycloneDX and an SPDX fixture whose components state a supplier and exactly one licence (one as an id, one as a name, one as an expression) promote to records whose own text carries a `supplier` and a `license` line equal to the stated texts exactly. | Test |
| FR-141-AC-19 | A component with no supplier and no licence, one with an SPDX `NOASSERTION` supplier and licence, and one with an empty supplier name yield records with no `supplier` key and no `license` key, and none carries the literal `unknown`; a component whose SBOM states the supplier `unknown` yields a record carrying `supplier: unknown`. | Test |
| FR-141-AC-20 | A component declaring two licences yields a record with no `license` key and appears in the report with both declared texts; a component with one readable and one unreadable licence yields no `license` key and appears with the readable text and `unreadable`; the `supplier` is still written in both. | Test |
| FR-141-AC-21 | A component whose supplier contains a control character yields a record whose text has no `supplier` line, and the report lists the component and field. | Test |
| FR-141-AC-22 | Given an existing record whose `supplier` and `license` a person set to values different from the SBOM's, promoting the document leaves the file byte-identical and the report lists both fields as differences; an existing record equal to the SBOM's values is listed with no difference; an existing record carrying `license_fact_ref` yields no licence difference. | Test |
| FR-141-AC-23 | An existing record that sets a `supplier` while the SBOM states none for its purl, and one that sets a `license` while the SBOM declares no licence, two licences, an unreadable licence or a licence containing a control character, are each listed as a difference and left byte-identical; an existing record that sets neither, for an SBOM component that states a supplier and a licence, is listed as a difference for both fields. | Test |
| FR-141-AC-24 | A component whose supplier and licence text contain a control character yields a record whose text has neither a `supplier` nor a `license` line, and the report lists both fields; a component whose only licence entry is unreadable yields a record with no `license` line and appears in the report. | Test |
| FR-141-AC-25 | A document listing one purl twice with different suppliers and licences yields one record carrying the first occurrence's values, and the report lists the conflict. | Test |
| FR-141-AC-26 | Given an SBOM that plans to create more than one `soup_record`, `quoin soup promote --discharges OB-1` refuses before writing any record, even if `OB-1` is unstated; the command offers no per-component mapping form. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-141-CON-1 | The command SHALL NOT contain an SBOM reader of its own; the reading, including the supplier and licences, is the FR-041 reader's. | Design | Inspection |
| FR-141-CON-2 | The command SHALL NOT define a record type; the schema is the owning module's. | Design | Inspection |
| FR-141-CON-3 | The command SHALL NOT parse, normalise, map or join licence text. | Design | Inspection |
| FR-141-CON-4 | The command SHALL NOT overwrite a field of an existing record. | Design | Test |

## Dependencies

- **Upstream**: [FR-041](./FR-041-sbom-inventory-evidence.md) (reader, supplier and licences), [FR-046](./FR-046-clause-discharge.md); `agent-ix/spec-objects-security` FR-009, FR-010, FR-011.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
