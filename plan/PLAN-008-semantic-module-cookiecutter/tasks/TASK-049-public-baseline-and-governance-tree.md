---
id: TASK-049
title: "Rendered public baseline and governance tree"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-045"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-081"
    type: references
  - target: "ix://agent-ix/quoin/FR-082"
    type: references
  - target: "ix://agent-ix/quoin/TC-1432"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1433"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1434"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1435"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1436"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1437"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1438"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1439"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1440"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1441"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1442"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1443"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1461"
    type: verifies
---

# TASK-049: Rendered public baseline and governance tree

## Scope

Render the licence, ownership, documentation and packaging surfaces a public
AGPL-3.0-or-later module needs, and the `spec/` tree that validates as rendered.

## Subtasks

- [x] Full AGPL-3.0-or-later text, and one SPDX identifier across `pyproject.toml`, `package.json`, the README and the package metadata (TC-1434).
- [x] `.github/CODEOWNERS`, `AGENTS.md`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `.gitignore`, `.gitattributes`, `Makefile` (TC-1435).
- [x] Matching payloads: the Python `include` and the npm `files` both carry the manifest, schemas and skeletons, and the schema toolchain is excluded from both (TC-1433).
- [x] `scripts/stage-npm.mjs` staging the payload so the tarball root is the module root, and removing the staged copies after packing.
- [x] Public npm publication target with public access; no credential, no token, no private publication default (TC-1436).
- [x] Manually triggered release workflows delegating to the shared reusable workflows, carrying no publish step of their own (TC-1437).
- [x] `docs/catalog-entry.md` naming the Quoin catalog file and the tracking project (TC-1438).
- [x] No local-path dependency reference and no version upper bound anywhere (TC-1432).
- [x] A `spec/` tree — master-requirements root, stakeholder, usecase, functional, non-functional, indexes, log, and a Test Matrix — that passes `quire validate` as rendered (TC-1439, TC-1443).
- [x] A Test Matrix whose every `Status` cell is drawn from the archetype's vocabulary, never `⚠️`, with `🚧` rows carrying reasons and every row tracing to a criterion that exists (TC-1440, TC-1441, TC-1442).
- [x] An Out of Scope section saying the module's domain types are the maintainer's to specify, and no requirement text copied from a maintained module repository (TC-1461).
