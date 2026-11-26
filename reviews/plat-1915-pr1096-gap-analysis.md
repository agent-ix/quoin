---
id: SR-967175026302197768900474181941935084890
title: "gap-analysis of quoin PR 1096"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1915; PR 1096; rust/crates/quoin-catalog; rust/crates/quoin-semantic; rust/crates/quoin-modules; rust/crates/quoin-cli/tests/tc_catalog_command.rs; FR-099"
review_set: subset
---

## Summary

Reviewed PR #1096 at 4938e7a97f9ff20f07efa655dab515c509d0b3fb. The root-key refusal and same-kind duplicate changes work on their tested paths, but cross-kind duplicate detection and partial installed roots still violate the claimed criteria.

## Verdict

FAIL for repository assurance — existing untagged criteria remain, and the AC-11/AC-12 bindings do not detect the reproduced failures recorded in the code review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional findings (placeholder) | - |

## Assurance Context

Inspected AP-201 at spec/assurance/AP-201-finding-quality.md and AP-202 at spec/assurance/AP-202-jev-lens-evaluation.md. The former warns against hiding gaps behind a green aggregate; the latter's live Jev scope is unaffected. This review records the concrete failures rather than treating tagged rows as proof. Unavailable selected context: none. Active exceptions: none.

## Coverage

Plan completion: not assessed

Matrix and code inspection distinguish static tags from the actual assertions. FR-099 AC-1/6/7/8/9/10 and CON-1 remain open as the PR states; AC-2 is already tagged and CON-2 is Inspection. AC-11 and AC-12 currently have tagged tests, but the reproduced failures show they are not fully discharged. No duplicate method findings are posted here; the two defects are owned by the separate code-review artifact.

Reverse/stub procedure: inventoried 951 Rust files and scanned the entire Rust tree for todo!/unimplemented! (zero hits). Inspected catalog build/resolve/duplicate behavior, SemanticValidators loading and read_module_semantic/read_semantic_block callers, contract-gate refusal, Gix extract and ModuleInstaller/reconcile/catalog consumption. These are owned by FR-099/FR-019/FR-070; no new unowned behavior was found in this flow. Tests use real schema bytes and filesystem/cache boundaries; the counting resolver is a narrow port seam, not a replacement for reconcile. Full manual reverse ownership adjudication outside the changed area is incomplete. Optional semantic audit was not requested and did not run; no repository-wide PASS is claimed.

## Examined units and bindings

```yaml
scope:
- id: FR-099-CON-1
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: Quoin SHALL NOT create a second catalog, registry or module resolver.
- id: FR-099-CON-2
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: '`quoin-quire` SHALL NOT re-implement Quire parsing, extraction or validation
    semantics.'
- id: FR-099-CON-3
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: Reconcile SHALL NOT perform network access when the declared modules are
    already materialized.
- id: FR-099-AC-1
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: '`quoin-core validators.run` returns the same verdict and non-success classification
    as the retained validator path over the golden corpus.'
- id: FR-099-AC-2
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: '`quoin-core quire.validate` returns the same verdict as the retained `src/quire`
    adapter for every corpus document, with `quire-rs` consumed as a Cargo dependency.'
- id: FR-099-AC-3
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: '`quoin-core catalog.list` assembles a catalog identical to the retained
    catalog for the same configuration root, resolves a type case-insensitively, and
    reports a planted duplicate type rather than selecting one.'
- id: FR-099-AC-4
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: A second reconcile over an already-materialized configuration root performs
    no git or network access and yields a catalog identical to the first.
- id: FR-099-AC-5
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: A manifest carrying an unknown key is refused by the Rust reader with the
    same classification as the retained reader.
- id: FR-099-AC-6
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: '`quoin-core semantic.validate` and `quoin-core completeness.analyze` return
    verdicts identical to the retained implementations over the golden corpus.'
- id: FR-099-AC-7
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: The Rust skill tests read the shipped Markdown skills and fail on a planted
    vocabulary drift, and the binary resolves the skills root from an argument, an
    environment value and its embedded fallback.
- id: FR-099-AC-8
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: '`package.json` declares no dependency on `@agent-ix/ix-cli-core`, and
    each imported symbol recorded in the baseline inventory has a named Rust implementation
    and a test; the gate reads the inventory rather than a hard-coded count.'
- id: FR-099-AC-9
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: Every asset under `skills/**/workflow-assets/**` carries a recorded disposition
    that distinguishes the first-party invariant shim from the vendored bundle, and
    an unclassified asset fails the gate.
- id: FR-099-AC-10
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: context_only
  excerpt: 'The ix-flow child process is spawned through one hardened path: real-path
    resolution, a resolution diagnostic when the executable cannot be resolved, a
    declared output ceiling, a timeout, and exit, signal and spawn-failure reported
    as three distinct outcomes.'
- id: FR-099-AC-11
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: A partially materialized module root is refused rather than served, so
    an interrupted checkout cannot satisfy the already-materialized path.
- id: FR-099-AC-12
  path: spec/functional/FR-099-rust-catalog-and-validation-capability.md
  role: examined
  excerpt: Duplicate type detection folds case under the same rule the resolver uses,
    so no two declarations are distinct to the duplicate gate and identical to the
    resolver.
bindings:
- test_id: tc_373_catalog_deduplicates_roots_names_and_reports_duplicate_types
  ac_id: FR-099-AC-3
  trace: correct
- test_id: tc_1650_catalog_list_replays_the_retained_fixture_in_json_and_human_forms
  ac_id: FR-099-AC-3
  trace: correct
- test_id: tc_521_catalog_list_discovers_a_nested_module_through_the_shipped_binary
  ac_id: FR-099-AC-3
  trace: correct
- test_id: tc_381_283_a_settled_lazy_reconcile_performs_no_network_io
  ac_id: FR-099-AC-4
  trace: correct
- test_id: tc_381_283_a_settled_lazy_reconcile_performs_no_network_io
  ac_id: FR-099-CON-3
  trace: correct
- test_id: tc_1915_unknown_module_manifest_root_key_is_refused_by_the_reader
  ac_id: FR-099-AC-5
  trace: correct
- test_id: tc_1915_partial_extraction_cache_is_rebuilt_before_use
  ac_id: FR-099-AC-11
  trace: correct
- test_id: tc_1915_duplicate_types_use_the_resolvers_case_fold
  ac_id: FR-099-AC-12
  trace: correct
```

## Checks

Independent execution of the coder-identified current-head compiled artifacts through locked-build.sh passed: manifest suite 15/15, catalog library 7/7, Gix partial-cache filter 1/1, settled-reconcile filter 1/1 and CLI catalog suite 5/5, zero failed/ignored selections. The rerun reused the compiled artifacts rather than rebuilding; coder compile output is retained in /tmp/plat1915-quoin-batch3-fr099-focused-final.log. The CLI suite took 68.45 seconds in the reviewer rerun. Independent cargo fmt --all --check and git diff --check passed. Broad workspace gates were not run under the owner's focused-only rule.

Fresh matrix used Quoin 0.37.0 (npmjs-cached executable), and Quire is 0.52.0, engine 0.62.2. Deduplicating criterion ids and filtering method exactly Test yields 1,712 tagged, 483 untagged and 11 ignored-only rows repo-wide. The supplied older first-added-file-date cohort is 361 to 355, excluding withdrawn FR-089/090; the entire date-cohort denominator was not independently reconstructed. The six new static statuses do not establish full AC-11/12 discharge because the two reproduced product failures remain. Registry trace queries resolve implements for read_semantic_block (AC-5) and find_duplicates (AC-12); six unrelated cfg-twin symbol collision warnings remain in campaign, selftest, completeness, config lock/service and bounded-read. Both requested file queries exited 0.

The PR title Close FR-099 catalog and manifest test gaps and branch codex/quoin-fr099-test-gaps omit PLAT-1915. The body contains Refs PLAT-1915, not a closing directive. The explicitly supplied ticket was used for this campaign batch rather than inferred from title/branch; this avoids the known ticket auto-close behavior. No dependency revision moved: ix-trace-rs stays on its existing workspace pin while becoming a normal dependency in the touched crates; quoin-catalog is added only as an internal test dependency.

The manifest allowlist is derived from properties of the embedded @agent-ix/semantic-schema@0.5.0 module-manifest schema (additionalProperties: false), not a second hand-maintained list. The existing no-semantic-block controls remain passing. AP-201 and AP-202 were read for context; test and catalog behavior are assessed against the actual requirements, with no release or verification claim inferred from a green static matrix.

## Reproductions

FND-001: two temporary module roots declare artifact CollisionT and object collisiont. Set QUOIN_MODULE_PATHS to both roots with the platform separator and use isolated IX_HOME. The shipped catalog list reports both entries with an empty duplicate list; catalog validate exits 0 with ok: true; catalog show resolves both spellings to module one. Raw result: /tmp/review1096-cross-kind.json.

FND-002: under isolated IX_HOME create a registry for the ten shipped default git-subdir declarations, create each target directory with only a name-only manifest.yaml, omit all schemas and completion markers, then invoke catalog list --json. Observed exit 0, ten modules, zero entries, empty stderr. Raw result: /tmp/review1096-partial-install.json.

## Disposition round 1

Rechecked 378539a2803c0c75594ce4b5e17cd02c97abe971. The repository assurance verdict remains FAIL. AC-11 still does not reject the original partial-with-manifest installed target, and AC-12 still misses same-module colliding declarations. See FND-002 and new FND-003 in the code-review artifact. PR mergeable: no.

Round-1 verification: reran the original CLI probes against the current compiled binary. Cross-module CollisionT/collisiont now yields a duplicate group and catalog validate exits 1 with ok: false, resolving FND-001. Original registered git-subdir name-only partial targets still yield catalog list exit 0, ten modules and zero entries, resolving neither completeness nor FND-002. A same-module artifact/object case yields two entries, an empty duplicate list and catalog validate exit 0 with ok: true (new FND-003). Raw results are /tmp/review1096-round1-reproductions.json; validation probes also emit non-fatal lagging-pin warnings because the synthetic registry omits recorded refs, which does not account for the wrong duplicate decision.

Independently reran current compiled artifacts through locked-build.sh: catalog library 7/7, missing-manifest reconcile regression 1/1 and settled non-empty-catalog/no-network reconcile regression 1/1; all passed. cargo fmt --all --check and git diff --check passed. Fresh matrix and reconcile trace queries exit 0; reconcile is marked implements FR-099-AC-11. Six existing cfg-twin symbol warnings are retained separately in /tmp/review1096-round1-trace-warnings.txt. No new broad test gate was added; manifest behavior is unchanged since the previous passing 15-test run. The coder's updated full focused log is /tmp/plat1915-quoin-batch3-fr099-focused-review-final.log.

The CLI fixture helper now inserts valid minimal manifests for rendering tests. The former frozen empty-catalog case appropriately drops its FR-099-AC-3 tag because its oracle was changed. It no longer provides the previous frozen empty-catalog parity evidence; the non-empty CLI projection and catalog behavior cases retain AC-3 links. Title/branch still omit PLAT-1915 and the refreshed body retains Refs PLAT-1915. No external version/revision moves occurred; ix-trace-rs moves from dev to normal dependencies at the same workspace pin for the implementation marker.

## Disposition round 2

Reviewed c0b75e15e086af52eb20f5d1b0268c81816f3abc. FND-002/003 and the held intermediate FND-004 are resolved. The scoped PR is mergeable; the repository assurance verdict remains FAIL from existing gaps.

Round-2 review at c0b75e15e086af52eb20f5d1b0268c81816f3abc includes the full diff from both intermediate fixes. Inspected materialize staging, marker write, target publish, prior-root restore, and copy_dir's symlink exclusion; marker creation follows successful copying and precedes target publication. Failed copies cannot publish a target marker. No dependency pin or workflow changes occurred. Implementation markers cover read_semantic_block, find_duplicates, reconcile and materialize at their actual criteria. The new failure-path test carries AC-11.

Independent locked-build reruns of current compiled artifacts passed catalog library 7/7, GitSubdir partial-with-manifest repair 1/1, nested-path settled reconcile 1/1, and failed-materialization marker 1/1; zero failures. Recompiled and ran the original standalone public-API nested-path reproduction against the current library: first_installed=1, second_unchanged=1, second_updated=0, resolver_calls=1. cargo fmt --all --check and git diff --check passed. The coder's complete focused 15/1/7/5/1/1 log is /tmp/plat1915-quoin-batch3-fr099-focused-review4-final.log. This review did not broaden the full workspace gates.

Fresh Quoin 0.37.0 matrix (unique criterion IDs, exact Test) remains 1,712 tagged, 483 untagged and 11 ignored-only repo-wide. No repository assurance PASS is claimed. Quire 0.52.0/engine 0.62.2 trace resolves materialize's AC-11 implements claim and the failed-copy test's AC-11 verifies claim; the six pre-existing cfg-twin warnings remain unrelated. All four findings now have a fixed current outcome, including previously fixed FND-001.
