---
id: FR-143
title: "Support status and end-of-support dates come from supplied public data"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-141"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-046"
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

The input file is a JSON array of objects `{purl, support_status, end_of_support, source}`. `purl` is a package URL
or a purl without version for an ecosystem-wide statement; `support_status` is a member of
`SoupSupportStatus`; `end_of_support` is a calendar date or `unknown`; `source` is a public URL or citation.

An entry matches a record when the type, namespace and name segments of the two purls, each percent-decoded, are
byte-equal (no case folding or type-specific normalisation), qualifiers and subpath are ignored, and the entry carries either no version or the record's version. Two entries are duplicates when they
are equal in type, namespace, name and version (an absent version equals only an absent version).

## Behavior

- When a report is generated with `--support <file>`, the command SHALL validate every entry before using any.
- If the file is not a JSON array of objects, then the command SHALL refuse the file.
- If an entry lacks `purl`, `support_status`, `end_of_support` or `source`, or its `source` is empty, or carries a status outside `SoupSupportStatus`, or an `end_of_support` that is neither a calendar date nor `unknown`, then the command SHALL refuse the file naming the entry.
- If an entry carries a key other than `purl`, `support_status`, `end_of_support` and `source`, then the command SHALL refuse the file naming the entry and the key.
- If two entries are duplicates, then the command SHALL refuse the file naming both.
- Where a record's `support_status` or `end_of_support` is stated and not `unknown`, the report SHALL show the record's value as the authoritative value.
- Where a record's value is `unknown` or absent and an entry matches the record's purl, the report SHALL show the entry's value labelled with its `source`.
- Where a record's value is `unknown` or absent and an entry matches the record's purl, the report SHALL NOT write it into the record.
- Where an entry states `unknown` for a field, the report SHALL treat the entry as stating nothing for that field, so that it neither fills a record's `unknown` or absent value nor disagrees with a stated one.
- If a record's `support_status` or `end_of_support` is outside its `agent-ix/spec-objects-security` FR-010 vocabulary, then the command SHALL refuse naming the record.
- If a record's `support_status` or `end_of_support` is outside its vocabulary, the command SHALL NOT treat the value as `unknown`.
- Where a version-less entry and a versioned entry both match, the report SHALL use the versioned entry.
- If a stated record value and a supplied entry disagree, then the report SHALL show both and flag the disagreement.
- If neither the record nor an entry states a value, then the report SHALL show `unknown`.
- If neither the record nor an entry states a value, the report SHALL NOT infer a value from release age or any other signal.
- If the UTC calendar date of the explicit `asOf` instant is after a component's end-of-support date, then the report SHALL flag the component as past end of support.
- If the UTC calendar date of `asOf` equals or precedes the end-of-support date, the report SHALL NOT flag the component as past end of support.
- If a component's end-of-support date is `unknown`, then the report SHALL NOT flag it as past end of support or as within support.
- The command SHALL take the `--as-of` value, when it is a calendar date, as that UTC date, and when it is an RFC 3339 instant, as its UTC calendar date.
- The command SHALL NOT read the wall clock; the comparison date is the `--as-of` value (FR-046).
- The command SHALL make no network request to obtain support data.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-143-AC-1 | An entry with no or empty `source`, an invalid status, an `end_of_support` of `soon`, or a duplicate each refuses the whole file, names the entry, and produces no report. | Test |
| FR-143-AC-2 | A record with `end_of_support: unknown` and a matching entry shows the entry's date labelled with its source, and the record file is unchanged. | Test |
| FR-143-AC-3 | A record with a stated date and a different entry date shows both, flags the disagreement, and treats the record's date as authoritative. | Test |
| FR-143-AC-4 | A record with `unknown` status and no matching entry shows `unknown` for both fields and carries no past-end-of-support flag. | Test |
| FR-143-AC-5 | With `--as-of 2026-06-01`, a component whose end of support is 2026-05-31 is flagged past end of support and one whose date is 2026-06-01 or later is not. | Test |
| FR-143-AC-6 | A versioned entry takes precedence over a version-less entry for the same package; a version-less entry matches a record of any version, and an entry for a different version matches nothing; `pkg:cargo/a` and `pkg:cargo/a@1` in one file are not duplicates. | Test |
| FR-143-AC-9 | With `--as-of 2026-05-31T12:00:00Z` a component ending 2026-05-31 is not flagged, and with `--as-of 2026-06-01T00:00:00Z` it is flagged; an `--as-of` with a non-UTC offset is compared by its UTC date. | Test |
| FR-143-AC-10 | A record with `support_status: maintained` refuses the report naming the record. Not yet backed: the `quoin soup report` command that tests it is PLAT-1518. | Test |
| FR-143-AC-7 | The command run with no network available produces the same output as with it. | Test |
| FR-143-AC-8 | Two runs with different wall-clock times and the same `--as-of` produce identical output. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-143-CON-1 | The author of the support file SHALL cite only public sources; the command checks only that each `source` is present and non-empty. | Process | Inspection |

## Dependencies

- **Upstream**: [FR-141](./FR-141-promote-sbom-components-to-soup-records.md), [FR-046](./FR-046-clause-discharge.md); `agent-ix/spec-objects-security` FR-010.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
