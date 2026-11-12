---
id: SR-20261112021201
title: "Spec review of quoin PR 825 (PLAT-1466 Filament IDE removal: FR-024, FR-030, FR-043, TC-268, PLAN-003)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@1b6ce909; spec/functional/FR-024-plugin-catalog-library-api.md, spec/functional/FR-030-evidence-store.md, spec/functional/FR-043-quality-benchmark.md, spec/matrix.md (TC-268), plan/PLAN-003-operational-evidence/plan.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-024"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-043"
    type: "reviews"
---

# Spec review of quoin PR 825

## Summary

Ticket: PLAT-1466. This review checks the diff only.

FR-024 Description: the parenthetical "(as filament-ide does, to avoid pulling quoin's CLI
dependency tree into a desktop bundle)" becomes "(to avoid pulling quoin's CLI dependency tree
into its bundle)". "Its" refers to "a consumer". In Dependencies, "Downstream: filament-ide
interoperates …" becomes "store-level consumers interoperate …", and the two body `ix://` links
to `agent-ix/filament-ide/FR-023` and `FR-020` are dropped. Those were body links, not
frontmatter `relationships`. FR-024's frontmatter edges (`traces_to` StR-003, `implements`
US-003) are unchanged.

Neither edit touches a `SHALL` statement or an acceptance criterion. The store-level
interop path is still allowed as an alternative to importing the library, so the requirement
means the same thing. A store-level consumer still has to keep the same registry and module
directory layout.

The other edits:

- **FR-030:** the indexing owner changes from `filament-ide-rs` to "the consuming application",
  and the no-SQLite rule is unchanged.
- **FR-043:** "Battletest pass 2 against a large real Rust corpus" keeps the historical
  measurements and drops the repo name.
- **TC-268:** the row drops "(filament-ide-rs FR-015/NFR-009)". The verbatim statements live in
  the bound test `architectural_claims_mint_layering_or_module_boundary`, and the row's clause
  and its binding to FR-031-AC-21 are unchanged. `quoin matrix --json` reports no
  `criterion-tagged-outside-row`.
- **PLAN-003:** the out-of-plan list drops `filament-ide-rs`.

`quire validate --scope .` passes for FR-024, FR-030, FR-043 and PLAN-003. The one advisory
warning is FR-030 line 149, `be able to`, which this diff does not touch. `spec/matrix.md` fails
validation only on the TC-1943 and TC-1944 tombstone rows at lines 1315 and 1316. Those rows are
already on origin/main and this diff does not touch them.

## Verdict

Clean for the diff. No requirement meaning changed, the relationship edges are intact, and the
removed links were to an out-of-scope repo.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
