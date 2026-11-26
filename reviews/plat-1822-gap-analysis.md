---
id: SR-967175026302197768900474181941935084767
title: "Gap analysis — Quoin design-matrix error preservation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; rust/crates/quoin-core/src/ops/quire/{mod.rs,tests.rs}, rust/crates/quoin-quire/src/{design_matrix.rs,evidence.rs}, rust/crates/quoin-cli/src/{analysis_evidence.rs,matrix.rs}, rust/crates/quoin-dossier/src/architecture.rs; spec/functional/FR-165-matrix-design-column.md, FR-193-dossier-architecture-content.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-165", type: references }
  - { target: "ix://agent-ix/quoin/FR-193", type: references }
---

## Summary

This bounded gap check examined the changed FR-165 error-preservation path and its direct design-matrix consumers. The new FR-165-AC-21 trace is backed by an assertion that checks valid entries and dropped-relation errors together; the architecture dossier caller still drops the error collection.

## Verdict

**CONDITIONAL** — the focused trace is present, but the direct architecture consumer can produce an incomplete report without surfacing a dropped relation.

## Coverage

- Plan completion: not assessed.
- Computed repository-wide Test Matrix: not run. The release brief prohibits whole-suite or long validation sweeps; this report does not claim a repository-wide gap audit.
- Focused matrix-to-test check: FR-165-AC-21 is tagged on `design_matrix_preserves_errors_alongside_valid_sibling_entries`; its assertions verify that the `coverage_matrix` contains the valid entry and `design_matrix_errors` contains the dropped relation.
- Direct consumer check: `ArchitectureGenerator::generate` reads `outcome.design_matrix.entries` but does not read `outcome.design_matrix.errors`. With valid entries and a dropped relation together, the architecture report can show valid relations while silently omitting the dropped relation. FR-193 does not define the dropped-relation case, and no dossier test exercises it.
- Stub and reverse-gap census: limited to the changed operation and its direct consumers; no repository-wide census was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-193 does not specify how architecture dossier generation handles `DesignMatrix.errors`, and its generator ignores them. A partial engine result can therefore omit a declared relation while retaining valid entries, without adding a gap or diagnostic to the dossier. Define and test the handling, then consume the errors in the generator. | rust/crates/quoin-dossier/src/architecture.rs:64; spec/functional/FR-193-dossier-architecture-content.md:24 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-193 defines dropped-relation reporting and the generator emits a gap per error; focused FR-193-AC-10/11 tests cover partial and error-only results. |

| FND-001 | fixed | Reconfirmed at PR head `4f6c1c48d1de019d2d29e472f52e01b62f3c47ce`; FR-193 still traces the error handling, and the v0.63.1 matrix API retains both valid entries and errors. |
