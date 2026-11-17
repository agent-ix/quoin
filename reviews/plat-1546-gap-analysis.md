---
id: SR-20261116075210
title: "Gap analysis — quoin PLAT-1546 threat-model dossier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; repository matrix, spec/functional/FR-202-dossier-threat-model-content.md, rust/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-202", type: references }
---

## Summary

Planless gap analysis of quoin's computed repository matrix and the PLAT-1546 implementation surface. The matrix leaves all 16 FR-202 acceptance criteria unbound; source contains trace attributes for 11 new tests, but Quoin does not bind them, and five criteria have no corresponding trace-tagged test.

## Verdict

**FAIL** — FR-202-AC-1 through FR-202-AC-16 are all untagged in the computed matrix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-202-AC-1 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-1, rust/crates/quoin-dossier/src/threat_model.rs:615 |
| FND-002 | high | FR-202-AC-2 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-2, rust/crates/quoin-dossier/src/threat_model.rs:691 |
| FND-003 | high | FR-202-AC-3 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-3, rust/crates/quoin-dossier/src/threat_model.rs:810 |
| FND-004 | high | FR-202-AC-4 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-4, rust/crates/quoin-dossier/src/threat_model.rs:869 |
| FND-005 | high | FR-202-AC-5 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-5, rust/crates/quoin-dossier/src/threat_model.rs:738 |
| FND-006 | high | FR-202-AC-6 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-6, rust/crates/quoin-dossier/src/threat_model.rs:921 |
| FND-007 | high | FR-202-AC-7 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-7, rust/crates/quoin-dossier/src/threat_model.rs:921 |
| FND-008 | high | FR-202-AC-8 has no trace-tagged test | FR-202-AC-8 |
| FND-009 | high | FR-202-AC-9 has no trace-tagged test | FR-202-AC-9 |
| FND-010 | high | FR-202-AC-10 has no trace-tagged test | FR-202-AC-10 |
| FND-011 | high | FR-202-AC-11 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-11, rust/crates/quoin-dossier/src/threat_model.rs:942 |
| FND-012 | high | FR-202-AC-12 has no trace-tagged test | FR-202-AC-12 |
| FND-013 | high | FR-202-AC-13 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-13, rust/crates/quoin-dossier/src/threat_model.rs:810 |
| FND-014 | high | FR-202-AC-14 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-14, rust/crates/quoin-dossier/src/threat_model.rs:942 |
| FND-015 | high | FR-202-AC-15 has no binder in the computed matrix; its tagged test is reported unbound | FR-202-AC-15, rust/crates/quoin-dossier/src/threat_model.rs:747 |
| FND-016 | high | FR-202-AC-16 has no trace-tagged test | FR-202-AC-16 |
| FND-017 | medium | Repository coverage reports 290 trace-tagged symbols that bind to no declared criterion | rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:730, rust/crates/quoin-cli/src/advise.rs:222 |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.35.0) and `quire matrix --scope . --format json` (quire 0.45.0, engine 0.60.0); both were run on the full repository. `quire matrix --scope . --strict` exited 1, reporting 1,973 untagged criteria.
- Matrix status counts from Quoin: 2,171 criteria total; 1,973 untagged, 18 tagged, 143 method-without-symbol, 37 retired. Every criterion had `no run evidence`.
- FR-202: 16/16 criteria untagged. Source has `#[trace]` on tests for AC-1–7, AC-11, AC-13–15, yet the matrix gives them no binders and `quire coverage` reports their tags as `tagged-without-citing-matrix-row`; AC-8, AC-9, AC-10, AC-12 and AC-16 have no trace-tagged tests.
- `quire coverage --scope . --json` exited 0 and reported 290 untracked symbols and 3,881 diagnostics. No untracked symbol named FR-202. The diagnostic volume and mismatch between present Rust `#[trace]` attributes and zero matrix binders make repository-wide matrix output inconsistent with the local Rust trace convention; the raw FR-202 untagged result is preserved above.
- Changed behavior inventory: CLI source adapter, flow-report adapter, dossier generator/registration, and four gap kinds; each maps to FR-202. No unowned changed behavior or source/test stub found in the changed files.
- Plan completion: not assessed
- Semantic review: skipped (not opted in)
