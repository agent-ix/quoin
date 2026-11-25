---
id: SR-967175026302197768900474181941935084777
title: "Code review — Quoin validation report and anomaly selection"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/{build.rs,src/help.rs,src/selftest.rs,src/validation.rs,src/validation/report.rs,tests/tc_217_validation_report.rs,tests/tc_218_validation_signoff.rs}, spec/functional/FR-217-validation-report.md, spec/validation/external-tools.v1.json"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-217", type: references }
---

## Summary

Reviewed the PR #1075 Rust implementation, changed report requirements, and focused test bindings. The review found duplicated decision sources, incorrect version-specific anomaly sourcing, and evidence gaps that can make the report incomplete or wrong while appearing complete.

## Verdict

**FAIL** — high-severity correctness and source-of-truth findings remain; the ticket scope also leaves explicit criteria unimplemented or untested.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies to report rendering, though its primary concern is Quoin's own measurement and finding-quality reports rather than this validation report. I reviewed report evidence completeness, provenance and explicit gaps against that context. No applicable ArchitectureDescription, MeasurementPlan/records, exception record, independence plan, structural-gap disposition, or producer-reliance decision for this validation report was found in the reviewed paths. AP-202 does not apply because this change does not evaluate or integrate a Jev semantic lens. The project policy is `recommend`, so this direct review remains an advisory review operation, not a workflow-enforced gate.

## Review Scope

- Changed source: `build.rs`, `validation.rs`, `selftest.rs`, `report.rs`, help text.
- Changed tests: report unit tests, report command integration tests, real EA sign-off test.
- Changed requirement FR-217, the external-tools index, and affected FR-218/FR-220 contracts.
- Rust idioms: repository `.claude/skills/rust-style/SKILL.md`; changed-crate source and test conventions.
- Focused checks reported by the coder: report unit tests 4/4, command integration tests 2/2, ignored sign-off test 1/1, changed-file formatting passed. I did not rerun the build lane.
- Local tool context: `/home/peter/.npm-global/bin/quoin` returned `quoin 0.36.8`; `quoin module list` returned `spec-artifacts-process@v0.36.0`; `quire --version` returned `quire 0.52.0 (engine 0.62.2)`. The session AGENTS note names Quoin 0.30.0 and Quire CLI 0.40.0/engine 0.56.0; the installed binaries are newer. Commands succeeded with no stderr, so this did not block review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `REQUIRED_TOOL_IDS` repeats the canonical nine IDs in `external-tools.v1.json`, creating a second source of truth that can drift. | rust/crates/quoin-cli/src/validation/report.rs:23, spec/validation/external-tools.v1.json:4 |
| FND-002 | high | `feature_ratings` duplicates the existing completeness assessor and collapses duplicate equal ratings into one value, so conflicting record multiplicity can be reported as an ordinary rating. | rust/crates/quoin-cli/src/validation/report.rs:575, rust/crates/quoin-completeness/src/ratings.rs:42 |
| FND-003 | high | The report selects anomalies embedded in the report-generator binary, not the binary/version that produced each run record; old retained runs can therefore be paired with a different anomaly set. | rust/crates/quoin-cli/src/validation/report.rs:109, spec/functional/FR-217-validation-report.md:39 |
| FND-004 | high | An invalid or partial external-tools index drops the whole index and all per-tool rows instead of reporting affected tool evidence as `not provided`. | rust/crates/quoin-cli/src/validation/report.rs:709, spec/functional/FR-220-external-tool-configuration-index.md:28 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified on PR head 79eaa3e8. |
| FND-002 | fixed | Verified on PR head 79eaa3e8. |
| FND-003 | fixed | Verified on PR head 79eaa3e8. |
| FND-004 | fixed | Verified on PR head 79eaa3e8. |

| FND-005 | fixed | AC-8 is now bound to tc_230_run_record_retains_versioned_anomaly_set_and_absence_state, which asserts provided state, producer version equality with toolVersion, authored path/raw documents, and provided-empty versus absent serialization. |
+++ [reviewer dispositions]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: let external_tools_index = embedded_external_tools_index(EMBEDDED_EXTERNAL_TOOLS_INDEX);
- fnd: FND-002
  outcome: fixed
  after_excerpt: let states = assess_feature_ratings(&feature_documents, repository_namespace);
- fnd: FND-003
  outcome: fixed
  after_excerpt: if set.producer_version != version || version == "not provided" {
- fnd: FND-004
  outcome: fixed
  after_excerpt: 'let mut ids = index.entries.keys().cloned().collect::<BTreeSet<_>>();

    ids.extend(index.invalid_ids.iter().cloned());

    ids.extend(evidence_by_id.keys().map(|id| (*id).to_owned()));'
- fnd: FND-005
  outcome: fixed
  after_excerpt: >-
    Given a self-test run, the retained record preserves the producing binary's anomaly-set provided state, producer version, and authored anomaly documents; the producer version equals toolVersion, and provided-empty differs from absent.
```

+++

## New findings (replacement PR #1077 review)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | FR-236-AC-8 is untagged; the retained anomaly-set provided state, producer version, raw documents, version equality, and empty-vs-absent behavior have no criterion-bound test. | FR-236-AC-8 |

The current matrix command was `quoin matrix --repo . --json`; FR-236-AC-8 reports `static_status: untagged`, `binders: []`, and “no evidence is bound to FR-236-AC-8”. The existing `tc_230_run_record_serializes_run_context_and_case_outcomes` test is tagged to FR-236-AC-1, AC-2, and AC-4, not AC-8; its current assertions do not cover all AC-8 behavior.
