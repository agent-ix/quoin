---
id: SR-20261103118166
title: "Scope-boundary analysis of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

Scope-boundary lens: allocation of the new rules between quoin install, quoin inspect, the filament-core-data schema and the quire-rs loader. Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-070 says 'Quoin SHALL report `semantic.unused-import`' without naming the command, FR-070-AC-12 says only 'yields', and FR-075 (install) does not list it; only FR-124 (`inspect`) does. The PLAT-212 deliverables call it an install-time diagnostic. State whether `quoin module install` emits it (as a non-failing warning) as well as `inspect`, and make AC-12 name the command. | spec/functional/FR-070-semantic-module-manifest-extension.md:85 |
| FND-002 | medium | FR-070 rejects an `import_types` key absent from `imports` (a cross-key rule JSON Schema cannot express), but quire-rs#637 FR-069-AC-16 refuses only a non-array list or non-string entry, so Quire loads a manifest Quoin refuses at install, while FR-070 says Quire applies the same block schema. Either have quire-rs FR-069 refuse the same cross-key case or state in FR-070 that the cross-key check is install-only. | spec/functional/FR-070-semantic-module-manifest-extension.md:82 |
| FND-003 | low | Neither filament-core-data schema on origin/main admits `import_types`, and no filament-core-data PR or ticket for it exists. Under additionalProperties false and CON-2, Quoin refuses `import_types` as an unknown key until that schema change ships, so FR-070-AC-8 cannot pass. The Upstream line cites '(PLAT-212)', a quoin ticket. Cite the filament-core-data work item that adds `import_types`. | spec/functional/FR-070-semantic-module-manifest-extension.md:45 |

## Verdict

Unknown-import-type is cleanly allocated to install (FR-070/FR-075) and inspect (FR-124); schema ownership is correctly placed with filament-core-data. The defects are an unallocated phase for unused-import, a cross-key rule only one of the two loaders enforces, and an upstream schema change with no work item.

## Scope

- `FR-070` (examined): When a declared import, with or without a type list, is referenced by no `$ref` in the module's own emitted schemas, Quoin SHALL report the diagnostic `semantic.unused-import` (severity warning) naming the import.
- `FR-070-AC-12` (examined): An import, with and without a type list, referenced by no `$ref` under `https://schemas.agent-ix.org/<org>/<repo>/` in the module's emitted schemas yields `semantic.unused-import` at warning severity naming it, and an import that one such `$ref` references yields none.
- `FR-124` (examined): An entry's `diagnostics` SHALL be the diagnostics of reading its block (FR-070, FR-073, FR-074) followed by one error per `semantic.imports` entry that no other installed module provides at exactly the locked version, naming the import, the locked version and the installed versions, and one error per import cycle (named over types where type lists exist, FR-075), followed by one `semantic.unknown-import-type` error per `semantic.import_types` type the installed provider does not export and one `semantic.unused-import` warning per declared import no `$ref` in the module's emitted schemas references (FR-070).
- `FR-124-AC-9` (examined): `inspect` reports on an importer a `semantic.unknown-import-type` error naming the package and type for a type its installed provider does not export, and a `semantic.unused-import` warning for an unreferenced import; the error exits 1, and a run whose only finding is the warning exits zero.
- `FR-070` (examined): When a key of `semantic.import_types` names a package absent from `semantic.imports`, or its list is empty, holds a duplicate, or holds an empty string, Quoin SHALL reject the manifest as a shape error naming the package.
- `FR-070-AC-9` (examined): An `import_types` key absent from `imports`, an empty list, a duplicate type name and an empty type name are each rejected naming the package.
- `FR-070` (examined): The module-manifest JSON Schema, owned by `agent-ix/filament-core-data` (`schema/semantic/v1/module-manifest.schema.json` and `module-semantic-block.schema.json`, published as `@agent-ix/semantic-schema`), SHALL publish the block with `additionalProperties: false`.
- `FR-070-CON-2` (examined): Quoin SHALL take the module-manifest schema from the published `@agent-ix/semantic-schema` package at the version its lockfile records, with no copy kept.
- `FR-070-AC-8` (examined): A manifest with `imports: { "agent-ix/spec-artifacts-iso": "0.2.0" }` and no `import_types` loads exactly as it did before this amendment, and one with `import_types: { "agent-ix/spec-artifacts-iso": ["StR", "FR"] }` alongside that import loads and the pack reports the lists.
- `FR-078` (examined): The rendered `semantic` block SHALL omit the optional `import_types` key; an author adds it by hand (FR-070).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | quire-rs#637 FR-069 (head 25f7dbde) also refuses a module with `semantic.unknown-import-type` at `<section>[<type>].data_schema.schema` when a declared-import `$ref` in its own data schemas names a type the provider does not export. Quoin FR-070 checks only `import_types` lists, so `quoin module install` accepts a module that `quire load` refuses, and PLAT-212's 'a manifest naming an imported type the package does not export is refused at install' is met only for the list form. Add the `$ref` form of the check to FR-070 (with an AC), or state why install leaves it to the loader. | spec/functional/FR-070-semantic-module-manifest-extension.md:42 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-070 now states `quoin module install` emits `semantic.unused-import` as a non-failing warning and `inspect` lists it; AC-12 names install. |
| FND-002 | fixed | FR-070 states `quire load` refuses the same shapes, including the cross-key case, and quire-rs FR-069 head 25f7dbde refuses a key absent from `semantic.imports` with `semantic.invalid-value`. |
| FND-003 | fixed | The Upstream line now names the filament-core-data schema change and its release as delivered under PLAT-212, whose deliverables include that schema change, and FR-070 states install refuses the key until the lockfile records that release, so AC-8's dependency is explicit. |
