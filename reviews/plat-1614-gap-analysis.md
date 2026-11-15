---
id: SR-20261114002438
title: "Gap analysis of quoin PR 928 (FR-191 dossier command frame)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@e81a79c8; spec/functional/FR-191-dossier-command.md, spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, rust/crates/quoin-dossier/**, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-040"
    type: "references"
---

# Gap analysis of quoin PR 928

## Summary

Ticket: PLAT-1614. Planless. Plan completion: not assessed. Scope is the PR diff against FR-191
and NFR-040.

`quire matrix` / `quoin matrix` could not be run: the CLI needed a permission this session did
not have. The matrix below is a **grep index** of `#[trace(...)]` markers in the changed files,
declared as that fallback. It is not the engine's computed matrix, so it cannot see
`tagged-by-ignored-test` status or stale evidence.

All 17 FR-191 criteria carry at least one tag. NFR-040-AC-1 is tagged. NFR-040-AC-2 is an
Inspection criterion: the inspection found no wall-clock or network use in
`rust/crates/quoin-dossier/src` (no `now(`, `SystemTime`, `Instant::` or HTTP crate; the
dependencies are ix-cli-kit, serde, serde_json, tempfile, thiserror and time). NFR-040-AC-3,
AC-4 and AC-5 depend on the real document content of FR-192..FR-195 (PLAT-1437, PLAT-1438) and
are context only for this PR.

The semantic review was not run: it is opt-in and the brief did not ask for it.

## Verdict

**CONDITIONAL**: three medium findings, and no FR-191 criterion is untagged.
Two of the tags do not back what they claim (FND-001 and FND-003). The shipped build cannot yet
meet the parts of FR-191 that name the four document kinds (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-191-AC-2 ("`generate` with no `--doc` writes the four files at `dossier/<document-id>.md`") is tagged only by `no_selector_selects_every_kind_in_registry_order`. That test calls `Registry::select(&[])` and writes no file. No CLI test runs `generate` without `--doc`, and with the stub registry it would exit 3 on `stub-fail`. The AC is counted as covered without its behaviour being asserted | rust/crates/quoin-dossier/src/registry.rs:192-203 |
| FND-002 | medium | The shipped binary registers no generator, so it breaks FR-191 "Selection and check" and "Outputs" as written. `--doc srs` exits 2 with `DOSSIER_UNKNOWN_KIND` although `srs` is one of the four spec names, and no `--doc` selects nothing and exits 0 instead of writing four documents. Meanwhile `dossier` is listed in root `--help`. This is expected to close as PLAT-1437/PLAT-1438 register generators, but nothing in the spec or the command says the surface is partial | rust/crates/quoin-dossier/src/lib.rs:58-67, rust/crates/quoin-dossier/src/registry.rs:118-145, rust/crates/quoin-cli/src/help.rs:89 |
| FND-003 | medium | `generated_frontmatter_validates_and_an_unknown_document_kind_does_not` is tagged `FR-018-AC-1`, `FR-018-AC-3`, `FR-018-AC-8`. In this repo those ids resolve to quoin's own FR-018 ("Map plugin source arguments to typed sources"), so the computed matrix binds the archetype test to unrelated plugin-source criteria. The spec-artifacts-process FR-018 it means is not a quoin criterion. The test belongs on FR-191 ("Document set") | rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs:740 |

## Coverage

Plan completion: not assessed

Grep-index matrix (declared fallback, not `quire matrix`):

| Criterion | Tagged by |
| --- | --- |
| FR-191-AC-1 | tc_1614 `generate_writes_exactly_the_selected_files_and_nothing_else`; engine `generate_writes_only_the_selected_files`, `an_unrelated_dotfile_with_a_staging_style_name_is_never_touched` |
| FR-191-AC-2 | registry `no_selector_selects_every_kind_in_registry_order` (FND-001) |
| FR-191-AC-3 | tc_1614 `an_unknown_kind_exits_2_names_it_and_writes_nothing`; registry `an_unknown_kind_is_named_with_the_known_ones`; error `codes_map_to_the_exit_table` |
| FR-191-AC-4 | tc_1614 `check_matches_then_differs_after_an_edit_then_differs_when_deleted_and_writes_nothing` |
| FR-191-AC-5 | same tc_1614 test; engine `check_reports_matches_then_differs_...` |
| FR-191-AC-6 | tc_1614 `export_with_a_missing_quire_exits_4_and_names_the_path`; export `a_missing_quire_is_reported_naming_the_path_tried`; error `codes_map_to_the_exit_table` |
| FR-191-AC-7 | tc_1614 `a_quire_refusal_exits_4_...` (recording quire); `real_quire_refuses_an_uncommitted_generated_file` (ignored) |
| FR-191-AC-8 | tc_1614 `export_runs_quire_once_as_a_bundle_...`; export `a_bundle_passes_files_in_registry_order_...`; registry `selection_is_in_registry_order_whatever_the_flag_order` |
| FR-191-AC-9 | tc_1614 `export_runs_quire_once_as_a_bundle_...`; export `one_document_is_exported_without_bundle_arguments` |
| FR-191-AC-10 | tc_1614 `export_looks_where_out_says_...`; export `a_missing_or_stale_file_stops_before_quire_starts` |
| FR-191-AC-11 | tc_1614 `export_of_a_stale_file_exits_1_...`; export `a_missing_or_stale_file_stops_before_quire_starts` |
| FR-191-AC-12 | tc_1614 `a_failing_source_exits_3_...`; engine `a_failing_source_writes_nothing_...`; error `codes_map_to_the_exit_table` |
| FR-191-AC-13 | tc_1614 `generate_check_and_export_without_a_version_exit_2_...`; engine `a_blank_version_is_refused_...` |
| FR-191-AC-14 | tc_1614 `gaps_are_bullets_in_gap_order_...`; writer x2; gap x3; engine `gaps_are_sorted_...` |
| FR-191-AC-15 | tc_1614 `gaps_are_bullets_in_gap_order_...`; order x4; gap `document_level_gaps_sort_before_gaps_naming_an_id` |
| FR-191-AC-16 | tc_1614 `an_incomplete_current_document_is_exported_not_refused` |
| FR-191-AC-17 | tc_1614 `an_out_path_that_cannot_be_created_exits_2_...`; engine `an_out_dir_that_cannot_be_created_...` |
| NFR-040-AC-1 | tc_1614 `two_generate_runs_produce_identical_files_and_json`; engine `gaps_are_sorted_...` |
| NFR-040-AC-2 | Inspection, done in this review: clean |
| NFR-040-AC-3..5 | out of scope (real generators, PLAT-1437/1438) |

Reverse gaps: `RegistryConflict` (exit 4) and the `test-stubs` feature have no owning statement
in FR-191. The exit-4 mismatch is recorded in the code-review artifact (FND-003 there). The stubs
are test-only scaffolding the ticket asks for. No source stubs were found: there are no
`todo!()` or `unimplemented!()` calls, and the empty `builtin_generators()` is the staged rollout
recorded in FND-002.

Semantic review: skipped (not opted in).

## Examined scope

```yaml
clean: false
scope:
  - {id: FR-191-AC-1, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`generate --doc srs --doc rmm` writes exactly two files and no other"}
  - {id: FR-191-AC-2, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`generate` with no `--doc` writes the four files at `dossier/<document-id>.md`"}
  - {id: FR-191-AC-3, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`generate --doc nope` exits 2 and writes no file"}
  - {id: FR-191-AC-4, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "On a fixture whose documents are all complete, `check` on a committed file with one hand-edited line reports that document as `differs` and exits 1; with no edit it reports `matches` and exits 0"}
  - {id: FR-191-AC-5, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`check` writes no file and a deleted file reports `differs`"}
  - {id: FR-191-AC-6, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export --quire <missing path>` exits 4 and the report names that path"}
  - {id: FR-191-AC-7, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export` over a current generated file that is not committed surfaces `quire export`'s refusal with its exit code on stderr and leaves no output file"}
  - {id: FR-191-AC-8, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export` of two or more documents invokes `quire export` once with the committed files in the order `DOS-SRS`, `DOS-ARCH`, `DOS-TRACE`, `DOS-RMM` regardless of `--doc` order, `--source-rev HEAD`, `--bundle-title \"Submission Dossier\"`, `--bundle-id DOS`, and `--approval`, `--template` and `--release` unchanged"}
  - {id: FR-191-AC-9, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export --doc srs` invokes `quire export` once with that one file and with neither `--bundle-title` nor `--bundle-id`"}
  - {id: FR-191-AC-10, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "A dossier generated with `--out other` is exported by `export --out other`, and `export` without `--out` looks under `dossier` and exits 1 naming the missing document"}
  - {id: FR-191-AC-11, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export` over a committed file that differs from the regenerated bytes exits 1, names the document and never starts `quire export`"}
  - {id: FR-191-AC-12, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "A source that fails makes `generate` exit 3, the report names that source in `source_failure`, no file is written for any selected document, and a file that existed before the run is unchanged"}
  - {id: FR-191-AC-13, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`generate`, `check` and `export` without `--version` exit 2 and write nothing"}
  - {id: FR-191-AC-14, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "A fixture with two gaps of different kinds gives a `Gaps` section with two bullets in gap order, the same two entries in the same order in `--json`, and `Gaps: none` for a complete document"}
  - {id: FR-191-AC-15, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "A fixture with requirements `FR-99` and `FR-100` and criteria `FR-100-AC-2` and `FR-100-AC-10` lists them in dossier order, and a document-level gap sorts before a gap that names a requirement"}
  - {id: FR-191-AC-16, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`export` of an `incomplete` document that is committed and current runs `quire export` and does not refuse it"}
  - {id: FR-191-AC-17, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "`generate --out <path that cannot be created>` exits 2 and writes nothing"}
  - {id: NFR-040-AC-1, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: examined, excerpt: "Two runs over the same commit give byte-identical files"}
  - {id: NFR-040-AC-2, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: examined, excerpt: "Inspection of the dossier code path finds no wall-clock call, and analysis of its dependency graph finds no network crate"}
  - {id: NFR-040-AC-3, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: context_only, excerpt: "For a fixture whose records state no criticality, risk, compliance or acceptability, none of those values appears in any generated file"}
  - {id: NFR-040-AC-4, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: context_only, excerpt: "Fixture documents created in a different order on disk give the same bytes, and swapping two `--clause-binding` options changes only the order of the two RMM tables"}
  - {id: NFR-040-AC-5, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: context_only, excerpt: "A `generate` run that selects only non-RMM documents (`srs`, `architecture`, `trace-matrix`) with no `--as-of` writes no time into any file"}
  - {id: FR-018-AC-1, path: spec/functional/FR-018-map-plugin-sources.md, role: context_only, excerpt: "`path:<dir>` maps to a path source"}
```

## Dispositions

Round 1, verified against fix commit 481d46a9 (`git diff e81a79c8..481d46a9`). No builds or tests were run, by instruction.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The four-file assertion lands with PLAT-1437/PLAT-1438. This round replaced the select-only tag with `engine::no_selector_writes_a_file_for_every_registered_kind` (engine.rs:374-394). That test runs `generate` with no `--doc`, then asserts a `dossier/<id>.md` file per registered kind and reads each one back. The "four files" half cannot be asserted until four generators exist, and the test's own doc comment says so. |
| FND-002 | deferred | PLAT-1437 ("Generate SRS and architecture documents") and PLAT-1438 ("Generate trace matrix and RMM documents") register the four generators. Both are open in Backlog. The partial surface is no longer silent. With an empty registry, every `--doc` value and no `--doc` exit 2 with `DOSSIER_NO_DOCUMENTS`, and the message names those tickets (registry.rs:119-125). The command writes no directory and never starts quire. |
| FND-003 | fixed | - |

- FND-003 after_excerpt (`rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs:772-780`): `/// defines the DossierDocument archetype (that repository's FR-018, not a /// quoin requirement), ... #[trace("FR-191")]`. The `FR-018-AC-1/3/8` tags are gone.
