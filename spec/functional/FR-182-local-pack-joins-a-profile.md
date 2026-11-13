---
id: FR-182
title: "A locally installed licensed pack joins a profile from local configuration only"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-180"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-181"
    type: "depends_on"
---

# FR-182: A locally installed licensed pack joins a profile from local configuration only

## Description

A pack whose standard text is licensed is installed as a module on one machine and is never named in a published
profile. It joins a profile in one way: its clause-set key is listed in `add` of a `source-profile.yaml`
([FR-180](./FR-180-source-profile-selection.md)) in a config root that is not published. Two such files are read.
The project config root's file is the repository's; the user config root's file ([FR-004](./FR-004-resolve-config-root.md))
belongs to the person and is the place for a licensed pack on a public repository. Both may carry `add`; the user file
carries `add` only, with no `profile` or `context`.

Evaluation of an added set is the same as any other ([FR-181](./FR-181-source-profile-evaluation.md)). On a machine
without the pack the entry is `unavailable`, so the same repository evaluates cleanly elsewhere.

## Behavior

- Quoin SHALL read `add` from the user config root's `source-profile.yaml` as well as the project config root's, appending user entries after project entries and removing exact-key duplicates.
- Quoin SHALL refuse a user `source-profile.yaml` that carries `profile` or `context`.
- Quoin SHALL NOT write a clause-set key into any file under a module root or any `profiles/` directory.
- Quoin SHALL mark in the report each set that came from `add`, with the layer (`project` or `user`).
- Quoin SHALL NOT copy clause or standard text into the report, which carries keys, clause ids, outcomes and reasons only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-182-AC-1 | A key in the user file's `add` is evaluated after the profile's sets and after project `add` entries, and its entry is marked `user`. | Test |
| FR-182-AC-2 | The same repository on a machine without that module reports the key `unavailable` and exits 0. | Test |
| FR-182-AC-3 | A user file with `profile` or `context` is refused. | Test |
| FR-182-AC-4 | After a run adding a user-layer key, no file under any module root or `profiles/` directory has changed. | Test |
| FR-182-AC-5 | The report holds no clause `text` or `title` field; it contains clause ids only. | Test |
| FR-182-AC-6 | A key present in both layers appears once, marked `project`. | Test |

## Dependencies

- **Upstream**: [FR-004](./FR-004-resolve-config-root.md), [FR-180](./FR-180-source-profile-selection.md), [FR-181](./FR-181-source-profile-evaluation.md)
