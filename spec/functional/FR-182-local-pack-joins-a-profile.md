---
id: FR-182
title: "A locally installed licensed pack joins a profile from local configuration only"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-180"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-181"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
---

# FR-182: A locally installed licensed pack joins a profile from local configuration only

## Description

A pack is defined by [FR-137](./FR-137-detect-and-place-licensed-packs.md): an installed module carrying a restricted
clause set, decided by the engine alone and kept outside any git work tree. A pack is never named in a published profile.
It joins a profile in one way: its clause-set key is listed in `add` of the user config root's `source-profile.yaml`
([FR-004](./FR-004-resolve-config-root.md)), which belongs to the person and is not part of any repository. The project
config root's file lives at `<cwd>/.ix` inside the repository, so it can be committed; a key declared by a pack
therefore does not belong in it.

The user file carries `add` only, with no `profile` or `context`. It applies to a run only when the project's
`source-profile.yaml` selects a profile ([FR-180](./FR-180-source-profile-selection.md)); with no project selection
file, no profile is selected and the user file is not read.

Evaluation of an added set is the same as any other ([FR-181](./FR-181-source-profile-evaluation.md)). On a machine
without the pack the entry is `unavailable`, so the same repository evaluates cleanly elsewhere.

## Behavior

- Where a profile is selected, Quoin SHALL read `add` from the user config root's `source-profile.yaml` as well as the project config root's, appending user entries after project entries and removing exact-key duplicates.
- When the user file carries `profile` or `context`, Quoin SHALL refuse it.
- When a project-layer `add` key is declared by an installed module that FR-137 identifies as a pack, Quoin SHALL refuse the run, naming the key and the layer.
- Quoin SHALL NOT write a clause-set key into any file under a module root or any `profiles/` directory.
- Quoin SHALL mark in the report each set with its layer (`profile`, `project` or `user`).
- Quoin SHALL NOT copy clause or standard text into the report, which carries keys, clause ids, outcomes and reasons only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-182-AC-1 | A key in the user file's `add` is evaluated after the profile's sets and after project `add` entries, and its entry has layer `user`. | Test |
| FR-182-AC-2 | The same repository on a machine without that module reports the key `unavailable` and exits 0. | Test |
| FR-182-AC-3 | A user file with `profile` or `context` is refused. | Test |
| FR-182-AC-4 | After a run adding a user-layer key, no file under any module root or `profiles/` directory has changed. | Test |
| FR-182-AC-5 | The report holds no clause `text` or `title` field; it contains clause ids only. | Test |
| FR-182-AC-6 | A key present in both layers appears once, with layer `project`, unless that key is declared by a pack (AC-7). | Test |
| FR-182-AC-7 | A project-layer `add` key declared by an installed pack exits non-zero naming the key and the `project` layer; the same key in the user file is accepted. | Test |
| FR-182-AC-8 | With a user file and no project selection file, no profile is selected, no set is evaluated, and the user file is not read. | Test |

## Dependencies

- **Upstream**: [FR-004](./FR-004-resolve-config-root.md), [FR-137](./FR-137-detect-and-place-licensed-packs.md), [FR-180](./FR-180-source-profile-selection.md), [FR-181](./FR-181-source-profile-evaluation.md)
