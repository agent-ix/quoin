---
id: SR-17077
title: "Gap analysis of quoin PR 798 (remove the semantic-module template and tools package)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@9a6555ac; spec/functional/FR-070, FR-073, FR-075, FR-124; spec/matrix.md; rust/crates/quoin-semantic/tests"
review_set: subset
---

# Gap analysis of quoin PR 798

## Summary

Ticket: PLAT-1385. Plan completion: not assessed. Question: did any removed requirement own
behaviour that survives in quoin. Checked the deleted FR-077 (`$ref` rewrite to imported bases,
unused-import counting) and FR-078 (manifest `semantic` block; AC-9 = TC-2138, no `import_types`)
against the surviving owners. FR-070 Behaviour lines 43-45 and AC-12/AC-13 own
`semantic.unused-import` and `semantic.unknown-import-type` over a module's own data schemas,
emitted or hand-written; FR-075 owns the `$ref`-derived type-level import graph; FR-073 owns `$ref`
resolution; FR-124 AC-9 owns the installed-inspection diagnostics. None of them cites a removed id,
and none depends on the emit tool. TC-2138 asserted only the rendered template; `import_types` is
owned by FR-070. NFR-020 scoped only the renderer and rendered-repo toolchain. All 68 removed TC
rows and 15 removed requirement/VC rows cite only removed ids; no surviving AC lost its row.

## Verdict

**CLEAN.** No surviving behaviour lost its requirement, criterion or test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
