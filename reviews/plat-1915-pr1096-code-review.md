---
id: SR-967175026302197768900474181941935084889
title: "code-review of quoin PR 1096"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1915; PR 1096; rust/crates/quoin-catalog; rust/crates/quoin-semantic; rust/crates/quoin-modules; rust/crates/quoin-cli/tests/tc_catalog_command.rs; FR-099"
review_set: subset
---

## Summary

Reviewed PR #1096 at 4938e7a97f9ff20f07efa655dab515c509d0b3fb. The root-key refusal and same-kind duplicate changes work on their tested paths, but cross-kind duplicate detection and partial installed roots still violate the claimed criteria.

## Verdict

CONDITIONAL — two reproduced medium findings remain; PR is not mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Duplicate grouping still requires equal EntryKind, while find_entry ignores kind. An artifact CollisionT in module one and an object collisiont in module two are distinct to the duplicate gate but identical to the resolver. Reproduced: catalog list has both entries and duplicates: []; catalog validate returns exit 0 with ok: true; both catalog show spellings select module one. Align the duplicate identity with resolver identity and add the cross-kind regression. | rust/crates/quoin-catalog/src/lib.rs:356 |
| FND-002 | medium | Lazy reconcile treats a registered target as materialized based only on is_dir and a non-empty target_path. A registered git-subdir target containing a name-only partial manifest is therefore skipped and served as a successful empty catalog. Reproduced with the ten default git-subdir entries: catalog list exits 0, reports ten modules and zero entries, with all schemas and completion markers absent. The new AC-11 test calls GixResolver directly and cannot catch this already-materialized shortcut. Validate materialization completeness before taking that shortcut and test the registered partial target through reconcile/catalog, preserving the no-network path for complete targets. | rust/crates/quoin-modules/src/reconcile.rs:70 |

## Assurance Context

Inspected AP-201 at spec/assurance/AP-201-finding-quality.md and AP-202 at spec/assurance/AP-202-jev-lens-evaluation.md. The former warns against hiding gaps behind a green aggregate; the latter's live Jev scope is unaffected. This review records the concrete failures rather than treating tagged rows as proof. Unavailable selected context: none. Active exceptions: none.

## Coverage

Reviewed all ten changed files, the schema-key loading/refusal path and callers, the catalog builder and case-insensitive resolver, the Gix extraction marker and installer/reconcile shortcut, and the relevant CLI/boundary paths. The AC-4/CON-3 test compares a non-empty installed catalog and verifies the resolver is not called a second time. The AC-5 regression asserts module refusal plus semantic.unknown-key error classification at the planted root key. Existing CLI catalog cases cover the frozen empty retained fixture and non-empty nested-module projection; no general golden-corpus parity claim is made here.

The new tests and changed functions carry relevant criterion markers with bare imports. The AC-11 cache test is a meaningful cache-repair check but does not establish the installed-target shortcut's completeness. The AC-12 test covers same-kind case-folding but misses the cross-kind collision. bindings.trace records relevant links, not full behavioral discharge. No new cross-boundary interface types or workflow edits occur.

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

## Dispositions

Round 1 at 378539a2803c0c75594ce4b5e17cd02c97abe971. FND-001 is fixed; FND-002 remains open and FND-003 is new. PR mergeable: no.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The cross-module grouping identity no longer includes EntryKind; CLI reproduction now reports and rejects the duplicate. |
| FND-002 | still-open | The predicate now requires manifest.yaml to exist, but the original failure used a registered git-subdir target with a name-only manifest and missing schemas/completion markers. That exact reproduction still returns exit 0, ten modules and zero entries. The new test removes the manifest, so it does not test or repair the reported partial-with-manifest case. |

## New findings — round 1

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The remaining duplicate gate counts distinct module names rather than declarations: repeated module names are suppressed and a group is reported only when modules.len() > 1. A single module declaring artifact CollisionT and object collisiont therefore exposes two entries identical to the resolver while reporting no duplicate. Reproduced at this head: catalog list includes both entries with duplicates: []; catalog validate exits 0 with ok: true. Detect multiple colliding declarations even within one module and add that regression. | rust/crates/quoin-catalog/src/lib.rs:365 |

Round-1 verification: reran the original CLI probes against the current compiled binary. Cross-module CollisionT/collisiont now yields a duplicate group and catalog validate exits 1 with ok: false, resolving FND-001. Original registered git-subdir name-only partial targets still yield catalog list exit 0, ten modules and zero entries, resolving neither completeness nor FND-002. A same-module artifact/object case yields two entries, an empty duplicate list and catalog validate exit 0 with ok: true (new FND-003). Raw results are /tmp/review1096-round1-reproductions.json; validation probes also emit non-fatal lagging-pin warnings because the synthetic registry omits recorded refs, which does not account for the wrong duplicate decision.

Independently reran current compiled artifacts through locked-build.sh: catalog library 7/7, missing-manifest reconcile regression 1/1 and settled non-empty-catalog/no-network reconcile regression 1/1; all passed. cargo fmt --all --check and git diff --check passed. Fresh matrix and reconcile trace queries exit 0; reconcile is marked implements FR-099-AC-11. Six existing cfg-twin symbol warnings are retained separately in /tmp/review1096-round1-trace-warnings.txt. No new broad test gate was added; manifest behavior is unchanged since the previous passing 15-test run. The coder's updated full focused log is /tmp/plat1915-quoin-batch3-fr099-focused-review-final.log.

The CLI fixture helper now inserts valid minimal manifests for rendering tests. The former frozen empty-catalog case appropriately drops its FR-099-AC-3 tag because its oracle was changed. It no longer provides the previous frozen empty-catalog parity evidence; the non-empty CLI projection and catalog behavior cases retain AC-3 links. Title/branch still omit PLAT-1915 and the refreshed body retains Refs PLAT-1915. No external version/revision moves occurred; ix-trace-rs moves from dev to normal dependencies at the same workspace pin for the implementation marker.

## New findings — held intermediate fix check

FND-004 was observed at 9d25da7cc96fa9d0fb9ca2ddca614cfbda12b777. The lead requested holding the next disposition until its fix landed; this is the first durable artifact/comment record of that observation.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Observed at intermediate head 9d25da7c: requiring the fetched-tree extraction marker at the installed root makes MarketplaceEntry.path-selected Git subroots look incomplete forever. The marker stays at the fetched tree root while only the selected nested directory is copied. A complete nested module is therefore re-resolved and updated on every lazy reconcile. The public-API reproduction returned first_installed=1, second_unchanged=0, second_updated=1, resolver_calls=2. This finding was handed to the lead/coder and publication held at the lead request pending a fix; the final outcome is recorded below. | rust/crates/quoin-modules/src/reconcile.rs:74 |

## Disposition round 2

Reviewed c0b75e15e086af52eb20f5d1b0268c81816f3abc. All findings are resolved; no new unresolved findings. PR mergeable: yes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | Reconcile now requires a distinct installed materialization marker for Git sources. The GitSubdir regression removes that marker while retaining the name-only manifest and proves re-resolution/replacement. |
| FND-003 | fixed | The group counts declarations independently of the deduplicated module-name display list. The same-module artifact/object CollisionT/collisiont regression now reports a duplicate. |
| FND-004 | fixed | The installer writes .quoin-materialize-complete only after copying the selected root successfully and before publishing it; Gix extraction markers remain internal to the fetch root. The original nested-path reproduction now returns unchanged=1, updated=0 and resolver_calls=1. |

Round-2 review at c0b75e15e086af52eb20f5d1b0268c81816f3abc includes the full diff from both intermediate fixes. Inspected materialize staging, marker write, target publish, prior-root restore, and copy_dir's symlink exclusion; marker creation follows successful copying and precedes target publication. Failed copies cannot publish a target marker. No dependency pin or workflow changes occurred. Implementation markers cover read_semantic_block, find_duplicates, reconcile and materialize at their actual criteria. The new failure-path test carries AC-11.

Independent locked-build reruns of current compiled artifacts passed catalog library 7/7, GitSubdir partial-with-manifest repair 1/1, nested-path settled reconcile 1/1, and failed-materialization marker 1/1; zero failures. Recompiled and ran the original standalone public-API nested-path reproduction against the current library: first_installed=1, second_unchanged=1, second_updated=0, resolver_calls=1. cargo fmt --all --check and git diff --check passed. The coder's complete focused 15/1/7/5/1/1 log is /tmp/plat1915-quoin-batch3-fr099-focused-review4-final.log. This review did not broaden the full workspace gates.

Fresh Quoin 0.37.0 matrix (unique criterion IDs, exact Test) remains 1,712 tagged, 483 untagged and 11 ignored-only repo-wide. No repository assurance PASS is claimed. Quire 0.52.0/engine 0.62.2 trace resolves materialize's AC-11 implements claim and the failed-copy test's AC-11 verifies claim; the six pre-existing cfg-twin warnings remain unrelated. All four findings now have a fixed current outcome, including previously fixed FND-001.
