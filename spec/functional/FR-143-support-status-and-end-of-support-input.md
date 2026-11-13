---
id: FR-143
title: "Support status and end-of-support dates come from supplied public data"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-141"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-010"
    type: "depends_on"
---

# FR-143: Support status and end-of-support dates come from supplied public data

## Description

A `soup_record` states `support_status` and `end_of_support` (`agent-ix/spec-objects-security` FR-010),
each of which may be `unknown`. `quoin` does not fetch support data from the network. A caller supplies a
file of support statements taken from public sources (a project's published support policy, its release
page, or a public end-of-life listing), and the report reads it. Each statement names its public source,
so a reader can check it. The input is read per invocation, as `--facts` is in FR-046.

The input file is a list of entries `{purl, support_status, end_of_support, source}`. `purl` is a package URL
or a purl without version for an ecosystem-wide statement; `support_status` is a member of
`SoupSupportStatus`; `end_of_support` is a calendar date or `unknown`; `source` is a public URL or citation.

## Behavior

- When a report is generated with `--support <file>`, the command SHALL validate every entry before using any.
- If an entry lacks `purl`, `support_status` or `source`, or carries a status outside `SoupSupportStatus`, then the command SHALL refuse the file naming the entry.
- If two entries name the same purl, then the command SHALL refuse the file naming both.
- Where a record's `support_status` or `end_of_support` is stated and not `unknown`, the report SHALL show the record's value as the authoritative value.
- Where a record's value is `unknown` or absent and an entry matches the record's purl, the report SHALL show the entry's value labelled with its `source`.
- Where a record's value is `unknown` or absent and an entry matches the record's purl, the report SHALL NOT write it into the record.
- Where a version-less entry and a versioned entry both match, the report SHALL use the versioned entry.
- If a stated record value and a supplied entry disagree, then the report SHALL show both and flag the disagreement.
- If neither the record nor an entry states a value, then the report SHALL show `unknown`.
- If neither the record nor an entry states a value, the report SHALL NOT infer a value from release age or any other signal.
- If the report's explicit `asOf` instant is later than a component's end-of-support date, then the report SHALL flag the component as past end of support.
- If a component's end-of-support date is `unknown`, then the report SHALL NOT flag it as past end of support or as within support.
- The command SHALL NOT read the wall clock; the comparison date is the `--as-of` value (FR-046).
- The command SHALL make no network request to obtain support data.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-143-AC-1 | An entry with no `source`, an invalid status, or a duplicate purl each refuses the whole file, names the entry, and produces no report. | Test |
| FR-143-AC-2 | A record with `end_of_support: unknown` and a matching entry shows the entry's date labelled with its source, and the record file is unchanged. | Test |
| FR-143-AC-3 | A record with a stated date and a different entry date shows both, flags the disagreement, and treats the record's date as authoritative. | Test |
| FR-143-AC-4 | A record with `unknown` status and no matching entry shows `unknown` for both fields and carries no past-end-of-support flag. | Test |
| FR-143-AC-5 | With `--as-of 2026-06-01`, a component whose end of support is 2026-05-31 is flagged past end of support and one whose date is 2026-06-01 or later is not. | Test |
| FR-143-AC-6 | A versioned entry takes precedence over a version-less entry for the same package. | Test |
| FR-143-AC-7 | The command run with no network available produces the same output as with it. | Test |
| FR-143-AC-8 | Two runs with different wall-clock times and the same `--as-of` produce identical output. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-143-CON-1 | The command SHALL NOT accept support data from a paid or private feed as a source; a `source` is a public reference. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-141](./FR-141-promote-sbom-components-to-soup-records.md), [FR-046](./FR-046-clause-discharge.md); `agent-ix/spec-objects-security` FR-010.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
