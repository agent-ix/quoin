---
id: SR-17099
title: "Gap analysis of Quoin PR 1040 (PLAT-1813 and PLAT-1814)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; FR-191 (AC-13, AC-15, AC-24, AC-29, AC-30), FR-192 (AC-7..9), FR-193 (AC-7, AC-8), FR-194 (AC-7, AC-8), FR-196 (AC-3, AC-9..12), FR-207-AC-10, against the PR 1040 diff"
review_set: subset
---

# Gap analysis of Quoin PR 1040

## Summary

Tickets: PLAT-1813 (safety class and run identity: FR-192, FR-193, FR-194, FR-196) and PLAT-1814 (records-of-processing identity: FR-191, FR-207). Planless run scoped to the PR diff.

Matrix verification: `quire matrix --scope . --format json` reports every in-scope criterion `tagged`: FR-191-AC-13/15/24/29/30, FR-192-AC-7/8/9, FR-193-AC-7/8, FR-194-AC-7/8, FR-196-AC-3/9/10/11/12 and FR-207-AC-10. Reverse gap: no production change in the diff lacks an owning requirement. No stubs.

Mutation check: the reviewer applied each mutation, ran the focused tests through `locked-build.sh`, and reverted the tree clean.

| Mutation | Killed by | Result |
| --- | --- | --- |
| Drop the `exports_only_with_release` filter in `engine.rs` | `export::tests::an_unfiltered_export_leaves_a_release_only_document_out_unless_release_is_given` | killed |
| Re-key the architecture part from `part.document` to `part.owner` | three architecture tests (FR-193-AC-7, AC-8, no_allocation) | killed |
| SRS/trace class falls back to criticality (shared `software_safety_class_of`) | `srs::tests::an_unrecorded_class_is_unassigned_and_never_copied_from_criticality_or_a_part` | killed |
| Trace matrix alone infers the class from criticality | none | **survived** |
| Architecture part inherits its parent (owner) part's class | none | **survived** |
| Drop the `run_identity_unrecorded` gate | `an_empty_tool_or_timestamp_is_one_gap_and_a_recorded_revision_is_verbatim` | killed |
| Compute a run outcome (append `run outcome failed/passed`) beside the tally | none | **survived** |
| Write a `version` into every document when `--version` is given (DOS-ROP included) | the two release-only export tests over the generic `Doc` fixture | killed (no records-of-processing test catches it) |
| `RecordsOfProcessingGenerator::exports_only_with_release` returns `false` (DOS-ROP in unfiltered export without `--release`) | only `records_of_processing_takes_its_identity_from_release_alone`, which asserts the hook's constant | killed by a constant-assertion only |
| Host stops sending `runs` to `matrix.build` (`quoin-cli/src/matrix.rs`) | none of `tc_1080_matrix_command` (13) or `tc_1438_dossier_matrices` (7) | **survived** |

The coder's own run-selection mutation (`max_by` to `min_by`) was not repeated.

## Verdict

**CONDITIONAL: no untagged criterion and no high finding.** Four behaviour mutations survive (computed outcome, parent-class inheritance, trace-matrix criticality inference, host not sending runs), and four spec texts need amending. See also SR-17098, which carries the high payload-ceiling finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-196 "SHALL NOT compute ... a run outcome" has no test that fails when an outcome is computed. The AC-9/AC-12 tests assert with `contains`, so a computed `run outcome failed` appended to the cell survives. Assert the whole run-identity cell exactly | rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs:255-273,321-337 |
| FND-002 | medium | FR-193-AC-8 ("even when its parent part ... record[s] a class") is not exercised: `part_row` hard-codes owner `OWNER`, which is no classed part, so inheriting the parent's class survives. The test comment claims "The owner cell names a classed parent", which the fixture does not do | rust/crates/quoin-dossier/src/architecture.rs:516-525,726-756 |
| FND-003 | low | No trace-matrix test has a requirement with criticality and no class (FR-10 has neither), so a trace-matrix-only fallback to criticality survives (FR-194 "SHALL NOT infer") | rust/crates/quoin-dossier/src/trace_matrix.rs:365-390 |
| FND-004 | medium | The FR-196-AC-3 clause "shows that typed cause and the tally of recorded outcome values" has no assertion. None of the four AC-3-tagged tests gives the failed-run criterion a selected run, so the tally on a failed-run row is never checked | rust/crates/quoin-dossier/src/verification_report.rs:562-620; rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs:134-148 |
| FND-005 | low | The FR-196-AC-11 clause "a criterion backed only by retained Inspection evidence gets neither run gap nor a run value" is not asserted. The AC-11 test covers only the no-binding and no-run cases | rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs:296-319 |
| FND-006 | medium | Mutation survived: deleting the host's `request.insert("runs", ...)` passes `tc_1080_matrix_command` and `tc_1438_dossier_matrices`. No CLI test shows that real retained runs reach `matrix.build` (`request.insert("runs", ...)`). Every run-identity test injects `runs` or `RunIdentity` directly. Add one `quoin matrix --json` test over a recorded store asserting `evidence_detail.bindings[].run` | rust/crates/quoin-cli/src/matrix.rs:157,167-186 |
| FND-007 | low | The records-of-processing release-only behaviour (FR-191-AC-30, FR-207-AC-10 "unfiltered export without --release omits DOS-ROP") is pinned for the real generator only by a test that asserts the hook's constant. The behavioural tests use the `Doc` test double, and no test exports unfiltered over the builtin registry with and without `--release` | rust/crates/quoin-dossier/src/records_of_processing.rs:1329-1336; rust/crates/quoin-dossier/src/export.rs:679-736 |
| FND-008 | low | FR-196's read list names `trace_ids`, but the selection correctly joins on `Binding.symbols`, the auditor's bound join (`quoin evidence record` already folds `trace_ids` into binding symbols). Amend FR-196 to name `Binding.symbols` and drop `trace_ids` | spec/functional/FR-196-dossier-verification-report-content.md:30 |
| FND-009 | low | `run_identity_unrecorded` cannot fire on a store-read record: `validate_envelope` refuses an empty `tool` or a non-canonical `timestamp` before a run is retained. The gap only fires on host-injected data. State this in FR-196 or drop the gap kind (value test) | rust/crates/quoin-evidence/src/store/records.rs:312-333; spec/functional/FR-196-dossier-verification-report-content.md:59 |
| FND-010 | low | FR-193 has the architecture document read the part class, while spec-objects-architecture FR-012 says "The part-level class is used by the field_consistency check below and by nothing else: no per-item report reads it". The two specs conflict, and one must be amended | spec/functional/FR-193-dossier-architecture-content.md:34,43-45 |
| FND-011 | low | FR-207-AC-10 asks for `check` with no `--doc` and no `--version` to report DOS-ROP. With the builtin registry, an unfiltered `check` also selects versioned kinds and exits 2 without `--version`. The test uses explicit `--doc` (and `--version 1.0` for the unfiltered case), so the AC text cannot hold as written | spec/functional/FR-207-dossier-records-of-processing-content.md:93; rust/crates/quoin-dossier/src/records_of_processing.rs:1278-1327 |

## Coverage

- Criteria examined: 18, all `tagged` in the computed matrix; 0 untagged.
- Reverse gap: none; stubs: none.
- Semantic review: skipped (not requested); mutation check done instead (table above).
- Plan completion: not assessed
