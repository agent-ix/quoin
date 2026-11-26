---
id: FR-166
title: "quoin release snapshot: write the evidence state of a release as one file, running no git"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-167"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-168"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-164"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-170"
    type: "depends_on"
  - target: "ix://agent-ix/engineering-assurance/FR-051"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-049"
    type: "references"
---

# FR-166: quoin release snapshot: write the evidence state of a release as one file, running no git

## Description

Tracked by Linear EA-101. `quoin release snapshot` writes the evidence state of one release as a single file, the
release evidence snapshot. The person who releases commits that file into the release commit, beside the release
record, and signs the release tag; the tag then binds the snapshot through git's own commit hash. Engineering
Assurance owns that placement and its check
(`ix://agent-ix/engineering-assurance/FR-051`); Quoin owns what the file says and how it is written. The split follows
`ix://agent-ix/engineering-assurance/FR-049`: Quoin runs no git, and Engineering Assurance supplies what only git
knows as data.

The command is a view of views. It builds the evidence status of FR-115, the SOUP report of FR-144 and the carried
anomalies of FR-162 from the repository files those requirements read, adds the change-assurance records of
FR-168, and renders them in the format of FR-167. It also states the open findings the other views leave unsaid: the
risk-completeness findings of FR-170, built as `quoin risk-summary` builds them, and the anomaly-gate failures of
FR-164, evaluated over the carried anomalies. A gate failure does not stop the snapshot, because the failure is itself an open finding (FR-167). A risk-summary failure refuses it, because a finding cannot be omitted unannounced. It adds no record type and changes no document, and it decides no
release and no risk: a finding names a gap, never an acceptance.

For the anomalies the command uses only the part of FR-162 that selects the `Anomaly` documents (FR-162 Selection)
and reads their authored values (FR-162 Row, FR-163 As authored). It does not use the part that derives values from
git history, which is the `fixingChange` of FR-136 and the `rationaleChange` of FR-163. The snapshot derives neither.

Caller precondition, which Quoin cannot check: every source is read from the `--repo` working tree, so the tree
must be a clean checkout of the baseline commit that `--baseline-commit` names. The snapshot then states the
evidence state of that baseline (`ix://agent-ix/engineering-assurance/FR-051`).

Out of scope: any git operation, resolving a tag or a commit, deciding whether a release is acceptable, signing,
and any digest, hash, manifest or pin of the snapshot or of what it covers.

## Inputs

- `--repo <dir>`, the repository whose spec, evidence store, SOUP records and anomalies are read.
- `--baseline-commit <hash>`, the full commit hash of the baseline, as Engineering Assurance supplies it. Quoin
  states it in the snapshot and does not resolve it.
- `--history <file>`, the release-history file of `ix://agent-ix/engineering-assurance/FR-052` (FR-168).
- `--binding <clause-binding-v1 report>` and `--as-of <instant>`, as `quoin soup report` takes them (FR-144).
- `--records <dir>`, optional, the `soup_record` directory of FR-144 (default `spec/soup/`).
- `--support <file>`, optional, the support-data file of FR-143. When absent, no support data is applied, as for
  `quoin soup report` without it.
- `--module DIR`, optional and repeatable, the safety module roots `quoin risk-summary` takes (FR-169); with none, the
  installed modules.
- `--out <path>`, the file to write.

## Outputs

- One file at `--out`, in the format of FR-167, and one line on standard output naming the path.
- A refusal with exit status 2 and no file, carrying one of the codes under Behavior.

## Behavior

- When a user runs `quoin release snapshot`, the command SHALL write exactly one file, at `--out`.
- The command SHALL write the file only after every section of FR-167 is built.
- The command SHALL build the evidence status through the code path `quoin matrix` uses (FR-115).
- The command SHALL build the SOUP rows through the code path `quoin soup report` uses (FR-144).
- The command SHALL select and read the carried anomalies through the selection and record-reading code of `quoin anomaly report` (FR-162).
- The command SHALL build the risk-completeness findings through the code path `quoin risk-summary` uses (FR-169, FR-170), over `--repo` and the `--module` roots.
- The command SHALL evaluate the anomaly-gate modes of FR-164 over the carried anomalies through the gate code of `quoin anomaly report`, with both modes.
- If a gate failure exists, then the command SHALL still write the snapshot.
- If `quoin risk-summary` would exit 2 over `--repo` and the `--module` roots, then the command SHALL refuse with `release_snapshot_input_invalid` naming the failing root (`--repo` or the `--module DIR`) and the record or module reason.
- If a markdown file below `--repo` cannot be read, or its frontmatter block is not a YAML mapping (invalid YAML, a list or a scalar), then the command SHALL refuse with `release_snapshot_input_unreadable` naming `--repo` and the file. The reader cannot tell a risk record from another file before reading it, so every such file refuses, as an unreadable scan file does.
- The command SHALL NOT refuse a markdown file whose frontmatter is a YAML mapping, or is empty, but names no `type` or `id` (an ADR page, or a `SKILL.md` holding only `name` and `description`).
- The command SHALL NOT report a markdown file with a YAML mapping as frontmatter, or an empty one, and no `type` or `id` as unreadable.
- The command SHALL NOT derive `fixingChange` or `rationaleChange`.
- The command SHALL NOT call the code of `quoin anomaly report` that reads git history.
- The command SHALL NOT run git.
- The command SHALL NOT call a command that runs git.
- The command SHALL NOT read the git object store, the index or any ref.
- The command SHALL NOT read the system clock.
- The command SHALL take every instant in the snapshot from `--as-of`.
- If `--out` already exists, then the command SHALL refuse with `release_snapshot_exists` and leave it unchanged.
- If `--baseline-commit` is not a full lowercase hexadecimal commit hash of 40 or 64 digits, then the command SHALL
  refuse with `release_snapshot_baseline_commit_invalid`.
- If an input named under Inputs is missing or unreadable, then the command SHALL refuse with
  `release_snapshot_input_unreadable` naming the input.
- If a retained scan file in the `--repo` evidence store cannot be read, then the command SHALL refuse with
  `release_snapshot_input_unreadable` naming `--repo` and the file, because an older scan of the same tool must not stand in for it.
- If a retained scan in the `--repo` evidence store has a timestamp that is not an instant, then the command SHALL
  refuse with `release_snapshot_input_invalid` naming `--repo`.
- If an input named under Inputs other than `--history` fails its own format, then the command SHALL refuse with
  `release_snapshot_input_invalid` naming the input.
- If `--history` fails its format, then the command SHALL refuse with `release_snapshot_history_invalid`, as
  [FR-168](./FR-168-release-snapshot-change-records.md) states, and never with `release_snapshot_input_invalid`.
- If an `Anomaly` document in scope has a mapping error and so no record (FR-136 `anomaly.no-record`), then the
  command SHALL refuse with `release_snapshot_anomaly_unreadable` naming the path and the mapping error's code.
  The snapshot has no `unreadable` member, so a carried anomaly never disappears from it unannounced.
- If the command refuses, then it SHALL write no file.
- The command SHALL write nothing other than `--out`.
- The command SHALL NOT sign, tag, commit or stage the file.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-166-CON-1 | The command SHALL contain no code that starts a git process or links a git library. | Architecture | Inspection |
| FR-166-CON-2 | The command SHALL contain no hashing code of its own for the snapshot. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-166-AC-1 | Run over a fixture repository with evidence, SOUP records, anomalies and a history file whose `baseline_commit` equals `--baseline-commit`, the command writes one file that parses as the FR-167 format, and no other file in the repository changes or appears. | Test |
| FR-166-AC-2 | Run in a directory that is not a git repository, with `git` absent from `PATH`, the command writes the same bytes as it does in a git repository whose anomaly files have commit history, and the snapshot contains no `fixingChange` or `rationaleChange` member. | Test |
| FR-166-AC-3 | A `--baseline-commit` that is a branch name, a short hash or an upper-case hash is refused with `release_snapshot_baseline_commit_invalid` and no file exists. | Test |
| FR-166-AC-4 | An existing `--out` is refused with `release_snapshot_exists` and its bytes are unchanged. | Test |
| FR-166-AC-5 | A missing `--history` is refused with `release_snapshot_input_unreadable` naming it; a malformed `--binding` is refused with `release_snapshot_input_invalid` naming it; each with exit status 2 and no file. | Test |
| FR-166-AC-6 | For the same repository, `--records`, `--support`, `--binding` and `--as-of`, every member that FR-167 names for a coverage row, a SOUP row and an anomaly equals the same-named value in the JSON output of `quoin matrix`, `quoin soup report` and `quoin anomaly report` (compared member by member, with the report's camelCase keys read as the snapshot's snake_case keys), and the snapshot has no member those reports do not define, except two that FR-167 derives: `group` of a SOUP row, which is FR-144's ordering group of the row, and `unassessed` of an anomaly, which is true exactly when the anomaly's `safety_impact` is `unassessed` (FR-162's row flag). Both are compared against those definitions instead. | Test |
| FR-166-AC-9 | An `Anomaly` document with a mapping error yields `release_snapshot_anomaly_unreadable` naming its path and the mapping code, exit status 2 and no file, while a fixture without one writes the snapshot; a malformed `--history` yields `release_snapshot_history_invalid`, not `release_snapshot_input_invalid`. | Test |
| FR-166-AC-12 | Over a fixture repository holding a control that a post-control estimate assumes and that has no incoming `verifies` edge, a situation with no harm and an `unassessed` anomaly, the snapshot's `open_findings` hold items for `control-without-verification`, `situation-without-harm` and `anomaly-unassessed`, and the same repository gives the first two as tokens of `quoin risk-summary` and the third as a failure of `quoin anomaly report --fail-on unassessed`; the command exits zero and writes the file. | Test |
| FR-166-AC-13 | A fixture that `quoin risk-summary` refuses with exit 2 (a `residual_risk` with an unknown decision word) yields `release_snapshot_input_invalid` naming `--repo` and the record, exit status 2 and no file; a `--module DIR` that does not load yields `release_snapshot_input_invalid` naming that `--module DIR`; a repository with no risk record yields a snapshot with no risk-completeness item. | Test |
| FR-166-AC-14 | Each of a risk record file, a non-risk markdown file that is not valid UTF-8, and a markdown file whose frontmatter block is not a YAML mapping, placed below `--repo`, yields `release_snapshot_input_unreadable` naming `--repo` and the file, exit status 2 and no file; a markdown file with a YAML mapping but no `type` and no `id` (a `SKILL.md` with only `name` and `description`), and one with an empty `---` block, are read and the snapshot is written; a file with `type: vulnerability` but no `id` yields `release_snapshot_input_invalid` naming `--repo` and the file, exit status 2 and no file. | Test |
| FR-166-AC-7 | Static inspection finds no git process, git library or hashing code in the command (CON-1, CON-2). | Inspection |
| FR-166-AC-8 | A `soup_record` with `end_of_support` `unknown` and a `--support` file stating a past date places the record in the `past_end_of_support` group, and the same run without `--support` places it in `other`; a `--records` directory other than the default is read. | Test |
| FR-166-AC-10 | A retained scan file in the evidence store that cannot be decoded yields `release_snapshot_input_unreadable` naming `--repo` and the file, exit status 2 and no file, although an older readable scan of the same tool exists. | Test |
| FR-166-AC-11 | A retained scan whose timestamp is not an instant makes the SOUP section refuse with the input `--repo` and the cause `scan_timestamp_invalid`, which the command reports as `release_snapshot_input_invalid` naming `--repo`; a scan that was not run is stated, not refused. | Test |

## Dependencies

- **Upstream**: [FR-167](./FR-167-release-snapshot-format.md), [FR-168](./FR-168-release-snapshot-change-records.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md), [FR-144](./FR-144-soup-report.md),
  [FR-162](./FR-162-anomaly-report.md), [FR-164](./FR-164-anomaly-report-gate.md),
  [FR-169](./FR-169-risk-summary-command.md), [FR-170](./FR-170-risk-summary-row-findings.md); `ix://agent-ix/engineering-assurance/FR-051` for placement in the release
  commit.
- **Downstream**: [NFR-036](../non-functional/NFR-036-release-snapshot-is-deterministic-and-never-decides.md); the
  dossier documents, which read the file.
