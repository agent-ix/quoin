---
id: SR-20261103118159
title: spec review of the PLAT-195 spec edits
type: SpecReview
analysis: base
scope: agent-ix/quoin; spec/functional/FR-076-semantic-module-template-variants.md; spec/functional/FR-082-generated-governance-tree.md; spec/matrix.md; templates/semantic-module/{{cookiecutter.repo_name}}/spec/non-functional/NFR-001-verification-without-skips.md
review_set: subset
date: '2026-11-10'
---

# spec review of the PLAT-195 spec edits

## Summary

Ticket: PLAT-195 (agent-ix/quoin PR #793, head a97402df).

Covers the edits to FR-076 (Notes and the new CON-4), the FR-082 note, the matrix row deletion, and the AC-2 note in the template's NFR-001.

## Scope

- `FR-076-CON-4` (examined): "The rendered repository SHALL depend on `@agent-ix/semantic-module-tools` at an exact version in `package.json` `devDependencies` and SHALL contain no local implementation of the schema emit pipeline."
- `FR-076#notes` (examined): driver paragraph rewritten to name the package
- `FR-076-CON-1` (context_only)
- `FR-082#notes` (examined): absent-tool inspection paths
- `matrix#TC-1456` (examined): deleted row for FR-076-CON-3, which is not defined on origin/main
- `NFR-001#notes` (examined): template AC-2 inspection paths

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-082 note now reads "the absent-tool paths in `tests/conftest.py`, and `tests/test_schema_emission.py`". The comma before "and" is left over from the old three-item list. | spec/functional/FR-082-generated-governance-tree.md:86-87 |

## Verdict

The spec edits are consistent with each other and with the code.

- CON-4 is atomic and testable, and tc_264_056 backs it.
- CON-1 still holds: the driver is now reached as a versioned dependency.
- The FR-076 Notes paragraph no longer defers the extraction to quoin#345.
- TC-1456 was a byte-identity row for a CON-3 that origin/main does not define. Deleting it removes a dangling ceremony row.
- The FR-082 and NFR-001 notes now name only paths that exist in a rendered repository.
- Skipping the number CON-3 avoids reusing a retired id.

Mergeable with respect to this method; FND-001 is a nit.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The stray comma is gone: the note now reads "the absent-tool paths in `tests/conftest.py` and `tests/test_schema_emission.py`". |
