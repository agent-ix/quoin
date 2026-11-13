---
id: FR-167
title: "Release snapshot format: a closed, ordered JSON document the dossier documents read"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-166"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-163"
    type: "references"
---

# FR-167: Release snapshot format: a closed, ordered JSON document the dossier documents read

## Description

The release snapshot written by `quoin release snapshot` ([FR-166](./FR-166-release-snapshot-command.md)) is one
canonical JSON document (`protocol::canonical_json`, as [FR-115](./FR-115-evidence-backed-test-matrix.md) uses)
with the protocol `quoin.release-snapshot/v1`. Its schema is closed: every object admits only the members named
here. The dossier documents read the same file that the release commit carries; the dossier export that
[FR-162](./FR-162-anomaly-report.md) feeds reads the snapshot's `anomalies` and the other sections without
running Quoin again.

Each section holds values that another requirement already defines. This requirement names the member and the
source and restates no vocabulary.

| Member | Content | Source |
|--------|---------|--------|
| `protocol` | the string `quoin.release-snapshot/v1` | this requirement |
| `as_of` | the `--as-of` instant, as supplied | FR-166 |
| `baseline_commit` | the `--baseline-commit` hash, as supplied | FR-166 |
| `previous_release` | `{ tag, commit }` of the previous release, or `null` for a first release | FR-168 |
| `coverage` | per requirement, per criterion: `id`, `static_status`, `evidence_status` | FR-115 |
| `evidence_freshness` | the count of criteria at each of the five evidence statuses | FR-115 |
| `open_findings` | each auditor finding of every criterion: `obligation`, `kind`, `summary`, `path`, `line`, `symbol` | FR-115 `evidence_detail.findings` |
| `soup` | `rows`: per record `purl`, `version`, `group`, `support_status`, `end_of_support`; and `not_assessed`: each item FR-144 appends after its rows, as `{ kind, tool, detail }` | FR-144 |
| `anomalies` | each carried anomaly: `id`, `title`, `status`, `resolution`, `severity`, `safety_impact`, `unassessed`, `release_rationale` | FR-162, FR-163 |
| `change_records` | the change-assurance records since the previous release | FR-168 |

`group` is one of `unresolved_advisory`, `past_end_of_support`, `other`, the three ordering groups of FR-144.
`kind` of a `not_assessed` item is one of `unmatched_advisory`, `unmatched_policy_finding`, `scan_not_run`,
`scan_refused`, `scan_vacuous`, `scan_unordered`; `tool` names the scanner (`grype`, `cargo-audit`, `cargo-deny`,
`licence-policy`); `detail` is the identifying text FR-144 states for the item (the advisory or finding id and purl,
or the reason). `evidence_freshness` has exactly the five keys `bound`, `stale`, `suspect`, `undischarged`, `no run evidence`.
`release_rationale` is the list of `{ author, rationale }` entries of FR-163 as authored. The snapshot carries
neither `fixingChange` (FR-136) nor `rationaleChange` (FR-163), because both derive from git history and the command
runs no git ([FR-166](./FR-166-release-snapshot-command.md)).

## Behavior

- The snapshot SHALL carry exactly the members of the table, each once, with `null` for a source value that is absent.
- The snapshot SHALL NOT carry a member that the table does not name, at any depth.
- The command SHALL order `coverage` by requirement id and then as FR-115 orders criteria.
- The command SHALL order `open_findings` by `obligation`, `kind`, `path`, `line`, `symbol` and `summary`, comparing
  strings bytewise, `line` numerically and an absent value before a present one.
- The command SHALL order `soup.rows` as FR-144 orders its rows.
- The command SHALL order `soup.not_assessed` by `kind`, `tool` and `detail`, comparing bytewise.
- The command SHALL order `anomalies` by anomaly id.
- The command SHALL order `change_records` as FR-168 states.
- The command SHALL carry each value as its source requirement states it, without a default or a derived grade.
- If a section's source is empty, then the command SHALL write the section as an empty list or zero counts, never omit it.
- The command SHALL NOT include a digest, hash, manifest, file listing or version counter of the snapshot or of
  any item it describes. The `baseline_commit` and the commits in `change_records` are the identifiers git gave the
  caller, not values Quoin computed.
- The command SHALL NOT include a score, percentage, rank, verdict or the word accepted, ready or releasable.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-167-AC-1 | A snapshot built from a fixture validates against the closed schema, and the same snapshot with one extra member at the top level, inside a coverage row, a finding, a SOUP row, an anomaly and a change record is refused by the schema each time. | Test |
| FR-167-AC-2 | `evidence_freshness` has exactly the five keys and no other, and its counts sum to the number of criteria in `coverage`. | Test |
| FR-167-AC-3 | A fixture with a `stale`, a `suspect` and a `bound` criterion carries each at its status in `coverage`, and the auditor findings of the first two appear in `open_findings`; a fixture with two `surviving-mutant` findings on one obligation that differ only in `line`, and two that differ only in `summary`, yields them in `path`, `line`, `symbol`, `summary` order whatever order the audit report lists them. | Test |
| FR-167-AC-4 | A fixture with an unresolved-advisory record, a past-end-of-support record and a clean record yields `soup.rows` in that group order with `group` set to `unresolved_advisory`, `past_end_of_support` and `other`, and a scan reported not run, a scan reported unordered, an advisory matching no record and a policy finding matching no record each appear in `soup.not_assessed` with `kind`, `tool` and `detail`, in `kind`, `tool`, `detail` order. | Test |
| FR-167-AC-5 | A fixture with an open anomaly carrying a two-row release rationale, a `closed` `wont-fix` one and an `unassessed` one lists all three by id, with `release_rationale` byte-equal to the record's entries, and no snapshot member named `rationaleChange`. | Test |
| FR-167-AC-6 | An empty evidence store yields `coverage` with every criterion `no run evidence` and `open_findings` holding one `undischarged` finding for each criterion, as FR-032-AC-5 reports and FR-115-AC-3 keeps in `evidence_detail`; the other sections are present, never absent. | Test |
| FR-167-AC-9 | A fixture anomaly file with git history that would give `fixingChange` and `rationaleChange` yields an anomaly row with neither member. | Test |
| FR-167-AC-7 | A scan of the snapshot's keys and values finds no digest, hash, manifest or score member, and no sentence stating a release accepted, ready or releasable. | Inspection |

## Dependencies

- **Upstream**: [FR-115](./FR-115-evidence-backed-test-matrix.md), [FR-144](./FR-144-soup-report.md),
  [FR-162](./FR-162-anomaly-report.md), [FR-163](./FR-163-anomaly-report-release-rationale.md),
  [FR-166](./FR-166-release-snapshot-command.md), [FR-168](./FR-168-release-snapshot-change-records.md).
- **Downstream**: the dossier documents, which read the snapshot.
