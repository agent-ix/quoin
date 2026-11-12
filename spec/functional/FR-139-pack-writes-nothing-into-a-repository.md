---
id: FR-139
title: "Installing or using a pack writes nothing into any repository"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
---

# FR-139: Installing or using a pack writes nothing into any repository

## Description

Installing, listing, inspecting, or removing a pack SHALL write only under
`IX_HOME` (the registry and the modules directory). Quoin SHALL NOT write any file,
clause text, path, or registry data from a pack into the source tree or into any
other directory outside `IX_HOME`.

## Rationale

The pack source is outside every repository (FR-137), and the installed copy is
under `IX_HOME`. This requirement closes the remaining route: quoin itself copying
pack content into a working repository, such as a cache, report, log, or evidence
file.

## Behavior

- `module install`, `list`, `inspect` and `remove` over a pack SHALL create, modify, or delete files only beneath `IX_HOME`.
- Those commands SHALL leave the pack source directory byte-identical.
- Those commands SHALL NOT write to the current working directory.
- When any of those commands completes, `git status --porcelain` in the source tree, the working directory's repository, and every other work tree in the test sandbox SHALL be empty.
- Diagnostics SHALL name sets as `authority/id@version` and SHALL NOT quote clause text.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-139-AC-1 | In a sandbox with a temp `IX_HOME`, a pack source in a temp directory, and two initialised git repositories (one as the process working directory), running install, list, inspect and remove for the pack leaves `git status --porcelain` empty in both repositories. | Test |
| FR-139-AC-2 | A recursive listing and content hash of the pack source directory is identical before and after install, list, inspect and remove. | Test |
| FR-139-AC-3 | A snapshot of every file in the sandbox outside `IX_HOME` is identical before and after the four commands. | Test |
| FR-139-AC-4 | Searching every file written under the sandbox outside the installed module directory for the fixture's clause-text sentinel finds none, including the registry. | Test |
| FR-139-AC-5 | After `remove`, the pack's directory under `IX_HOME` is gone and the registry has no entry for it. | Test |
| FR-139-AC-6 | A control run that deliberately writes a file into a repository is detected by the same oracle, so the check can fail. | Test |

## Constraints

Fixtures SHALL be original synthetic content with `example.invalid` authorities.

## Dependencies

- **Upstream**: [FR-137](./FR-137-detect-and-place-licensed-packs.md)
