# PLAT-1904 quire-rs dependency move record

## Revisions

- Quoin old dependency: quire-rs `v0.63.1`, commit `bd797be83f643a31e3a9c86580a06b4f6dbfa6bb`.
- Quoin new dependency: package `0.63.2`, exact merged main revision `5f6268a21e1aa4ede52f098f4925616d2750a281`.
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
- `reviews/plat-1904-pr751-code-review.md` (A)
- `reviews/plat-1904-pr751-gap-analysis.md` (A)
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
- `reviews/pr748/plat-1892-code-review.md` (A)
- `reviews/pr748/plat-1892-gap-analysis.md` (A)
- `reviews/pr748/plat-1892-replay-summary.tsv` (A)
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
- `src/obligation/inventory.rs` (M)
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
- `tests/it/coverage_matrix.rs` (M)
- `tests/it/coverage_rollup.rs` (M)
- `tests/it/formalizes_relation.rs` (M)
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
- `coverage::EvidenceCoverageMode` and the `mode` argument added to `compute_with_evidence`
- `obligation::inventory::InventoryModelPolicy` and `InventoryOptions::model_policy`
- `impact` module: `LineRange`, `SourceSnapshot`, `FileChange`, `ImpactInput`, `RevisionSide`, `ChangedSymbol`, `UnmappedChange`, `ImpactReason`, `SelectedTest`, `RelatedImplementation`, `ImpactFinding`, `ImpactReport`, `DependencyMoveInput`, `UpstreamChange`, `DependencyLocalReference`, `DependencyTestReason`, `DependencyMoveReport`, `analyze_dependency_move`, `ImpactError`, `analyze`, `CallerSearch`, `LinkedCaller`, and `UnresolvedCallSite`
- `semantic::relations::RelationSource::validation_path`
- `semantic::surface::SemanticExtraction::record_relations`
- `EmbeddedAnomalyDocument` and `embedded_anomaly_documents`
- `validate_document_in_indexed_bundle`

A search of Quoin Rust source for those names found these references:

- `rust/crates/quoin-quire/src/design_matrix.rs` calls `coverage::compute_coverage_for_design_matrix`.
- `rust/crates/quoin-quire/src/evidence.rs` calls `coverage::compute_with_evidence` with `EvidenceCoverageMode::{Strict, DesignMatrix}` and constructs `InventoryOptions` with `InventoryModelPolicy::{Strict, DesignMatrix}`. The strict `compute()` path remains strict; only matrix evidence opts into the design-matrix policy.
- `rust/crates/quoin-quire/tests/fr029_dropped_declarations.rs` exercises the design-matrix integration.

Quoin does not reference the new anomaly, impact-analysis, embedded-document, or indexed-bundle APIs. The new transitive `ix-trace-rs` lock entry is used by Quire-RS for implementation markers; Quoin's direct source references to `ix-trace-rs` continue to use its workspace main pin.

The added `CallerSearch`, `LinkedCaller`, `UnresolvedCallSite`, `RelationSource::validation_path`, and `SemanticExtraction::record_relations` items also have no Quoin Rust references.

The changed Quire-RS paths and public API names above were searched in Quoin's Rust source, specs, and review records. There are no direct imports by upstream file path. The upstream `plat-1856.json` is under quire-rs's test tree and is not exposed by the `quire-fixtures` API, so the CLI regressions use one shared, minimal Quoin-owned fixture.

## Tests and checks

- `CARGO_HOME=/home/peter/.cache/cargo-home-shared cargo metadata --manifest-path rust/Cargo.toml --locked --offline --format-version 1` passed against `5f6268a`.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` and `git diff --check` passed.
- Through `/home/peter/dev/worktrees/locked-build.sh` against `5f6268a`, the real-host regression `cargo test --locked -p quoin-cli --test tc_1080_matrix_command tc_1080_211_matrix_keeps_valid_rows_when_a_required_relation_drops -- --exact` passed (1 passed, 0 failed), including the strict non-relation-drop assertion.
- Through `/home/peter/dev/worktrees/locked-build.sh` against `5f6268a`, `cargo test --locked -p quoin-cli --test tc_1614_dossier_command architecture_dossier -- --nocapture` passed (2 passed, 0 failed), covering FR-193-AC-10 and FR-193-AC-11 separately.
- The earlier standalone design-matrix regression and strict QQ-1022 refusal tests passed against `d7d418fa`; the real-host checks above exercise the additional evidence-aware engine APIs from `5f6268a`.
- The first compile attempt against `d7d418fa` exposed that the public function is under `quire_rs::coverage`, not the crate root. The integration now uses `quire_rs::coverage::compute_coverage_for_design_matrix`; both focused tests passed after this correction. The earlier queued attempt against `42bbf796` was canceled before it started after an upstream compile defect was reported.
