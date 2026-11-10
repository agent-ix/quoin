---
id: SR-20261103118163
title: "Base spec review of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

Base checklist over the PR #796 diff (FR-070, FR-075, FR-077, FR-078, FR-124, spec/matrix.md): ID formats, AC/TC numbering and uniqueness, coverage rules, and stale-reference replacement. Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-124-AC-9 (TC-2135) is added only to the Test Case Summary; spec/matrix.md has no FR-124 row in Functional Requirement Coverage, so the new AC (like FR-124-AC-1..8, a pre-existing gap) is invisible in the coverage table. Add an FR-124 coverage row listing its ACs and TCs. | spec/functional/FR-124-inspect-installed-semantic-modules.md:77 |

## Verdict

ID formats are correct and unique: FR-070-AC-8..12 continue AC-7, FR-075-AC-6/7 continue AC-5 without reusing the withdrawn AC-2, FR-124-AC-9 continues AC-8, and TC-2130..TC-2137 are new and unique repo-wide (TC-2129 is unused, a harmless gap). Every new AC has exactly one TC row, all honestly marked Pending, and the FR-070/FR-075 coverage rows were moved from Covered to Pending. The stale `filament-core-service#21` schema-owner reference is correctly replaced: origin/main of agent-ix/filament-core-data carries `schema/semantic/v1/module-manifest.schema.json` (inline `semantic` block, additionalProperties false) and `module-semantic-block.schema.json`, and `schema/package.json` is named `@agent-ix/semantic-schema`. No hash/digest/pin use is introduced. `quire validate` passes on all five FR files (5/5 grammar-clean). `quire validate spec/matrix.md` fails only on TC-1943/TC-1944 (Priority/Type `-`, Status `Tombstone.`); the identical failure reproduces on origin/main a134c72f, the rows came from #716 (859068d7), and the PR does not touch them, so it is pre-existing and not caused by this PR.

## Scope

- `FR-070-AC-8` (examined): A manifest with `imports: { "agent-ix/spec-artifacts-iso": "0.2.0" }` and no `import_types` loads exactly as it did before this amendment, and one with `import_types: { "agent-ix/spec-artifacts-iso": ["StR", "FR"] }` alongside that import loads and the pack reports the lists.
- `FR-070-AC-9` (examined): An `import_types` key absent from `imports`, an empty list, a duplicate type name and an empty type name are each rejected naming the package.
- `FR-070-AC-10` (examined): With the provider installed and its `exports` lacking `Nope`, an `import_types` entry listing `Nope` fails `quoin module install` with `semantic.unknown-import-type` naming the package and `Nope`.
- `FR-070-AC-11` (examined): With the provider not loaded, the same `import_types` entry raises no `semantic.unknown-import-type`.
- `FR-070-AC-12` (examined): An import, with and without a type list, referenced by no `$ref` under `https://schemas.agent-ix.org/<org>/<repo>/` in the module's emitted schemas yields `semantic.unused-import` at warning severity naming it, and an import that one such `$ref` references yields none.
- `FR-075-AC-6` (examined): For a fixture importing a package with `import_types: ["StR", "FR"]`, the derived `package-manifest.json` import entry carries `exports` naming `StR` and `FR`, and an import with no type list carries `exports: []`.
- `FR-075-AC-7` (examined): Two fixture modules importing each other's types fail install with a cycle named over the types that form it; the same two modules with no type lists fail naming the packages.
- `FR-124-AC-9` (examined): `inspect` reports on an importer a `semantic.unknown-import-type` error naming the package and type for a type its installed provider does not export, and a `semantic.unused-import` warning for an unreferenced import; the error exits 1, and a run whose only finding is the warning exits zero.
- `FR-070` (examined): The module-manifest JSON Schema, owned by `agent-ix/filament-core-data` (`schema/semantic/v1/module-manifest.schema.json` and `module-semantic-block.schema.json`, published as `@agent-ix/semantic-schema`), SHALL publish the block with `additionalProperties: false`.
- `spec/matrix.md` (examined): TC-2130..TC-2137 added to the Test Case Summary (TC-2130..2134 -> FR-070-AC-8..12, TC-2135 -> FR-124-AC-9, TC-2136/2137 -> FR-075-AC-6/7), all 🚧 Pending; FR-070 and FR-075 coverage rows extended and set to 🚧 Pending.
