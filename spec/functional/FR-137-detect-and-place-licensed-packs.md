---
id: FR-137
title: "Detect a licensed pack at install and require it to live outside any git work tree"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-019"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-083"
    type: "references"
---

# FR-137: Detect a licensed pack at install and require it to live outside any git work tree

## Description

A **pack** is an installed module that carries at least one restricted clause set,
as quire-rs FR-083 defines restricted. Quoin SHALL derive pack status from the
engine's `check_publishable` over the loaded module; a non-empty result makes the
module a pack. `quoin module install path:<dir>` SHALL refuse a pack whose source
directory lies inside a git work tree, and `quoin module list` SHALL mark each
pack.

## Rationale

Pack status is derived, not declared. The clause sets are the only licence signal
(FR-083), so a registry flag or directory convention would be a second copy that can
disagree with it: a module could be flagged a non-pack while carrying licensed
text, the failure this guard exists to prevent. Deriving it from the loaded clause
sets adds no user-declared field, leaves the registry record unchanged, and cannot
drift. The cost is one engine call per install and per list.

A pack must live outside every repository because a git work tree is the most
likely route by which licensed text reaches a public history; once committed it is
not removable by deleting it at HEAD.

## Engine route

`quoin-quire` is the only crate that touches the rights decision. It SHALL gain one
function that loads a module and returns quire-rs `check_publishable` for it, and
SHALL list a `publish_check` capability token, with its compile-time witness, the
way `engine.rs` lists `clause_sets`. `quoin-modules` SHALL call that function
through `quoin-core`; pack derivation is hosted in `quoin-quire`. Quoin restates no
rights rule.

## Behavior

- `quoin-quire` SHALL expose one function returning the restricted clause sets of a module directory, computed by `check_publishable` alone.
- `quoin-quire` SHALL list the `publish_check` capability token with a witness that fails to compile when `check_publishable` is absent.
- Quoin SHALL decide pack status by that function's result alone.
- Quoin SHALL declare no pack field, flag or directory convention.
- `quoin module install path:<dir>` SHALL resolve `<dir>` to its canonical path, following symlinks, before judging it.
- `quoin module install path:<dir>` SHALL refuse a pack when the canonical path or any ancestor contains a `.git` entry, directory or file.
- The `.git` walk SHALL ignore gitignore state.
- The `.git` walk SHALL ignore the `GIT_DIR` and `GIT_WORK_TREE` environment variables.
- The `.git` walk SHALL count a stray `.git` entry.
- A refusal SHALL name the source directory, the enclosing work-tree root and the restricted sets as `authority/id@version`.
- A refusal SHALL state that a pack is installed from a directory outside any git work tree.
- A refusal SHALL exit non-zero.
- A refusal SHALL NOT contain clause text.
- A refused install SHALL leave the registry and the modules directory unchanged.
- A pack whose canonical source lies outside every git work tree SHALL install normally.
- A module with no restricted clause set SHALL install from inside a git work tree as before.
- Quoin SHALL refuse a pack install whose `IX_HOME` modules directory resolves inside a git work tree, with the same error shape.
- `quoin module list` SHALL give each entry `"pack": true`, `"pack": false`, or `"pack": null`, computed from the installed module on each call.
- If an installed module fails to load, then its `list` row SHALL show `"pack": null` and the load error.
- If an installed module fails to load, then `list` SHALL still list every other module and exit zero.
- A pack's registry record SHALL carry no field beyond those of any other installed module.
- If the source's clause sets fail to load, then install SHALL fail with the load error.
- If the source's clause sets fail to load, then install SHALL NOT treat the module as a non-pack.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-137-AC-1 | A synthetic `example.invalid` module with one `text: licensed` clause set, installed from a temp directory outside any work tree into a temp `IX_HOME`, installs, and `list` reports `"pack": true`. | Test |
| FR-137-AC-2 | The same module under a directory containing `.git` is refused by `quoin module install path:<dir>` with non-zero exit; the error names the source, the work-tree root and `authority/id@version`; the registry and modules directory are byte-identical before and after. | Test |
| FR-137-AC-3 | A module whose sets are all `text: none`, `original` or `explicitly_cleared` installs from inside a git work tree, and `list` reports `"pack": false`. | Test |
| FR-137-AC-4 | With `.git` as a file (linked worktree or submodule) above the source, the pack is refused. A stray empty `.git` directory above it also refuses. | Test |
| FR-137-AC-5 | The refusal output contains none of the fixture's clause-text sentinel. | Test |
| FR-137-AC-6 | After a pack install, the registry entry's serialized keys equal those of a non-pack install. Replacing the installed pack's `licensed` set with `text: original` changes `list` to `"pack": false` on the next call. | Test |
| FR-137-AC-7 | A pack install with `IX_HOME` inside a git work tree is refused with the work-tree root named and nothing written. | Test |
| FR-137-AC-8 | A module whose `clause_sets` entry has no `rights` object fails install with the load error naming the file, and nothing is installed. | Test |
| FR-137-AC-9 | A symlink outside any work tree pointing at a pack inside a work tree is refused; a symlink inside a work tree pointing at a pack outside every work tree installs. | Test |
| FR-137-AC-10 | A pack under a gitignored path of its enclosing work tree is refused. With `GIT_DIR` and `GIT_WORK_TREE` set to a different location, the verdict is unchanged. | Test |
| FR-137-AC-11 | With one installed module's clause-set file made unreadable, `list` exits zero, shows `"pack": null` and the load error on that row, and lists every other module with its verdict. | Test |
| FR-137-AC-12 | The `quoin-quire` function returns exactly what `check_publishable` returns on the same loaded fixture, and removing the `publish_check` witness's target fails compilation. A scan of the new non-test quoin source finds none of `TextRights`, `Licensed`, `clearance_ref`; fixtures and tests are out of scope. | Test, Inspection |

## Constraints

Fixtures SHALL be original synthetic content with `example.invalid` authorities. No
real publication name, identifier or text appears in the repository.

## Dependencies

- **Upstream**: [FR-019](./FR-019-manage-plugin-registry.md); quire-rs FR-083 (`check_publishable`) — `quoin-quire`'s quire-rs dependency moves to a version that contains `check_publishable`.
- **Downstream**: [FR-138](./FR-138-packs-never-come-from-hosted-sources.md), [FR-139](./FR-139-pack-writes-nothing-into-a-repository.md)
