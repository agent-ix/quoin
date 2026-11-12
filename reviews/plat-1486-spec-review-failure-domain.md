---
id: SR-20261112024910
title: "Failure-domain review of quoin PR 830 (FR-137..139)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin@3157f4ba; spec/functional/FR-137-detect-and-place-licensed-packs.md, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md"
review_set: subset
---

# Failure-domain review of quoin PR 830

## Summary

Ticket: PLAT-1486. Checked work-tree detection and the write boundary. Covered already: `.git` as
a file (linked worktrees and submodules, FR-137-AC-4), the resolved path (so a symlink is
followed), `IX_HOME` inside a work tree (AC-7), and load failure staying fail-closed (AC-8).
quire-rs `ClauseSet.rights` has no serde default, so AC-8's oracle is true.

## Verdict

**PASS WITH FINDINGS, four medium.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Only `path:` installs are checked. A pack installed from a `git:` or `git-subdir:` source installs silently, and that source is licensed text already in a git history. Refuse a pack from any git source, or state the rule and add an AC | spec/functional/FR-137-detect-and-place-licensed-packs.md:41 |
| FND-002 | medium | The edge cases of the ancestor walk are not decided. A gitignored directory, and an `IX_HOME` under a dotfiles repo at `$HOME`, both have a `.git` ancestor and are refused. A stray `.git` that is not a repository is also refused. A `GIT_DIR`/`GIT_WORK_TREE` work tree with no `.git` entry is not detected. State each outcome explicitly. Add ACs for the ignored-path case and for a symlinked source on each side of the boundary | spec/functional/FR-137-detect-and-place-licensed-packs.md:41, spec/functional/FR-137-detect-and-place-licensed-packs.md:51 |
| FND-003 | medium | FR-137-AC-6 says to delete the licensed clause-set file from the installed directory. The manifest still declares that set, so the engine load fails rather than reporting `pack: false`. `list` behaviour when an installed module fails to load is not specified. Change the AC to edit `text:` (as FR-138-AC-7 does), and state that `list` fails closed with the load error, never `pack: false` | spec/functional/FR-137-detect-and-place-licensed-packs.md:64 |
| FND-004 | medium | The FR-139 title ("using a pack") and Rationale (no "cache, report, log, or evidence file") claim more than the Behavior delivers, which binds only install/list/inspect/remove. Other quoin commands that load an installed pack and write into the repo are not covered. Either retitle to the four commands, or add a bullet that no output any quoin command writes outside `IX_HOME` contains clause text from a restricted set | spec/functional/FR-139-pack-writes-nothing-into-a-repository.md:2, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md:28 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-137, path: spec/functional/FR-137-detect-and-place-licensed-packs.md, role: examined, excerpt: "install path:<dir> SHALL refuse a pack when <dir> or any ancestor of its resolved path contains a .git entry (directory or file)."}
- {id: FR-139, path: spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, role: examined, excerpt: "module install, list, inspect and remove over a pack SHALL create, modify, or delete files only beneath IX_HOME."}
```
