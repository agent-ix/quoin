---
id: SR-20261103118165
title: "Integrity analysis of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

Integrity lens: internal and cross-repo consistency of the amended FR-070/075/077/078/124 statements, constraints CON-1/CON-2 and their AC/TC coverage. Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-075 names type-level cycle nodes `<org>/<repo>:<Type>`; the companion quire-rs#637 FR-069 names them `<org>/<repo>#<Type>`. The chosen form is `<org>/<repo>#<Type>`. Change FR-075 (and any TC text that spells the node, currently none) to `#` so both engines print the same cycle. | spec/functional/FR-075-semantic-package-exports-and-locks.md:32 |
| FND-002 | low | The unknown-import-type refusal and the provider-not-loaded exemption are stated normatively twice, in FR-070 and again in FR-075, with slightly different wording ('installed provider package' vs 'provider package'). Keep the rule in one FR (FR-070 owns the diagnostic) and have FR-075 reference it, so the two cannot drift. | spec/functional/FR-075-semantic-package-exports-and-locks.md:35-36 |
| FND-003 | low | The new FR-078 statement 'The rendered `semantic` block SHALL omit the optional `import_types` key' has no AC or TC. Add an AC (rendered manifest has no `import_types` key) or fold it into an existing FR-078 render AC. | spec/functional/FR-078-generated-manifest-semantic-block.md:53 |

## Verdict

FR-070-CON-1 stays true: `import_types` is optional and adds no required key anywhere. FR-070-CON-2 is consistent with the amendment (the schema is read from the published package, no copy), and the derived import `exports` (strings, minLength 1, uniqueItems) fits filament-core-data package-manifest.schema.json. FR-077 and FR-124 additions are consistent with FR-070. The defects are a cross-repo naming mismatch, duplicated normative text, and one untested new statement.

## Scope

- `FR-070-CON-1` (examined): The `semantic` block (including `import_types`) SHALL add no required key to the manifest root, `ObjectTypeEntry`, `ArchetypeEntry`, or `ArtifactTypeEntry`.
- `FR-070-CON-2` (examined): Quoin SHALL take the module-manifest schema from the published `@agent-ix/semantic-schema` package at the version its lockfile records, with no copy kept.
- `FR-070` (examined): `semantic.import_types` SHALL be an optional key that maps `<org>/<repo>` to a non-empty array of unique, non-empty type-name strings, naming the types of that package the module uses.
- `FR-070` (examined): `semantic.imports` SHALL keep its shape `{ "<org>/<repo>": "<exact semver>" }`, so a manifest that declares imports and no `import_types` validates and behaves exactly as before.
- `FR-075` (examined): Where the modules on an import cycle carry `semantic.import_types` lists, the failure SHALL name the cycle over the types that form it (`<org>/<repo>:<Type>`), and otherwise over packages.
- `FR-070` (examined): When a type listed in `semantic.import_types` is not named by the `exports` of the installed provider package, Quoin SHALL refuse `quoin module install` with the diagnostic `semantic.unknown-import-type` (severity error), naming the package and the type.
- `FR-075` (examined): If a type listed in `semantic.import_types` is not named by the provider package's `exports`, then `quoin module install` SHALL fail with `semantic.unknown-import-type` (FR-070) naming the package and the type.
- `FR-070` (examined): When the provider package of an `import_types` entry is not loaded, Quoin SHALL NOT check its type list.
- `FR-075` (examined): Where the provider package of an `import_types` entry is not loaded, `quoin module install` SHALL NOT check that entry's types.
- `FR-078` (examined): The rendered `semantic` block SHALL omit the optional `import_types` key; an author adds it by hand (FR-070).
- `FR-077` (examined): Where a `$ref` names a model of an imported semantic module, the emit command SHALL rewrite it to that module's schema base at the version `semantic.imports` declares, rather than to this module's base or to the semantic-core base; the rewritten `$ref` URLs are the references `semantic.unused-import` counts (FR-070).
- `FR-124` (examined): An entry's `diagnostics` SHALL be the diagnostics of reading its block (FR-070, FR-073, FR-074) followed by one error per `semantic.imports` entry that no other installed module provides at exactly the locked version, naming the import, the locked version and the installed versions, and one error per import cycle (named over types where type lists exist, FR-075), followed by one `semantic.unknown-import-type` error per `semantic.import_types` type the installed provider does not export and one `semantic.unused-import` warning per declared import no `$ref` in the module's emitted schemas references (FR-070).
- `FR-075` (examined): `quoin module install` SHALL fill each derived `imports` entry's `exports` with that import's declared `semantic.import_types` list as type names, and with `[]` when none is declared, so the type-level import graph is available to consumers of `package-manifest.json`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-075, FR-124 and TC-2137 now spell type-level nodes `<org>/<repo>#<Type>`, matching quire-rs FR-069. |
| FND-002 | fixed | FR-075 now defers to the rules 'stated once in FR-070' for `semantic.unknown-import-type` and the unresolved-import error. |
| FND-003 | fixed | FR-078-AC-9 and TC-2138 added for the omitted `import_types` key. |
