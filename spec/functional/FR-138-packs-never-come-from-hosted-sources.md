---
id: FR-138
title: "Packs never come from hosted sources"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-083"
    type: "references"
  - target: "ix://agent-ix/quire-cli/FR-029"
    type: "references"
---

# FR-138: Packs never come from hosted sources

## Description

Quoin SHALL refuse to install a pack from any source other than `path:<dir>`, and
`quoin module ensure-defaults` SHALL refuse a default entry that is a pack. Pack
status is the one FR-137 derives.

## Rationale

A pack that arrives from a github, git or git-subdir source originates in
a hosted repository, where the licensed text is already published. Quoin has no
module-publish command, so there is nothing for quoin to gate on the way out; the
publish gate for modules is `quire publish-check` (quire-cli FR-029), which this
requirement does not duplicate. What quoin controls is what it ingests: a pack only
from a local directory (FR-137), and never in the default module set that quoin
distributes.

## Behavior

- `quoin module install <SOURCE>` SHALL refuse a pack when the source is a github, git or git-subdir source.
- A refusal SHALL name the module, the source kind and the restricted sets as `authority/id@version`.
- A refusal SHALL exit non-zero.
- A refusal SHALL leave the registry and the modules directory unchanged.
- A refusal SHALL NOT contain clause text.
- A module with no restricted clause set SHALL install from those sources as before.
- `quoin module ensure-defaults` SHALL refuse a default entry whose module is a pack, naming it.
- A refused `ensure-defaults` SHALL leave the registry unchanged for that entry.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-138-AC-1 | For each of a git, git-subdir and github source resolving to a synthetic pack, install exits non-zero naming the module, source kind and `authority/id@version`, and the registry and modules directory are byte-identical before and after. | Test |
| FR-138-AC-2 | The same sources resolving to a non-pack module install. | Test |
| FR-138-AC-3 | A default-module entry resolving to a synthetic pack makes `ensure-defaults` exit non-zero naming that module, and the registry holds no entry for it. | Test |
| FR-138-AC-4 | No refusal on stdout or stderr contains the fixture's clause-text sentinel. | Test |
| FR-138-AC-5 | The same pack installed from `path:<dir>` outside any work tree still installs (FR-137-AC-1), so the refusal depends on the source kind and not on the pack alone. | Test |

## Constraints

Fixtures SHALL be original synthetic content with `example.invalid` authorities.

## Dependencies

- **Upstream**: [FR-137](./FR-137-detect-and-place-licensed-packs.md); the publish gate is quire-cli FR-029 and quire-rs FR-083.
