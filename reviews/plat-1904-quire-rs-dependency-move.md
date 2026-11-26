# PLAT-1904 quire-rs dependency move record

## Revisions

- Quoin old dependency: quire-rs `v0.63.1`, commit `bd797be83f643a31e3a9c86580a06b4f6dbfa6bb`.
- Quoin new dependency: package `0.63.2`, exact public-main revision `d7d418fa2727f7de87a16dfec5b38ada7d9ee776` (`PLAT-1897: Remove duplicate implements import`).
- The `v0.63.2` tag points to `1d8381188b37a95f0fe4cd6b7c57389058ac45b1`, not the requested merged main revision. Quoin pins the exact requested main revision; this does not create or move an engine tag.

The move consumes Quire's `coverage::compute_coverage_for_design_matrix`, which retains valid coverage entries when a required-relation declaration was dropped, while ordinary coverage remains strict. The design-matrix path allows only dropped `RequiredRelation` declarations through its precheck; all other dropped declarations still return the existing QQ-1022 error.

## Upstream changed files (`git diff --name-status old new`)
- `.github/workflows/wheels.yml` (M)
- `CHANGELOG.md` (M)
- `Cargo.lock` (M)
- `Cargo.toml` (M)
- `build.rs` (A)
- `crates/quire-fixtures/fixtures/mapping/cell-cases.json` (M)
- `crates/quire-fixtures/fixtures/mapping/config-version.expected.json` (M)
- `crates/quire-fixtures/fixtures/mapping/operations.expected.json` (M)
- `reviews/plat-1449-quire-rs-code-review.md` (A)
- `reviews/plat-1449-quire-rs-spec-review-base.md` (A)
- `reviews/plat-1449-quire-rs-spec-review-dependency.md` (A)
- `reviews/plat-1449-quire-rs-spec-review-integrity.md` (A)
- `reviews/plat-1848-code-review.md` (A)
- `reviews/plat-1848-spec-review-base.md` (A)
- `reviews/plat-1888-spec-review-base.md` (A)
- `reviews/plat-1888-spec-review-dependency.md` (A)
- `reviews/plat-1888-spec-review-ears-conformance.md` (A)
- `reviews/plat-1888-spec-review-evidence.md` (A)
- `reviews/plat-1888-spec-review-failure-domain.md` (A)
- `reviews/plat-1888-spec-review-integrity.md` (A)
- `reviews/plat-1888-spec-review-scope-boundary.md` (A)
- `reviews/plat-1889-code-review.md` (A)
- `reviews/plat-1889-gap-analysis.md` (A)
- `reviews/plat-1897-duplicate-trace-dependency-review.md` (A)
- `reviews/plat-1897-pr750-code-review.md` (A)
- `reviews/plat-1898-code-review.md` (A)
- `reviews/plat-1898-gap-analysis.md` (A)
- `reviews/plat-1904-code-review.md` (A)
- `reviews/plat-1904-gap-analysis.md` (A)
- `reviews/plat-1904-spec-review-base.md` (A)
- `reviews/plat-1904-spec-review-integrity.md` (A)
- `reviews/plat-1915-code-review.md` (A)
- `reviews/plat-1915-gap-analysis.md` (A)
- `reviews/plat-1916-code-review.md` (A)
- `reviews/plat-1916-gap-analysis.md` (A)
- `reviews/pr730/plat-1884-code-review.md` (A)
- `reviews/pr730/plat-1884-gap-analysis.md` (A)
- `reviews/pr730/plat-1884-spec-review-base.md` (A)
- `reviews/pr730/plat-1884-spec-review-dependency.md` (A)
- `reviews/pr730/plat-1884-spec-review-ears-conformance.md` (A)
- `reviews/pr730/plat-1884-spec-review-evidence.md` (A)
- `reviews/pr730/plat-1884-spec-review-failure-domain.md` (A)
- `reviews/pr730/plat-1884-spec-review-integrity.md` (A)
- `reviews/pr730/plat-1884-spec-review-scope-boundary.md` (A)
- `reviews/pr732/plat-1640-code-review.md` (A)
- `schemas/output/semantic-v1.schema.json` (M)
- `spec/functional/FR-040-object-edge-vocabulary.md` (M)
- `spec/functional/FR-062-implements-relation.md` (M)
- `spec/functional/FR-069-semantic-module-contract-at-load.md` (M)
- `spec/functional/FR-076-relationships-extraction.md` (M)
- `spec/functional/FR-077-trace-search-index.md` (M)
- `spec/functional/FR-081-record-sections-extraction.md` (M)
- `spec/functional/FR-092-matrix-design-column.md` (M)
- `spec/functional/FR-098-embedded-anomaly-documents.md` (A)
- `spec/functional/FR-099-diff-impact-analysis.md` (A)
- `spec/functional/index.md` (M)
- `spec/usecase/US-021-consume-quire-rs-anomaly-documents.md` (A)
- `spec/usecase/US-022-review-source-change-impact.md` (A)
- `spec/usecase/index.md` (M)
- `spec/validation/FRR-001-core-process-risk.md` (M)
- `spec/validation/FRR-002-security-reporting.md` (M)
- `src/anomaly_sources.rs` (A)
- `src/corpus/validate.rs` (M)
- `src/coverage.rs` (M)
- `src/coverage/reconcile.rs` (M)
- `src/diagnostic.rs` (M)
- `src/impact.rs` (A)
- `src/lib.rs` (M)
- `src/loader/mod.rs` (M)
- `src/loader/paths.rs` (M)
- `src/parser/slug.rs` (M)
- `src/registry.rs` (M)
- `src/selftest.rs` (M)
- `src/semantic/context.rs` (M)
- `src/semantic/model.rs` (M)
- `src/semantic/properties.rs` (M)
- `src/semantic/record.rs` (M)
- `src/semantic/relations.rs` (M)
- `src/semantic/surface.rs` (M)
- `src/symbols/trace.rs` (M)
- `src/validate_document.rs` (M)
- `tests/fixtures/corpus_cases/plat-1916-semantic-type-index.json` (A)
- `tests/fixtures/loader-home/foo/README.md` (A)
- `tests/fixtures/loader-home/foo/loader-fixture/examples/fixture.record.json` (A)
- `tests/fixtures/loader-home/foo/loader-fixture/manifest.yaml` (A)
- `tests/fixtures/loader-home/foo/loader-fixture/schemas/FR.schema.json` (A)
- `tests/fixtures/semantic/cases.expected.json` (M)
- `tests/it/coverage_rollup.rs` (M)
- `tests/it/relation_report.rs` (M)
- `tests/it/semantic_contract.rs` (M)
- `tests/it/semantic_model.rs` (M)
- `tests/it/semantic_properties.rs` (M)
- `tests/it/semantic_record_sections.rs` (M)
- `tests/it/semantic_surface.rs` (M)
- `tests/it/semantic_systems.rs` (M)

## Public API changes and Quoin references

New public items across the dependency range:

- `anomaly_sources`
- `coverage::compute_coverage_for_design_matrix`
- `impact` module: `LineRange`, `SourceSnapshot`, `FileChange`, `ImpactInput`, `RevisionSide`, `ChangedSymbol`, `UnmappedChange`, `ImpactReason`, `SelectedTest`, `RelatedImplementation`, `ImpactFinding`, `ImpactReport`, `DependencyMoveInput`, `UpstreamChange`, `DependencyLocalReference`, `DependencyTestReason`, `DependencyMoveReport`, `analyze_dependency_move`, `ImpactError`, and `analyze`
- `EmbeddedAnomalyDocument` and `embedded_anomaly_documents`
- `validate_document_in_indexed_bundle`

A search of Quoin Rust source for those names found `compute_coverage_for_design_matrix` only: `rust/crates/quoin-quire/src/design_matrix.rs` calls it, and `rust/crates/quoin-quire/tests/fr029_dropped_declarations.rs` exercises that integration. The new anomaly, impact-analysis, embedded-document, and indexed-bundle APIs have no Quoin references. Existing `compute_coverage` and `compute_with_evidence` call sites remain in their strict reporting paths.

An exact-path search for the 95 changed upstream file paths found 17 paths mentioned in Quoin source/spec/review files, mostly in earlier review records. None of those references are source imports by upstream file path; they are dependency notes, review context, or shared filenames such as `Cargo.toml` and `Cargo.lock`. No additional Quoin call site for the new APIs was found.

## Tests and checks

- `cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 --no-deps` passed before the new upstream fix; full metadata for `d7d418fa` is being checked.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` and `git diff --check` passed.
- Focused matrix regression and strictness tests have not yet run against `d7d418fa`; the prior queued attempt against `42bbf796` was canceled before it started, after an upstream compile defect was reported. The focused tests will run through `locked-build.sh` against this corrected revision.

## PLAT-1904 host-path blocker and proposed engine contract

The current Quoin command path is `quoin matrix` → `quoin-cli::analysis_evidence::compute_for_matrix` → `quoin_quire::evidence::compute_for_matrix` → `compute_with`. That path calls strict `coverage::walk` and then `quire_rs::coverage::compute_with_evidence`; it does not call `design_matrix::compute`. The new engine `compute_coverage_for_design_matrix` API accepts a plain coverage request and has no evidence-backing counterpart, so the current standalone Quoin integration does not fix the issue's real host path. PR #1095 remains draft and is not mergeable until this is resolved.

Minimal engine API proposal: add an evidence-aware matrix coverage entry point (for example `compute_coverage_with_evidence_for_design_matrix`) that accepts the same `Spec`, `Registry`, `SymbolGraph`, scope, and `BackingInput` as `compute_with_evidence`. It may ignore only dropped `RequiredRelation` declarations, preserving the current `DeclarationsDropped` refusal for every other kind. The returned report must retain evidence backing and coverage for valid sibling relations; the existing design-matrix report must carry one dropped-relation error for each dropped required relation.

Minimal real-host acceptance test after that API is available:

1. Read `tests/fixtures/corpus_cases/plat-1856.json` from quire-rs at the selected revision.
2. Materialize `input.module_manifest` unchanged as `module/manifest.yaml`; materialize every `input.documents` entry under `repo/spec/`.
3. Run `quoin matrix --repo <repo> --module <module> --no-project-config --json` using the built Quoin binary.
4. Assert exit 0 and JSON output retaining `requirement-has-design`; assert one `design_matrix_errors` entry for `connection-carries-classification`; exercise the architecture dossier path to assert a `design_relation_dropped` gap and no `design_relations_unavailable` gap.

The issue's recorded v0.63.1 reproduction exits 2 with empty stdout and `CORE_REFUSED` / `QQ-1022` for the dropped `connection-carries-classification` declaration. The draft's current unit test only exercises the standalone `design_matrix::compute` API and is not acceptance evidence for this CLI reproduction.
