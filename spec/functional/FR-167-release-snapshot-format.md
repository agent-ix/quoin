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
  - target: "ix://agent-ix/quoin/FR-164"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-170"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-032"
    type: "depends_on"
---

# FR-167: Release snapshot format: a closed, ordered JSON document the dossier documents read

## Description

The release snapshot written by `quoin release snapshot` ([FR-166](./FR-166-release-snapshot-command.md)) is one
canonical JSON document (`protocol::canonical_json`, as [FR-115](./FR-115-evidence-backed-test-matrix.md) uses)
with the protocol `quoin.release-snapshot/v1`. Its schema is closed: every object admits only the members named
here. The `tool_identity` and `clean_tree_precondition` members are required in every snapshot; a reader refuses a
snapshot that omits either. The dossier
documents read the same file that the release commit carries; the dossier export that
[FR-162](./FR-162-anomaly-report.md) feeds reads the snapshot's `anomalies` and the other sections without
running Quoin again.

Each section holds values that another requirement already defines. This requirement names the member and the
source and restates no vocabulary.

| Member | Content | Source |
|--------|---------|--------|
| `protocol` | the string `quoin.release-snapshot/v1` | this requirement |
| `tool_identity` | `{ name, version }` identifying the Quoin executable that wrote this snapshot | FR-166 / this requirement |
| `clean_tree_precondition` | string `not_verified`, stating that Quoin did not verify the caller's clean-tree precondition | FR-166 / this requirement |
| `as_of` | the `--as-of` instant, as supplied | FR-166 |
| `baseline_commit` | the `--baseline-commit` hash, as supplied | FR-166 |
| `previous_release` | `{ tag, commit }` of the previous release, or `null` for a first release | FR-168 |
| `coverage` | per requirement: `id` and `criteria`, a list holding for each criterion `id`, `static_status` and `evidence_status` | FR-115 |
| `evidence_freshness` | the count of criteria at each of the five evidence statuses | FR-115 |
| `open_findings` | each open finding, as one item of the closed shape `obligation`, `kind`, `summary`, `path`, `line`, `symbol`, and for a `stale-evidence` finding the optional `cause` (`missing-run` or `failed-run` when present), omitted when absent and omitted on every other kind. Three sources contribute items: the auditor findings of every criterion, the risk-completeness findings, and the anomaly-gate findings, each mapped as stated below | FR-115 `evidence_detail.findings`; FR-170; FR-164 |
| `soup` | `rows`: per record `purl`, `version`, `group`, `support_status`, `end_of_support`; and `not_assessed`: each item FR-144 appends after its rows, as `{ kind, tool, detail }` | FR-144 |
| `vulnerabilities` | Each vulnerability record whose status is `open` or `in_remediation`: `id`, `cve_id`, `severity`, `affects`, `references`, `status`, `disposition`, `disposition_by`, and present `discovered_at`, `disclosed_at`, and `fixed_at` values, copied as authored | spec-objects-security FR-032 |
| `anomalies` | each carried anomaly: `id`, `title`, `status`, `resolution`, `severity`, `safety_impact`, `unassessed`, `release_rationale` | FR-162, FR-163 |
| `change_records` | the change-assurance records since the previous release | FR-168 |

Open findings come from three sources, told apart by `kind`; the three sets of kinds are disjoint.

| Source | `kind` | `obligation` | `summary` | `path` | `line`, `symbol`, `cause` |
|--------|--------|--------------|-----------|--------|---------------------------|
| auditor finding (FR-115) | the auditor kind | the criterion's obligation | the finding's summary | as FR-115 supplies | as FR-115 supplies |
| risk-completeness finding (FR-170) | the FR-170 `finding` token, one of the thirteen tokens FR-170 defines | the first element of the finding's `names` | the finding's `names` joined by `, ` | `null` | `null`, `null`, omitted |
| anomaly-gate finding (FR-164) | `anomaly-unassessed` for a failure of mode `unassessed`, `anomaly-unjustified-impacting` for mode `unjustified-impacting` | the anomaly id | the mode word of the failure | the failure's `path`, repository-relative | `null`, `null`, omitted |

The risk-completeness findings are those that `quoin risk-summary` (FR-169) places in the `findings` of its rows,
computed by the same code. A finding that FR-170 places on several rows is one item, identified by its token and its
`names`. The anomaly-gate findings are the failures that `quoin anomaly report --fail-on unassessed --fail-on
unjustified-impacting` (FR-164) states in `gate.failures`, evaluated over the anomalies the snapshot carries; each
gate failure is one item, and an anomaly fails at most one mode. A finding states that a link, a record or a rationale is missing. It
does not say that a risk or an anomaly is acceptable: people decide that, and the snapshot never does. An `open_findings` list holding
none of these three sources states that no finding exists in them and is not an acceptance.

`group` is not a member of FR-144's JSON; it is the row's FR-144 ordering group, and `unassessed` is the FR-162 row flag
(true exactly when `safety_impact` is `unassessed`). `group` is one of `unresolved_advisory`, `past_end_of_support`, `other`, the three ordering groups of FR-144.
`kind` of a `not_assessed` item is one of `unmatched_advisory`, `unmatched_policy_finding`, `scan_not_run`,
`scan_vacuous`; a scan with an invalid retained timestamp refuses the snapshot rather than appearing as a kind,
and a reader refuses any other `kind`; `tool` names the scanner (`osv-scanner`, `cargo-audit`, `cargo-deny`,
`licence-policy`); `detail` is defined per `kind`: for `unmatched_advisory`, the advisory id as the scan reports it, a `|`, and the purl the
advisory names (empty when it names none); for `unmatched_policy_finding`, the rule id, a `|`, and the purl the
finding names (empty when it names none); for the two `scan_` kinds, the state word `not run` or `vacuous`
that FR-144 and FR-142 use for that state. FR-144 renders a scan log refused at recording as `not run`, so no separate refused kind exists. `evidence_freshness` has exactly the five keys `bound`, `stale`, `suspect`, `undischarged`, `no run evidence`.
`release_rationale` is the list of `{ author, rationale }` entries of FR-163 as authored. The snapshot carries
neither `fixingChange` (FR-136) nor `rationaleChange` (FR-163), because both derive from git history and the command
runs no git ([FR-166](./FR-166-release-snapshot-command.md)).
`tool_identity.name` is `quoin` and `tool_identity.version` is the exact version string printed by the
running executable for `quoin --version`. Every snapshot includes this member and
also `clean_tree_precondition` with the literal value `not_verified`; Quoin does not inspect git state and
does not claim the caller's clean-tree precondition was met.

## Behavior

- The snapshot SHALL carry exactly the members of the table, each once, with `null` for an absent source value.
- A reader SHALL refuse a snapshot that omits `tool_identity` or `clean_tree_precondition`.
- A reader SHALL validate the closed shape and allowed values of each of those members, including refusing `null`.
- A reader SHALL refuse a `soup.not_assessed` item whose `kind` is not one of `unmatched_advisory`,
  `unmatched_policy_finding`, `scan_not_run` or `scan_vacuous`.
- If a retained scan has a timestamp that is not an instant, then the command SHALL refuse the snapshot, as FR-166
  states, and SHALL NOT write a `scan_unordered` item.
- The command SHALL include `tool_identity` and `clean_tree_precondition` in every snapshot.
- The command SHALL set `tool_identity.name` to `quoin` and `tool_identity.version` to the version string its own
  executable prints for `quoin --version`.
- The command SHALL set `clean_tree_precondition` to `not_verified`.
- The command SHALL NOT inspect git state to verify the clean-tree precondition.
- The command SHALL add to `open_findings` one item for each distinct risk-completeness finding of `quoin risk-summary` over `--repo`, with `obligation` the first element of `names`, `kind` the finding token, `summary` the `names` joined by `, `, `path`, `line` and `symbol` `null` and no `cause`.
- The command SHALL add to `open_findings` one item for each failure of FR-164's `unassessed` and `unjustified-impacting` modes over the carried anomalies, with `obligation` the anomaly id, `kind` `anomaly-unassessed` or `anomaly-unjustified-impacting`, `summary` the mode word, `path` the failure's path as a repository-relative path (never absolute), `line` and `symbol` `null` and no `cause`.
- The command SHALL NOT add to `open_findings` an item for a hazard, situation, control, estimate or residual risk that has the link or record the FR-170 rule asks for, or for an anomaly that passes both gate modes.
- The command SHALL NOT assign a severity, score, class or acceptability to any `open_findings` item.
- The command SHALL NOT omit an `open_findings` item because a person accepted the risk it names.
- WHEN serializing `open_findings`, the command SHALL omit the `cause` member when it is absent.
- The command SHALL NOT serialize absent `cause` as `null`.
- The snapshot SHALL NOT carry a member that the table does not name, at any depth.
- The command SHALL order `coverage` by requirement id and then as FR-115 orders criteria.
- The command SHALL order `open_findings` by `obligation`, `kind`, `path`, `line`, `symbol`, `cause` and `summary`, comparing
  strings bytewise, `line` numerically and an absent value before a present one.
- The command SHALL carry a `stale-evidence` finding's `cause` into `open_findings` exactly as FR-115 supplies it, including omitting it when FR-115 omits it.
- The command SHALL omit `cause` from a finding whose kind is not `stale-evidence`.
- The command SHALL omit `cause` from a `stale-evidence` finding when FR-115 supplies that finding without `cause`.
- The command SHALL NOT derive `cause` from `summary`.
- The command SHALL include in `vulnerabilities` each selected vulnerability record whose `status` is `open` or `in_remediation`, with every field in the table copied from its authored record without deriving or defaulting a value.
- A reader SHALL interpret a missing `vulnerabilities` member in a `quoin.release-snapshot/v1` document as an empty list. This covers snapshots written before the member was added; a present member SHALL still have the declared list shape.
- The command SHALL omit a selected vulnerability record whose `status` is `remediated` or `closed`.
- The command SHALL carry each included vulnerability's `status` as `open` or `in_remediation`, using the vocabulary defined by spec-objects-security FR-032.
- The command SHALL order `vulnerabilities` by record id.
- The command SHALL NOT infer vulnerability status, disposition, owner, discovery date, disclosure date, or fixed date from an advisory scan, VEX statement, anomaly, or other analysis.
- The command SHALL order `soup.rows` as FR-144 orders its rows.
- The command SHALL order `soup.not_assessed` by `kind`, `tool` and `detail`, comparing bytewise.
- The command SHALL order `anomalies` by anomaly id.
- The command SHALL order `change_records` as FR-168 states.
- The command SHALL carry each value as its source requirement states it, without a default or a derived grade.
- If a section's source is empty, then the command SHALL write the section as an empty list or zero counts, never omit it.
- The command SHALL NOT include a digest, hash, manifest, file listing or version counter of the snapshot or of
  any item it describes. The `baseline_commit` and the commits in `change_records` are the identifiers git gave the
  caller, not values Quoin computed.
- The command SHALL NOT include a score, percentage, rank, verdict or the word accepted, ready or releasable; the stem "accept" inside a finding token of `open_findings` (`accepted-outside-acceptable-region`, `benefit-risk-not-accepted`, `not-accepted-inside-acceptable-region`) and inside a value copied from a record is not counted.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-167-AC-1 | A snapshot built from a fixture validates against the closed schema, and the same snapshot with one extra member at the top level, inside a coverage row, a finding, a SOUP row, a vulnerability, an anomaly and a change record is refused by the schema each time. | Test |
| FR-167-AC-2 | `evidence_freshness` has exactly the five keys and no other, and its counts sum to the number of criteria in `coverage`. | Test |
| FR-167-AC-3 | A fixture with a `stale`, a `suspect` and a `bound` criterion carries each at its status in `coverage`, and the auditor findings of the first two appear in `open_findings`; a fixture with two `surviving-mutant` findings on one obligation that differ only in `line`, and two that differ only in `summary`, yields them in `path`, `line`, `symbol`, `summary` order whatever order the audit report lists them. | Test |
| FR-167-AC-4 | A fixture with an unresolved-advisory record, a past-end-of-support record and a clean record yields `soup.rows` in that group order with `group` set to `unresolved_advisory`, `past_end_of_support` and `other`, and a scan reported not run, an advisory matching no record and a policy finding matching no record each appear in `soup.not_assessed` with `kind`, `tool` and `detail`, in `kind`, `tool`, `detail` order. A retained scan with an invalid timestamp refuses the snapshot under FR-166-AC-10 and FR-030-AC-29. | Test |
| FR-167-AC-5 | A fixture with an open anomaly carrying a two-row release rationale, a `closed` `wont-fix` one and an `unassessed` one lists all three by id, with `release_rationale` byte-equal to the record's entries, and no snapshot member named `rationaleChange`. | Test |
| FR-167-AC-6 | An empty evidence store yields `coverage` with every criterion `no run evidence` and `open_findings` holding one `undischarged` finding for each criterion, as FR-032-AC-5 reports and FR-115-AC-3 keeps in `evidence_detail`; the other sections are present, never absent. | Test |
| FR-167-AC-7 | A scan of the snapshot's keys and values finds no digest, hash, manifest or score member, and no sentence stating a release accepted, ready or releasable; the stem "accept" appears only inside a finding token of `open_findings` and inside values copied from a record. | Inspection |
| FR-167-AC-9 | A fixture anomaly file with git history that would give `fixingChange` and `rationaleChange` yields an anomaly row with neither member. | Test |
| FR-167-AC-10 | A fixture with a `missing-run` and a `failed-run` `stale-evidence` finding yields `open_findings` items whose `cause` equals each, byte for byte, as FR-115 supplies it. | Test |
| FR-167-AC-11 | An `open_findings` item of any kind other than `stale-evidence` has no `cause` key. | Test |
| FR-167-AC-12 | A `stale-evidence` finding that FR-115 supplies without `cause` yields an `open_findings` item with no `cause` key, not `null`. | Test |
| FR-167-AC-13 | A newly written snapshot has `tool_identity` with exactly `name` and `version`, where the name is `quoin` and version equals the running executable's `--version` output, and has `clean_tree_precondition: "not_verified"`; the closed snapshot reader accepts a full snapshot and refuses a snapshot missing `tool_identity`, a snapshot missing `clean_tree_precondition`, a `scan_unordered` item, `null`, unknown members in `tool_identity`, a different tool name, a blank version, a `soup.not_assessed` item of an unknown kind, or any clean-tree value other than `not_verified`, and accepts a `soup.not_assessed` item of kind `scan_not_run`. | Test |
| FR-167-AC-15 | A fixture whose `control` is assumed by a post-control estimate and has no incoming `verifies` edge yields an `open_findings` item with `kind` `control-without-verification`, `obligation` the control id, `summary` the control id, `path`, `line` and `symbol` `null` and no `cause`, and `quoin risk-summary` over the same fixture carries the same token naming the same control. | Test |
| FR-167-AC-16 | The same fixture with an incoming `verifies` edge added to the control yields no `control-without-verification` item, and a complete chain yields no item of any FR-170 token. | Test |
| FR-167-AC-17 | A control with no incoming `verifies` edge that appears on two rows of `quoin risk-summary` yields exactly one `control-without-verification` item; a fixture with a `benefit-risk-not-accepted` finding naming a residual risk and its covering benefit-risk ids yields one item whose `obligation` is the residual-risk id and whose `summary` lists every name in `names` order. | Test |
| FR-167-AC-18 | A fixture with an `unassessed` anomaly and an `impacting` anomaly with no release rationale yields one `anomaly-unassessed` item and one `anomaly-unjustified-impacting` item, each with the anomaly's id and path; a closed `fixed` anomaly and an `impacting` anomaly with a rationale yield none, as FR-164-AC-5 and FR-164-AC-4 state. | Test |
| FR-167-AC-19 | A fixture holding auditor, risk-completeness and anomaly-gate findings yields `open_findings` in `obligation`, `kind`, `path`, `line`, `symbol`, `cause`, `summary` order whatever order the sources produce them, and no item carries a key outside the closed shape or a severity, score or acceptance word, the stem "accept" occurring only inside a finding token. | Test |
| FR-167-AC-14 | A scan with an invalid retained timestamp refuses the snapshot section under FR-166-AC-11, so the snapshot never contains a `scan_unordered` item. | Test |
| FR-167-AC-20 | A fixture with `open`, `in_remediation`, `remediated`, and `closed` vulnerability records carries only the first two into `vulnerabilities`, copies each listed source field unchanged, orders included records by id, and emits an empty list when neither open status is present. | Test |
| FR-167-AC-21 | A fixture with an included vulnerability record whose status or disposition conflicts with advisory scan, VEX, or anomaly data carries the authored values unchanged; a record with status `remediated` or `closed` remains absent. | Test |
| FR-167-AC-22 | A valid `quoin.release-snapshot/v1` written before the `vulnerabilities` member was added is accepted by the reader and exposes `vulnerabilities` as an empty list; an invalid present value is still refused. | Test |

## Dependencies

- **Upstream**: [FR-115](./FR-115-evidence-backed-test-matrix.md) (including the typed `cause` of FR-032-AC-18..21), [FR-144](./FR-144-soup-report.md),
  [FR-162](./FR-162-anomaly-report.md), [FR-163](./FR-163-anomaly-report-release-rationale.md), [FR-164](./FR-164-anomaly-report-gate.md),
  [FR-169](./FR-169-risk-summary-command.md), [FR-170](./FR-170-risk-summary-row-findings.md),
  [FR-166](./FR-166-release-snapshot-command.md), [FR-168](./FR-168-release-snapshot-change-records.md),
  spec-objects-security [FR-032](ix://agent-ix/spec-objects-security/FR-032) (authored vulnerability lifecycle fields).
- **Downstream**: the dossier documents, which read the snapshot. The closed shape of an `open_findings` item does
  not change and `kind` is an open string for the reader, so the release record of FR-201 renders the added items
  in their source order with no change of its own; the format version stays `quoin.release-snapshot/v1` and no
  reader accepts a second shape. A snapshot written before this change simply lacks the added items; this command never rewrites an existing snapshot (an existing `--out` is refused), so a tagged release's snapshot stays as written and regenerating one is the caller's act.
