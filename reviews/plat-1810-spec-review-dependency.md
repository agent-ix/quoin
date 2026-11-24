---
id: SR-20261124001344
title: "dependency review of the dossier risk content, safety class, run identity and records-of-processing check/export spec change"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, spec/functional/FR-192-dossier-srs-content.md, spec/functional/FR-193-dossier-architecture-content.md, spec/functional/FR-194-dossier-trace-matrix-content.md, spec/functional/FR-196-dossier-verification-report-content.md, spec/functional/FR-199-dossier-risk-management-content.md, spec/functional/FR-207-dossier-records-of-processing-content.md (PR #1038)"
review_set: subset
---

# dependency review of PR #1038

## Summary

Tickets: PLAT-1810, PLAT-1813, PLAT-1814. This review checked every claim the change
makes about an upstream source or a sibling spec against `origin/main` of quoin and of
the repositories it reads from (quire-cli, quire-rs, spec-objects-safety,
spec-objects-architecture, spec-artifacts-iso). Claims that hold: the records-of-processing
generator returns `false` from `supports_check()` and `supports_export()`
(`records_of_processing.rs:50-56`) and registers right after the threat-model
generator (`lib.rs` builtin order), so `DOS-ROP` after `DOS-TM` matches the
registry; `MatrixCriterion` and `EvidenceDetail` have the fields FR-196 names; the
`quoin-risk-advice` `ResidualEntry` has only `residual_risk`, `decision`,
`decided_by`, and `Row` carries bare ids for hazard, situation and harms; the
spec-objects-architecture `part` declares `software_safety_class` (FR-012, manifest);
quire-rs FR-090 carries `software_safety_class` on coverage-matrix requirement entries
with `carried_vocabulary`. Three claims fail against the upstream contracts and are
high findings.

## Findings

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-001 | high | `quire export` refuses a document that has neither a `version` member nor `--release` (quire-cli FR-034 Identity, FR-040-AC-1, exit 3), and gives every bundle input that same identity (quire-cli FR-039 section A). DOS-ROP carries no version by design. So `export --doc records-of-processing` without `--release` fails with exit 4, contrary to FR-191-AC-13, FR-191-AC-24 and FR-207-AC-10; and because unfiltered `export` now includes DOS-ROP whenever it is registered, every unfiltered bundle export without `--release` fails, which breaks today's working export. The spec must decide how DOS-ROP gets an export identity (for example: require `--release` when DOS-ROP is selected for export, and say so in FR-191 Inputs and the ACs, or carry `--version` into DOS-ROP for export), or leave DOS-ROP out of unfiltered export. | spec/functional/FR-191-dossier-command.md:81; FR-191-AC-13; FR-191-AC-24; FR-207-AC-10; quire-cli FR-034, FR-039, FR-040-AC-1 | wrong-requirement |
| FND-002 | high | The FR-199 source contract adds hazard, situation and harm texts, residual-risk rationale and the full `benefit_risk` and `overall_residual_risk_evaluation` lists to the `risk-summary` source, and FR-199 Inputs say that source is `quoin risk-summary <scope> --format json`. FR-169 (untouched) fixes that JSON to exactly `attribution` and `rows`, every row and residual-risk entry to closed key sets (FR-169-AC-1, FR-169-AC-2), and does not read `overall_residual_risk_evaluation` at all. The contract cannot be built without breaking FR-169. Either amend FR-169 in this PR (new keys, new record kind, ACs) or state that the dossier reads a library-level structure that is not the CLI JSON and name it. | spec/functional/FR-199-dossier-risk-management-content.md:39; FR-169-AC-1; FR-169-AC-2; rust/crates/quoin-risk-advice/src/summary.rs:157-185 | wrong-requirement |
| FND-003 | high | FR-196 asks for a run outcome "as the run file records it" and a source commit "as the run file records it". The retained run record (`quoin-evidence` `RunRecord`) has no run-level outcome, only one `outcome` per symbol entry, and no source-commit field at all; `quoin-assurance` reads no run file itself. So the outcome cannot be shown without choosing among entries (which FR-196 forbids), and the source commit is never recorded, which makes `run_identity_unrecorded` fire on every run-backed criterion and keeps DOS-VR permanently `incomplete`. Define the outcome as the outcomes of the criterion's bound symbols in that run, and either drop source commit or name the field that holds it (the host-observed `RunObservation.performed_revision` of FR-116 is not a run-file value) and add it to the run record in its own requirement. | spec/functional/FR-196-dossier-verification-report-content.md:31,55,58; FR-196-AC-9; FR-196-AC-10; rust/crates/quoin-evidence/src/types/run.rs:106-192; rust/crates/quoin-evidence/src/qualify/mod.rs:120-145 | wrong-requirement |
| FND-004 | medium | The FR-193 source contract puts `software_safety_class` on the `systems-records` source. That source is quire-rs `PartRecord` (`owner`, `declared_type`, `multiplicity`) wrapped by `quoin-quire::systems::Part`; it has no frontmatter. The class is a frontmatter field of the `part` document (spec-objects-architecture FR-012, record key `softwareSafetyClass`). As written the contract needs an unplanned quire-rs change, and it does not say how a `PartRecord` owner id is matched to the `part` document that carries the class. Name the part document record as the source. | spec/functional/FR-193-dossier-architecture-content.md:28; quire-rs src/semantic/systems.rs:32-39; spec-objects-architecture FR-012 | wrong-requirement |
| FND-005 | low | FR-192, FR-193 and FR-194 now depend on FR-160, NFR-034, quire-rs FR-090 and spec-objects-architecture FR-012, and cite them in the body, but none of these edges is added to their frontmatter `relationships`, so `quire trace` and blast-radius queries miss them. | spec/functional/FR-192-dossier-srs-content.md:27,41; spec/functional/FR-193-dossier-architecture-content.md:28; spec/functional/FR-194-dossier-trace-matrix-content.md:43 | missing-requirement |
| FND-006 | low | FR-192, FR-193 and FR-194 cite NFR-034 for the no-inference rule, but NFR-034's Scope applies only to `quoin evidence audit --class-policy`, `quoin matrix --class-policy` and `quoin class obligations`. A reader following the link finds the rule does not cover the dossier. Extend NFR-034's Scope (and its metric) to `quoin dossier`, or cite it as context only. | spec/functional/FR-192-dossier-srs-content.md:41; spec/non-functional/NFR-034-class-reports-state-and-never-assign.md | wrong-requirement |
| FND-007 | low | FR-192 names the carried field `software_safety_class` directly, while FR-160 states "Quoin holds no field name" and takes it from the policy's `class_field`. Both can stand, but FR-192 should say that the dossier reads the carried name the ISO module declares (spec-artifacts-iso FR-009) and why that differs from FR-160. | spec/functional/FR-192-dossier-srs-content.md:27; FR-160 | wrong-requirement |

## Verdict

Not mergeable: FND-001, FND-002 and FND-003 are high. FND-001 belongs to PLAT-1814,
FND-002 to PLAT-1810, FND-003, FND-004 and FND-005..007 to PLAT-1813. FR-107, FR-164,
FR-166/167, FR-195, FR-201 and FR-211 are not contradicted by the change. FR-166/167
build release-snapshot findings through the `risk-summary` code path; the new fields do
not change the row `findings` they read, so FND-002's fix need not touch them.

## New findings (disposition pass 1)

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-008 | low | FR-193 matches a part's class to the `PartRecord` "by the part document's id" (available as `quoin-quire` `Part.document`), but the architecture generator today keys parts by `PartRecord.owner`, the owning declaration. The spec is precise; the coder must key the class line and the `safety_class_unassigned` subject by the part document id and not attach it to the owner. Candidate for accepted-no-change (implementation note). | spec/functional/FR-193-dossier-architecture-content.md:34; rust/crates/quoin-dossier/src/architecture.rs:185-188 | - |

## Dispositions

| FND | outcome | reason |
| --- | ------- | ------ |
| FND-001 | fixed | Rounds 1 and 2 (e53273f1, 750089e5). DOS-ROP export identity is `--release` only; explicit selection without `--release` exits 4 with quire code 3 and `--output` unchanged (FR-191-AC-29); unfiltered `export` includes DOS-ROP only with `--release` (FR-191-AC-30); AC-13, AC-24 and FR-207-AC-10 rewritten. Matches quire-cli FR-034/FR-039/FR-040. |
| FND-002 | fixed | Round 1 (e53273f1). FR-169 amended: top-level keys `attribution`, `rows`, `benefit_risks`, `overall_evaluations`; new row keys; `rationale` on residual entries; reads `overall_residual_risk_evaluation`; AC-1/AC-2 amended, AC-29..34 new. FR-199 Inputs cite FR-169. |
| FND-003 | fixed | Round 1 (e53273f1). FR-196 reads the real `RunRecord` members (`suite`, `tool`, `timestamp`, per-entry `symbol`, `outcome`, `trace_ids`), shows a tally of recorded outcomes, and shows source revision `not recorded` with no gap; `run_identity_unrecorded` only for an empty `timestamp` or `tool`. |
| FND-004 | fixed | Round 1 (e53273f1). FR-193 source contract: class from the `part` document frontmatter, record key `softwareSafetyClass`, exposed by `quoin-quire` beside each `Part`. |
| FND-005 | fixed | Round 1 (e53273f1). FR-192/193/194 frontmatter add FR-160, quire-rs FR-090 and spec-artifacts-iso FR-009 or spec-objects-architecture FR-012 edges. |
| FND-006 | fixed | Round 1 (e53273f1). NFR-034 citations removed; the no-inference rule is stated in each FR on its own. |
| FND-007 | fixed | Round 1 (e53273f1). FR-192 reads "the carried name that the ISO module declares for the class (spec-artifacts-iso FR-009)" and states why it differs from FR-160. |
