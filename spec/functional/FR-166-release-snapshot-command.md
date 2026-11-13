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
FR-168, and renders them in the format of FR-167. It adds no record type and changes no document.

Out of scope: any git operation, resolving a tag or a commit, deciding whether a release is acceptable, signing,
and any digest, hash, manifest or pin of the snapshot or of what it covers.

## Inputs

- `--repo <dir>`, the repository whose spec, evidence store, SOUP records and anomalies are read.
- `--baseline-commit <hash>`, the full commit hash of the baseline, as Engineering Assurance supplies it. Quoin
  states it in the snapshot and does not resolve it.
- `--history <file>`, the release-history file of `ix://agent-ix/engineering-assurance/FR-052` (FR-168).
- `--binding <clause-binding-v1 report>` and `--as-of <instant>`, as `quoin soup report` takes them (FR-144).
- `--out <path>`, the file to write.

## Outputs

- One file at `--out`, in the format of FR-167, and one line on standard output naming the path.
- A refusal with exit status 2 and no file for an input the command cannot read.

## Behavior

- When a user runs `quoin release snapshot`, the command SHALL write exactly one file, at `--out`.
- The command SHALL write the file only after every section of FR-167 is built.
- The command SHALL build the evidence status through the code path `quoin matrix` uses (FR-115).
- The command SHALL build the SOUP rows through the code path `quoin soup report` uses (FR-144).
- The command SHALL build the carried anomalies through the code path `quoin anomaly report` uses (FR-162).
- The command SHALL NOT run git.
- The command SHALL NOT call a command that runs git.
- The command SHALL NOT read the git object store, the index or any ref.
- The command SHALL read the clock for no value; the only instant in the snapshot is `--as-of`.
- If `--out` already exists, then the command SHALL refuse with `release_snapshot_exists` and leave it unchanged.
- If `--baseline-commit` is not a full lowercase hexadecimal commit hash of 40 or 64 digits, then the command SHALL
  refuse with `release_snapshot_baseline_commit_invalid`.
- If an input named under Inputs is missing, unreadable or fails its own format, then the command SHALL refuse
  naming the input and write no file.
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
| FR-166-AC-1 | Run over a fixture repository with evidence, SOUP records, anomalies and a history file, the command writes one file that parses as the FR-167 format, and no other file in the repository changes or appears. | Test |
| FR-166-AC-2 | Run in a directory that is not a git repository, with `git` absent from `PATH`, the command writes the same bytes as it does in a git repository. | Test |
| FR-166-AC-3 | A `--baseline-commit` that is a branch name, a short hash or an upper-case hash is refused with `release_snapshot_baseline_commit_invalid` and no file exists. | Test |
| FR-166-AC-4 | An existing `--out` is refused with `release_snapshot_exists` and its bytes are unchanged. | Test |
| FR-166-AC-5 | A missing or malformed `--history`, `--binding` or repository input is refused naming the input, with exit status 2 and no file. | Test |
| FR-166-AC-6 | The evidence status, SOUP rows and anomaly rows in the snapshot equal those that `quoin matrix`, `quoin soup report` and `quoin anomaly report` render for the same inputs and `--as-of`. | Test |
| FR-166-AC-7 | Static inspection finds no git process, git library or hashing code in the command (CON-1, CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-167](./FR-167-release-snapshot-format.md), [FR-168](./FR-168-release-snapshot-change-records.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md), [FR-144](./FR-144-soup-report.md),
  [FR-162](./FR-162-anomaly-report.md); `ix://agent-ix/engineering-assurance/FR-051` for placement in the release
  commit.
- **Downstream**: [NFR-036](../non-functional/NFR-036-release-snapshot-is-deterministic-and-never-decides.md); the
  dossier documents, which read the file.
