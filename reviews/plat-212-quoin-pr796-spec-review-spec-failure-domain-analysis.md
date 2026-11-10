---
id: SR-20261103118164
title: "Failure-domain analysis of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

Failure-domain lens over the new import_types, unknown-import-type, unused-import and type-level cycle rules, compared against the companion agent-ix/quire-rs#637 FR-069 text (head e2b6bc42). Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-075 keeps the unconditional package-level rule (any cycle over `semantic.imports` fails install) and the new bullet only changes how the cycle is named. quire-rs FR-069 makes the graph type-level when lists exist and loads a package back edge with no type-level cycle (FR-069-AC-20). The same two modules therefore fail `quoin module install` but load in Quire, contradicting the fixed decision 'type-level cycle when type lists exist'. The mixed case (only some modules on the cycle carry lists) is also undefined in Quoin, where quire-rs says a list-less package contributes a package node. Make detection type-level when lists exist, state the mixed rule, and add the back-edge-without-type-cycle case to FR-075-AC-7. | spec/functional/FR-075-semantic-package-exports-and-locks.md:31-32 |
| FND-002 | medium | The type-level graph has nodes (imported types) but no edges defined: `import_types` says module A uses B#T, not which of A's own types use it, so 'the types that form it' cannot be computed from the manifest alone. Two implementers would differ (e.g. edge from every A export vs. edge from the A type whose emitted schema `$ref`s B#T). State the edge rule (identically to quire-rs FR-069). | spec/functional/FR-075-semantic-package-exports-and-locks.md:32 |
| FND-003 | medium | Quoin has no 'loaded' state at install: FR-075 already fails install when no installed module provides the import, so the 'provider not loaded' branch is reachable only when install is failing for an unresolved import (and in `inspect`, where that is also an error). FR-070-AC-11 names no command and no expected overall outcome, so a test can pass by asserting the absence of one code on any run. Define 'not loaded' as 'not installed', and state AC-11 as: install fails with the unresolved-import error only, raising no `semantic.unknown-import-type`. | spec/functional/FR-070-semantic-module-manifest-extension.md:84 |
| FND-004 | medium | unused-import counts `$ref`s in the module's own *emitted* schemas (FR-077). A module whose data schemas are hand-authored files by path (FR-073), not FR-077 emissions, has no emitted schemas, so every import it declares would warn as unused. quire-rs counts the module's own *data schemas*. Use the data schemas the manifest ships (the FR-073 `data_schema` files), which covers emitted and hand-authored modules and matches quire-rs. | spec/functional/FR-070-semantic-module-manifest-extension.md:85 |

## Verdict

The shape-error, unknown-import-type and derived `exports` rules are well bounded and agree with quire-rs on code names (`semantic.unknown-import-type` error, `semantic.unused-import` warning). The defects are in the cycle rule, the undefined type-level edge, the provider-not-loaded branch at install, and what counts as the module's schemas for unused-import.

## Scope

- `FR-075` (examined): If the import graph over `semantic.imports` contains a cycle, then `quoin module install` SHALL fail naming the cycle.
- `FR-075` (examined): Where the modules on an import cycle carry `semantic.import_types` lists, the failure SHALL name the cycle over the types that form it (`<org>/<repo>:<Type>`), and otherwise over packages.
- `FR-075-AC-7` (examined): Two fixture modules importing each other's types fail install with a cycle named over the types that form it; the same two modules with no type lists fail naming the packages.
- `FR-070` (examined): When the provider package of an `import_types` entry is not loaded, Quoin SHALL NOT check its type list.
- `FR-070-AC-11` (examined): With the provider not loaded, the same `import_types` entry raises no `semantic.unknown-import-type`.
- `FR-075` (examined): Where the provider package of an `import_types` entry is not loaded, `quoin module install` SHALL NOT check that entry's types.
- `FR-075` (examined): If `semantic.imports` names a package that no installed module provides a version satisfying it, then `quoin module install` SHALL fail naming the import and the installed versions.
- `FR-070` (examined): When a declared import, with or without a type list, is referenced by no `$ref` in the module's own emitted schemas, Quoin SHALL report the diagnostic `semantic.unused-import` (severity warning) naming the import.
- `FR-070` (examined): A reference, for `semantic.unused-import`, is a `$ref` whose URL is under the imported package's schema base `https://schemas.agent-ix.org/<org>/<repo>/`.
- `FR-077` (examined): Where a `$ref` names a model of an imported semantic module, the emit command SHALL rewrite it to that module's schema base at the version `semantic.imports` declares, rather than to this module's base or to the semantic-core base; the rewritten `$ref` URLs are the references `semantic.unused-import` counts (FR-070).
- `FR-075` (examined): `quoin module install` SHALL fill each derived `imports` entry's `exports` with that import's declared `semantic.import_types` list as type names, and with `[]` when none is declared, so the type-level import graph is available to consumers of `package-manifest.json`.
- `FR-075-AC-6` (examined): For a fixture importing a package with `import_types: ["StR", "FR"]`, the derived `package-manifest.json` import entry carries `exports` naming `StR` and `FR`, and an import with no type list carries `exports: []`.
- `FR-070-AC-10` (examined): With the provider installed and its `exports` lacking `Nope`, an `import_types` entry listing `Nope` fails `quoin module install` with `semantic.unknown-import-type` naming the package and `Nope`.
