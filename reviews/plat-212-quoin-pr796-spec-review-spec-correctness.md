---
id: SR-20261103118167
title: "Spec-correctness pass of PLAT-212 import_types (quoin#796)"
type: SpecReview
analysis: spec-correctness
scope: "agent-ix/quoin; spec/functional/FR-070, FR-075, FR-077, FR-078, FR-124; spec/matrix.md (PR #796 diff origin/main...6c8c1394)"
review_set: subset
---

## Summary

`quire properties --json` over the new ACs (FR-070-AC-8..12, FR-075-AC-6/7, FR-124-AC-9) to check which ones a property test can be grounded on. Ticket: PLAT-212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

FR-070-AC-8, AC-9 and AC-12 classify as extractable universal assertions. AC-9 carries a domain (`import_types` key), a precondition and an oracle span. The rest are specific-scenario examples, which is right for fixture-driven install and inspect tests. No property test was emitted: this is a spec-only PR and there is no implementation entry point to ground the domain or oracle against. That is expected at this stage, not a defect. Defects in the AC text itself are recorded under failure-domain, integrity and scope-boundary.

## Scope

- `FR-070-AC-8` (examined): A manifest with `imports: { "agent-ix/spec-artifacts-iso": "0.2.0" }` and no `import_types` loads exactly as it did before this amendment, and one with `import_types: { "agent-ix/spec-artifacts-iso": ["StR", "FR"] }` alongside that import loads and the pack reports the lists.
- `FR-070-AC-9` (examined): An `import_types` key absent from `imports`, an empty list, a duplicate type name and an empty type name are each rejected naming the package.
- `FR-070-AC-10` (examined): With the provider installed and its `exports` lacking `Nope`, an `import_types` entry listing `Nope` fails `quoin module install` with `semantic.unknown-import-type` naming the package and `Nope`.
- `FR-070-AC-11` (examined): With the provider not loaded, the same `import_types` entry raises no `semantic.unknown-import-type`.
- `FR-070-AC-12` (examined): An import, with and without a type list, referenced by no `$ref` under `https://schemas.agent-ix.org/<org>/<repo>/` in the module's emitted schemas yields `semantic.unused-import` at warning severity naming it, and an import that one such `$ref` references yields none.
- `FR-075-AC-6` (examined): For a fixture importing a package with `import_types: ["StR", "FR"]`, the derived `package-manifest.json` import entry carries `exports` naming `StR` and `FR`, and an import with no type list carries `exports: []`.
- `FR-075-AC-7` (examined): Two fixture modules importing each other's types fail install with a cycle named over the types that form it; the same two modules with no type lists fail naming the packages.
- `FR-124-AC-9` (examined): `inspect` reports on an importer a `semantic.unknown-import-type` error naming the package and type for a type its installed provider does not export, and a `semantic.unused-import` warning for an unreferenced import; the error exits 1, and a run whose only finding is the warning exits zero.
