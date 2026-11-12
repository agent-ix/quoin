---
id: FR-137
title: "Detect a licensed pack at install and require it to live outside any git work tree"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-019"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-083"
    type: "references"
---

# FR-137: Detect a licensed pack at install and require it to live outside any git work tree

## Description

A **pack** is an installed module that carries at least one restricted clause set,
as quire-rs FR-083 defines restricted. Quoin SHALL derive pack status by calling
the engine's `check_publishable` over the loaded module; a non-empty result makes
the module a pack. `quoin module install path:<dir>` SHALL refuse a pack whose
source directory lies inside a git work tree, and `quoin module list` SHALL mark
each pack.

## Rationale

Pack status is derived, not declared. The clause sets are the only licence signal
(FR-083), so a registry flag or a directory convention would be a second copy of
that signal that can disagree with it: a module could be flagged a non-pack while
carrying licensed text, which is the exact failure the guard exists to prevent.
Deriving it from the loaded clause sets adds no user-declared field, leaves the
registry record unchanged, and cannot drift. The cost is one engine call per
install and per list, which is local and bounded by the module size. Quoin holds
no rights policy of its own (FR-138-AC-6).

A pack must live outside every repository because a git work tree is the most
likely route by which licensed text reaches a public history: once committed it is
not removable by deleting it at HEAD.

## Behavior

- Quoin SHALL decide whether an installing module is a pack by the engine's `check_publishable` result alone, and SHALL declare no pack field, flag, or directory convention.
- `quoin module install path:<dir>` SHALL refuse a pack when `<dir>` or any ancestor of its resolved path contains a `.git` entry (directory or file).
- A refusal SHALL name the source directory, the enclosing work-tree root, and the restricted sets as `authority/id@version`.
- A refusal SHALL state that a pack is installed from a directory outside any git work tree.
- A refused install SHALL exit non-zero.
- A refused install SHALL leave the registry and the modules directory unchanged.
- A refusal SHALL NOT contain clause text.
- A pack whose source lies outside every git work tree SHALL install normally.
- A module with no restricted clause set SHALL install from inside a git work tree as before.
- `quoin module list` SHALL include `"pack": true` on each entry whose installed module is a pack, and `"pack": false` on every other entry, computed from the installed module on each call.
- A pack's registry record SHALL carry no field beyond those of any other installed module.
- Quoin SHALL refuse a pack install whose target modules directory resolves inside a git work tree, with the same error shape.
- If the source's clause sets fail to load, then install SHALL fail with the load error.
- If the source's clause sets fail to load, then install SHALL NOT treat the module as a non-pack.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-137-AC-1 | A synthetic `example.invalid` module with one `text: licensed` clause set, installed from a temp directory outside any work tree into a temp `IX_HOME`, installs, and `quoin module list` reports `"pack": true` for it. | Test |
| FR-137-AC-2 | The same module placed under a directory containing `.git` is refused by `quoin module install path:<dir>` with a non-zero exit; the error names the source directory, the work-tree root and `authority/id@version`, and the registry file and modules directory are byte-identical before and after. | Test |
| FR-137-AC-3 | A module whose sets are all `text: none`, `original` or `explicitly_cleared` installs from inside a git work tree, and `list` reports `"pack": false` for it. | Test |
| FR-137-AC-4 | With `.git` as a file (a linked worktree) rather than a directory above the source, the pack is still refused. | Test |
| FR-137-AC-5 | The refusal output contains none of the fixture's clause-text sentinel. | Test |
| FR-137-AC-6 | After a pack install, the registry entry's serialized keys equal those of a non-pack install, and deleting a pack's licensed clause set from the installed directory changes `list` to `"pack": false` on the next call. | Test |
| FR-137-AC-7 | A pack install with `IX_HOME` set inside a git work tree is refused with the work-tree root named and nothing written. | Test |
| FR-137-AC-8 | A module whose `clause_sets` entry has no `rights` object fails install with the load error naming the file, and nothing is installed. | Test |
| FR-137-AC-9 | A static check over the install and list source finds no token `licensed` or `text` rights comparison; the verdict comes only from the `check_publishable` call. | Inspection |

## Constraints

Fixtures SHALL be original synthetic content with `example.invalid` authorities. No
real publication name, identifier or text appears in the repository.

## Dependencies

- **Upstream**: [FR-019](./FR-019-manage-plugin-registry.md), quire-rs FR-083 (`check_publishable`)
- **Downstream**: [FR-138](./FR-138-never-publish-an-installed-pack.md), [FR-139](./FR-139-pack-writes-nothing-into-a-repository.md)
