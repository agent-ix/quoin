---
id: SR-20261103118168
title: "Gap analysis of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

Planless gap analysis over the PR diff: new requirements vs the Test Matrix and code. Plan completion: not assessed. Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The PR has no production code changes. Every new AC maps to one new TC, and those TCs are honestly Pending with no tagged tests yet, as expected for a spec-first PR. No coverage inflation: the FR-070 and FR-075 rows moved from Covered to Pending. The PLAT-212 acceptance criteria are all specced: string-form imports unchanged (FR-070-AC-8), unknown type refused at install (FR-070-AC-10), unreferenced import diagnosed (FR-070-AC-12), and a type-level graph exposed with cycles named over types (FR-075-AC-6/7). The open spec defects are in the sibling SR files. The FR-124 coverage-row gap is base FND-001.

## Scope

- `FR-070-AC-8` (examined): A manifest with `imports: { "agent-ix/spec-artifacts-iso": "0.2.0" }` and no `import_types` loads exactly as it did before this amendment, and one with `import_types: { "agent-ix/spec-artifacts-iso": ["StR", "FR"] }` alongside that import loads and the pack reports the lists.
- `FR-070-AC-10` (examined): With the provider installed and its `exports` lacking `Nope`, an `import_types` entry listing `Nope` fails `quoin module install` with `semantic.unknown-import-type` naming the package and `Nope`.
- `FR-070-AC-12` (examined): An import, with and without a type list, referenced by no `$ref` under `https://schemas.agent-ix.org/<org>/<repo>/` in the module's emitted schemas yields `semantic.unused-import` at warning severity naming it, and an import that one such `$ref` references yields none.
- `FR-075-AC-6` (examined): For a fixture importing a package with `import_types: ["StR", "FR"]`, the derived `package-manifest.json` import entry carries `exports` naming `StR` and `FR`, and an import with no type list carries `exports: []`.
- `FR-075-AC-7` (examined): Two fixture modules importing each other's types fail install with a cycle named over the types that form it; the same two modules with no type lists fail naming the packages.
- `FR-124-AC-9` (examined): `inspect` reports on an importer a `semantic.unknown-import-type` error naming the package and type for a type its installed provider does not export, and a `semantic.unused-import` warning for an unreferenced import; the error exits 1, and a run whose only finding is the warning exits zero.
- `spec/matrix.md` (examined): TC-2130..TC-2137 added to the Test Case Summary (TC-2130..2134 -> FR-070-AC-8..12, TC-2135 -> FR-124-AC-9, TC-2136/2137 -> FR-075-AC-6/7), all 🚧 Pending; FR-070 and FR-075 coverage rows extended and set to 🚧 Pending.
