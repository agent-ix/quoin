---
id: FR-144
title: "SOUP report: one row per record, unresolved and past-end-of-support first"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-143"
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

- When a user runs `quoin soup report --repo <repo> --as-of <instant>`, the command SHALL render one row for each `soup_record` in the repository.
- Each row SHALL state the component's purl, version, the requirements it serves, its known anomalies, support status, end-of-support date, its attached advisories and the disposition of each advisory.
- If a `soup_record` has no `serves`, then its row SHALL state `not stated` for an absent list and `none` for an empty list.
- If a field is absent or `unknown`, then its row SHALL state `unknown` or `not stated` as the record does.
- If a field is absent or `unknown`, its row SHALL NOT substitute a default.
- The command SHALL order rows with those carrying an unresolved advisory first, then those past end of support, then the rest, each group ordered by purl.
- The command SHALL append the unmatched advisories of FR-142 and the scans reported not run, refused or vacuous after the rows.
- If two `soup_record` files carry one purl, then the command SHALL refuse with an error naming both files.
- The command SHALL render Markdown by default and a JSON document with `--json`.
- The command SHALL read only the `soup_record` files, the retained scans, the stored discharge facts and the `--support` file.
- The command SHALL NOT read the SBOM document.
- The command SHALL write nothing into the repository.
- The command SHALL render the same bytes for the same inputs and `--as-of`.
- The command SHALL emit no score, percentage, rank or class.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-144-AC-1 | A repository with three fixture records yields three rows each carrying every listed column. | Test |
| FR-144-AC-2 | A record with `serves` absent, one with `serves: []` and one with ids render `not stated`, `none` and the ids respectively. | Test |
| FR-144-AC-3 | A record with an unresolved advisory sorts before a record past end of support, which sorts before a clean record; ties sort by purl. | Test |
| FR-144-AC-4 | A record with both an unresolved advisory and a past end-of-support date appears once, in the first group, and shows both flags. | Test |
| FR-144-AC-5 | Unmatched advisories, a not-run scan, a refused scan and a vacuous scan each appear after the rows with their reason. | Test |
| FR-144-AC-6 | Two records with one purl refuse the report naming both files. | Test |
| FR-144-AC-7 | Running the command twice with the same inputs and `--as-of` yields byte-identical Markdown and byte-identical JSON. | Test |
| FR-144-AC-8 | The command run in a repository with the SBOM document removed yields the same output. | Test |
| FR-144-AC-9 | After the command runs, no file in the repository is changed or created. | Test |
| FR-144-AC-10 | The JSON output has no key named or typed as a score, rank, class or percentage. | Test |
| FR-144-AC-11 | The row of a component with an unresolved advisory states the reason the advisory is unresolved, and the row of a resolved advisory states the disposition's decision and attester. | Test |

## Dependencies

- **Upstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-143](./FR-143-support-status-and-end-of-support-input.md), [FR-135](./FR-135-requirements-mapping-matrix-view.md); `agent-ix/spec-objects-security` FR-009.
- **Downstream**: none.
