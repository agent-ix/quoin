---
id: SR-17098
title: "Code review of Quoin PR 1040 (PLAT-1813 and PLAT-1814: dossier safety class, run identity, records-of-processing identity)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; rust/crates/quoin-assurance/src/matrix.rs, rust/crates/quoin-cli/src/{analysis_evidence.rs,matrix.rs}, rust/crates/quoin-dossier/src/{architecture.rs,engine.rs,error.rs,export.rs,gap.rs,records_of_processing.rs,registry.rs,risk_management.rs,rmm.rs,srs.rs,testing.rs,trace_matrix.rs,verification_report.rs}, rust/crates/quoin-quire/src/{records.rs,systems.rs}, and the five changed test files"
review_set: subset
---

# Code review of Quoin PR 1040

## Summary

Tickets: PLAT-1813 (FR-192, FR-193, FR-194, FR-196 safety class and run identity) and PLAT-1814 (FR-191 and FR-207 records-of-processing identity). Scope is `git diff origin/main...HEAD` of branch `plat-1813-code` only. The Rust lane (`rust-review` with the repository `rust-style` skill) is folded into this file.

What is right: the class is shown as recorded and `unassigned` plus one `safety_class_unassigned` gap when absent in the SRS, trace matrix and architecture documents; the architecture part is now keyed by `Part.document` (the part's own document), as FR-193 requires, instead of `PartRecord.owner`; the run selection orders by parsed instant then file name, the same order as `RetainedRun`; the tally counts recorded outcomes of bound entries only; `run_identity_unrecorded` fires only for an empty timestamp or tool and never for an absent source revision. The `supports_check`/`supports_export` hooks are replaced by one `exports_only_with_release` hook, which matches FR-191: an unfiltered `export` without `--release` drops `DOS-ROP`, and an explicit selection is passed through so `quire export` refuses it. Removing `DOSSIER_OPERATION_UNSUPPORTED` is safe: no other crate, spec, doc or test references it, and the error catalogue's `all()` list and its round-trip tests are updated with it. The `#[cfg(test)] render` removed from `engine.rs` had no caller on `origin/main`. The dossier lib-test fixes (`Summary.unusable`, the 4-argument `discharge_report_from_store`) match `origin/main` signatures, so those tests did not compile on main before this PR.

Overlap with work in flight: PR #1039 (PLAT-1854) rewrites `compute_for_matrix` in `quoin-cli/src/analysis_evidence.rs` (return type `Failure`) and the call site in `quoin-cli/src/matrix.rs`. This PR changes both to return a 3-tuple, so whichever merges second has a textual and semantic conflict. The PLAT-1810 worktree (no PR yet) also grows `GapKind::ALL` from 39 to 41 in `quoin-dossier/src/gap.rs`, so that line conflicts and the merged array must be 43. PLAT-1810 also makes the same `rmm.rs` fix and similar `risk_management.rs` `unusable` fixes. The identical `rmm.rs` hunk merges cleanly; `risk_management.rs` may conflict.

Gates run (focused, through `locked-build.sh`): `cargo test -p quoin-dossier -p quoin-quire --lib --test tc_1617_verification_report --test tc_1437_generators` passed at the branch head (127 + 36 lib tests and both integration files). Clippy, `cargo deny` and the full suite were not run (focused-tests rule).

## Verdict

**FAIL: one high finding.** The run-identity payload sends every retained run, with every entry, through the 16 MiB `matrix.build` ceiling. A store with enough history will then have `quoin matrix` and the dossier refused. The other findings are medium or low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `matrix.build` now carries every retained run record with every entry (bound or not, any suite), and the request is refused above `MAX_MATRIX_BUILD_BYTES` (16 MiB). The store is append-only (about 90 bytes per entry, so a 2,000-test suite fills the cap in about 90 recorded runs). Once that happens, `quoin matrix` and the dossier trace-matrix and verification-report documents are refused. Fix: reduce host-side, sending only entries whose symbol a binding of that suite names (or only each suite's newest run that holds a bound entry), and test the bound | rust/crates/quoin-cli/src/matrix.rs:157,167-186; rust/crates/quoin-core/src/ops/matrix.rs:20 |
| FND-002 | medium | Every design-join element becomes a Parts-table row, so a design element that is not a `part` gets a `safety_class_unassigned` gap it can never clear: a `Component`, or a cross-repo `repo/id` that the local systems records never class. FR-193 asks for a class only for a part, and only part documents carry the field (spec-objects-architecture FR-012) | rust/crates/quoin-dossier/src/architecture.rs:188-196,231-239; rust/crates/quoin-dossier/src/architecture.rs:153-160 |
| FND-003 | medium | The SRS and trace matrix read the class from the requirement's frontmatter through a hard-coded key (`SOFTWARE_SAFETY_CLASS_KEY = "software_safety_class"`). They do not read the coverage-matrix `carried` entry that the merged FR-192 and FR-194 name. The deviation is justified on substance: quire-rs `coverage/reconcile.rs` builds `coverage_matrix` only from obligations, so a requirement with zero criteria has no entry. But the spec is not amended, and the constant contradicts FR-160-AC-13 ("No quoin source ... contains ... a carried field name") | rust/crates/quoin-quire/src/records.rs:74-89; rust/crates/quoin-dossier/src/srs.rs:173-178 |
| FND-004 | medium | The source revision is dead data. The host's `run_records` never sets `source_revision`, so every real report reads `not recorded`. The FR-196-AC-10 "verbatim revision" clause is tested only by injecting a JSON member that no production input sets. Yet the same `RetainedRun` the host already holds carries `observation.performed_revision` (the FR-116 CI checkout revision) | rust/crates/quoin-cli/src/matrix.rs:167-186; rust/crates/quoin-assurance/src/matrix.rs:66-69,334-336 |
| FND-005 | low | The `runs` wire shape is written twice: as a `json!` literal in `quoin-cli` and as the `Deserialize`-only `MatrixRun`/`MatrixRunEntry` in `quoin-assurance`. Drift shows up only at run time, through `deny_unknown_fields`, which goes against the repository rule "The type is the schema". Derive `Serialize` on `MatrixRun` and build it from `RetainedRun` | rust/crates/quoin-cli/src/matrix.rs:167-186; rust/crates/quoin-assurance/src/matrix.rs:53-82 |

## Recommendations on the coder's spec-gap notes

- (a) Class read from the requirement frontmatter instead of `carried`: the deviation is correct in output, and the stated reason (no coverage entry for a zero-criteria requirement) is verified in quire-rs. Amend FR-192 and FR-194 **in this PR**. Have them read the requirement's own mapped record key `softwareSafetyClass` (spec-artifacts-iso FR-009 maps it), the same mechanism FR-193 already uses for a part. That also stops calling it a carried read, and FR-160-AC-13 then holds for carried names. Either way a field name lives in quoin source. FR-160-AC-13 should say so explicitly ("a carried field name used to apply floors"), or the reader must take the name from the loaded ISO manifest's `traceability.carried_fields`.
- (b) Source revision: wire `RetainedRun.observation.performed_revision` and amend FR-196 to name the FR-116 observation as its source (shown verbatim, `not recorded` when absent). If the team does not want it, delete `source_revision` from `MatrixRun`/`RunIdentity`, the column text and the AC-10 clause. Do not keep a field that can only ever read `not recorded`.
- (c) `trace_ids` unused: the binding-symbol join is the right one. The bindings file's `symbols` already include those bound through `trace_ids` by `quoin evidence record`, and that is the join the auditor binds on. Amend FR-196 to read `Binding.symbols` and drop `trace_ids` from the read list (see SR-17099).
- (d) Payload ceiling: see FND-001. Bound it in the host (`quoin-cli/src/matrix.rs::run_records`), not by raising the 16 MiB cap.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `Document::record_member` does not read the engine's mapped record: it rebuilds a frontmatter key from the camelCase record key by a naming convention (`softwareSafetyClass` to `software_safety_class`). It matches the ISO mapping today, but a module whose mapping takes the key from a differently spelled source would make every class read silently `unassigned`. This is also a general public helper built on that guess. Read the mapped record, or name the frontmatter key the module maps, and keep it private to the class reader | rust/crates/quoin-quire/src/records.rs:74-105 |

## Dispositions

Round 1 covers fix commits 35e762f6, d530c084, 67069dac and 0b51bb70 on top of 42e56d0c. I checked them by inspection against the diff; I did not re-run the build.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `quoin_assurance::matrix::bounded_runs` sends, per binding, only the newest run holding a bound entry, and in it only the entries a binding of its suite names. The host calls it, and `the_runs_sent_to_the_matrix_are_bounded_and_read_the_same_identity` asserts both the reduction and an unchanged identity |
| FND-002 | fixed | The part-level class is removed: `architecture.rs` and `systems.rs` are back to the merge base, so no design element gets a `safety_class_unassigned` gap |
| FND-003 | fixed | FR-192 is amended to read the requirement's own record key `softwareSafetyClass` and states why the `carried` entry is not used. `SOFTWARE_SAFETY_CLASS_KEY` is gone, so no quoin source names a carried field (FR-160-AC-13 holds) |
| FND-004 | fixed | `MatrixRun::from(&RetainedRun)` takes `observation.performed_revision`, FR-196 and AC-10 are amended to name the FR-116 observation, and `tc_1080_211_the_matrix_carries_the_run_identity_the_store_retains` drives it end to end through `--run-observation` |
| FND-005 | fixed | `MatrixRun` and `MatrixRunEntry` derive `Serialize`, and the host builds them through `From<&RetainedRun>` and `bounded_runs`. The `json!` literal is deleted, leaving one wire shape |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The amended FR-192 says "no quoin source names a carried field", and FR-160-AC-13 says no quoin source contains a carried field name. But `Document::software_safety_class` now reads the literal `software_safety_class`, which is both the frontmatter key and the ISO module's `traceability.carried_fields` name. The behaviour is right; the two spec sentences are false by string identity. Reword FR-192 to say it reads the frontmatter field `software_safety_class`, and scope FR-160-AC-13 to the carried name used to apply class floors | rust/crates/quoin-quire/src/records.rs:74-86; spec/functional/FR-192-dossier-srs-content.md:33 |

## Dispositions (round 2)

Round 2 covers fix commit c7f409d0 after merge 628c76b3 (origin/main #1042), head c7f409d0, checked by inspection. The focused tests were not run by me (the coder's were still queued).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | fixed | `record_member` and its camelCase-to-snake_case guess are deleted. `software_safety_class()` reads the frontmatter field `software_safety_class` directly (null treated as absent), which is the field spec-artifacts-iso FR-009 defines and maps to `softwareSafetyClass` |
