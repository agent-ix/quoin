---
id: FR-168
title: "Release snapshot change records: the change-assurance records since the previous release, taken as data"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-166"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-167"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-052"
    type: "depends_on"
---

# FR-168: Release snapshot change records: the change-assurance records since the previous release, taken as data

## Description

The version history of a release is the set of change-assurance records
([FR-063](./FR-063-change-assurance-record-integrity.md)) merged between the previous release and this one. Only git can say
which records those are, so Engineering Assurance computes the list and writes it as the release-history file
(`ix://agent-ix/engineering-assurance/FR-052`). `quoin release snapshot` ([FR-166](./FR-166-release-snapshot-command.md))
takes that file as data and copies its entries into the snapshot's `change_records` and `previous_release`
([FR-167](./FR-167-release-snapshot-format.md)). Quoin does not decide which records belong, verify a commit, or
read a repository's history.

Quoin copies no schema of the history file. It reads the members named under Inputs and refuses a file that lacks
one.

## Inputs

- The release-history file, protocol `engineering-assurance.release-history/v1`, read for these members only:
  `baseline_commit` (a string), `previous_release` (`{ tag, commit }` or `null`) and `records`, a list of `{ record_id, revision, path, commit }`.

## Outputs

- `previous_release` and `change_records` in the snapshot, with `change_records` ordered by `record_id` bytewise, then by
  `revision` numerically ascending, then by `commit` bytewise.

## Behavior

- The command SHALL copy each `records` entry's `record_id`, `revision`, `path` and `commit` into `change_records` as given.
- The command SHALL copy `previous_release` as given.
- The command SHALL write exactly the supplied entries, ordered as Outputs states.
- The command SHALL NOT read a record file to confirm an entry.
- The command SHALL NOT fetch a commit.
- If the history file's `baseline_commit` differs from `--baseline-commit` by any byte, then the command SHALL refuse with
  `release_snapshot_history_baseline_mismatch` naming both values.
- The command SHALL compare the two values as strings and resolve neither.
- If the history file's protocol is not `engineering-assurance.release-history/v1`, then the command SHALL refuse with
  `release_snapshot_history_invalid` naming the protocol.
- If the history file lacks `baseline_commit`, `previous_release` or `records`, or an entry lacks one of its four members, then the
  command SHALL refuse with `release_snapshot_history_invalid` naming the member.
- If two entries carry the same `record_id` and `revision`, then the command SHALL refuse with
  `release_snapshot_history_invalid` naming both.
- When `previous_release` is `null`, the command SHALL write `previous_release` as `null` and state every entry of
  `records` as supplied.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-168-AC-1 | A history file with three records out of order yields `change_records` ordered by `record_id`, `revision` then `commit`, with all four members of each entry equal to the file's. | Test |
| FR-168-AC-2 | A history file with `previous_release` `null` yields `previous_release` `null` and the supplied entries; one with a tag and commit yields both unchanged. | Test |
| FR-168-AC-3 | A file of another protocol, a file without `records`, an entry without `commit`, and two entries with one id and revision are each refused with `release_snapshot_history_invalid` naming the protocol, member or pair, and no snapshot file exists. | Test |
| FR-168-AC-4 | The command run with the record files of the history absent from the working tree and with `git` absent from `PATH` yields the same `change_records`. | Test |
| FR-168-AC-5 | An empty `records` list yields `change_records` `[]` and a snapshot, not a refusal. | Test |
| FR-168-AC-6 | A history file whose `baseline_commit` differs from `--baseline-commit` is refused with `release_snapshot_history_baseline_mismatch` naming both values, with no snapshot file, in a directory with `git` absent from `PATH`. | Test |

## Dependencies

- **Upstream**: [FR-166](./FR-166-release-snapshot-command.md), [FR-167](./FR-167-release-snapshot-format.md),
  [FR-063](./FR-063-change-assurance-record-integrity.md); `ix://agent-ix/engineering-assurance/FR-052` for the history file.
- **Downstream**: the dossier documents.
