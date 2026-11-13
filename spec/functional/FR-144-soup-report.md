---
id: FR-144
title: "SOUP report: one row per record, unresolved and past-end-of-support first"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-149"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-150"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-143"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-133"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-135"
    type: "references"
  - target: "ix://agent-ix/spec-objects-security/FR-009"
    type: "depends_on"
---

# FR-144: SOUP report: one row per record, unresolved and past-end-of-support first

## Description

`quoin soup report` renders the software-of-unknown-provenance report from the `soup_record` files of a
repository, the retained scans (FR-142) and, optionally, supplied support data (FR-143). It is a view, as
the matrix of FR-135 is: it persists nothing and adds no record type.

## Behavior

- When a user runs `quoin soup report --repo <repo> --binding <clause-binding-v1 report> --as-of <instant>`, the command SHALL render one row for each `soup_record` in the repository.
- Each row SHALL state the component's purl, version, the requirements it serves, its known anomalies, support status, end-of-support date, its attached advisories (from cargo-audit, cargo-deny and grype, each naming its tool) and the disposition of each advisory.
- Each row SHALL state its attached policy findings (licence and ban findings from `licence-policy` and `cargo-deny`, each naming its tool, rule id and severity word) and the disposition of each, apart from its advisories.
- Each row SHALL state its policy status as exactly one of `no policy findings in the scan read` (as FR-142 defines it), `policy findings not assessed: licence-policy not run`, `policy findings not assessed: licence-policy scan vacuous`, `policy findings not assessed: licence-policy scan unordered`, `not evaluated by the licence-policy scan read`, or the attached policy findings.
- Each row with a disposition SHALL state that the disposition covers the findings of the record reported by a scan at or before its attestation.
- Each row with a disposition SHALL show a finding reported by a later scan unresolved.
- The command SHALL count a record with an unresolved policy finding in the group of rows that carry an unresolved advisory.
- The command SHALL append unmatched policy findings and a `licence-policy` or `cargo-deny` scan reported not run, unordered or vacuous after the rows, in FR-142's states.
- The command SHALL NOT state a record `allowed`.
- The command SHALL render an `unknown` licence as a policy finding with its disposition, never as absent.
- Each row SHALL state its advisory status as `free of advisories` (as FR-142 defines it), `advisories not assessed: unsupported purl type`, or the attached advisories and the state of each blocking scan.
- If a record is `unsupported` under FR-142, its row SHALL NOT state `free of advisories`.
- The command SHALL place an `unsupported` record in the group of rows that are neither unresolved nor past end of support, ordered by purl.
- If a `soup_record` has an absent `serves` or `known_anomalies`, then its row SHALL state `not stated` for that field.
- If a `soup_record` has an empty `serves` or `known_anomalies`, then its row SHALL state `none` for that field.
- If a field is absent or `unknown`, then its row SHALL state `unknown` or `not stated` as the record does.
- If a field is absent or `unknown`, its row SHALL NOT substitute a default.
- The command SHALL order rows with those carrying an unresolved advisory first, then those past end of support, then the rest, each group ordered by purl.
- The command SHALL append the unmatched advisories of FR-142 and the scans reported not run, refused or vacuous, grype among them, after the rows.
- If two `soup_record` files carry one purl, then the command SHALL refuse with an error naming both files.
- If `--records` does not exist, then the command SHALL refuse with an error naming the path.
- If a `soup_record` file states no purl, then the command SHALL refuse with an error naming the file.
- If a record's `version`, or a known anomaly's `source_ref`, is not a string, then the command SHALL refuse with an error naming the record and the field.
- The command SHALL render Markdown by default and a JSON document with `--json`.
- The command SHALL read only the `soup_record` files below `--records <dir>` (default `spec/soup/`), the retained scans, the `--binding` report, the stored discharge facts and the `--support` file.
- The command SHALL NOT read the SBOM document.
- The command SHALL write nothing into the repository.
- The command SHALL render the same bytes for the same inputs and `--as-of`.
- The command SHALL emit no score, percentage, rank or class.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-144-AC-1 | A repository with three fixture records yields three rows each carrying every listed column. | Test |
| FR-144-AC-2 | A record with `serves` absent, one with `serves: []` and one with ids render `not stated`, `none` and the ids respectively, and `known_anomalies` renders `not stated` and `none` for absent and empty. | Test |
| FR-144-AC-3 | A record with an unresolved advisory sorts before a record past end of support, which sorts before a clean record; ties sort by purl. | Test |
| FR-144-AC-4 | A record with both an unresolved advisory and a past end-of-support date appears once, in the first group, and shows both flags. | Test |
| FR-144-AC-5 | Unmatched advisories, a not-run scan (including one whose log was refused at recording, FR-142), and a vacuous scan each appear after the rows with their reason. | Test |
| FR-144-AC-6 | Two records with one purl refuse the report naming both files. | Test |
| FR-144-AC-7 | Running the command twice with the same inputs and `--as-of` yields byte-identical Markdown and byte-identical JSON. | Test |
| FR-144-AC-8 | The command run in a repository with the SBOM document removed yields the same output. | Test |
| FR-144-AC-12 | Omitting `--binding` exits non-zero with a usage message, and a record whose `disposition_clause` is not in the binding report renders its advisories unresolved with that reason. | Test |
| FR-144-AC-9 | After the command runs, no file in the repository is changed or created. | Test |
| FR-144-AC-10 | The JSON output has no key named or typed as a score, rank, class or percentage. | Test |
| FR-144-AC-11 | The row of a component with an unresolved advisory states the reason the advisory is unresolved, and the row of a resolved advisory states the disposition's decision and attester. | Test |
| FR-144-AC-13 | `--records` naming a directory that does not exist exits non-zero with an error naming the path and prints no report. | Test |
| FR-144-AC-14 | A `soup_record` file with no purl exits non-zero with an error naming the file and prints no report. | Test |
| FR-144-AC-15 | A record whose `version` is a number, or whose anomaly `source_ref` is a number, exits non-zero naming the field and prints no report. | Test |
| FR-144-AC-16 | A fixture with an npm record carrying a grype advisory, resolved by an attested disposition, and a PyPI record carrying another, unresolved, renders both advisories on their own rows with tool `grype`, the resolved one carrying decision and attester and the unresolved one its reason. | Test |
| FR-144-AC-17 | A `lake` record states `advisories not assessed: unsupported purl type` in the third ordering group, never `free of advisories`; grype not run renders after the rows. | Test |
| FR-144-AC-18 | A fixture with an npm record carrying a `licence-denied` finding resolved by an attested disposition, a PyPI record carrying a `licence-unknown` finding unresolved, and an npm record carrying `package-banned`, renders each on its own row with tool, rule id and severity word, in a list apart from advisories, the unresolved ones sorting in the first group. | Test |
| FR-144-AC-19 | With no retained `licence-policy` scan, every row states `policy findings not assessed: licence-policy not run` and none states `no policy findings in the scan read`; with a vacuous scan, with an unordered scan, and for a record absent from `evaluated`, the row states the matching status of the list above; a finding for a purl with no record appears after the rows as unmatched. | Test |
| FR-144-AC-20 | A record with an advisory attested before a later `licence-policy` ban finding renders the advisory resolved, the ban finding unresolved with `scan newer than attestation`, and the stated disposition scope. | Test |

## Dependencies

- **Upstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-143](./FR-143-support-status-and-end-of-support-input.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md); `agent-ix/spec-objects-security` FR-009.
- **Context only**: [FR-135](./FR-135-requirements-mapping-matrix-view.md), whose matrix view this report follows.
- **Downstream**: none.
