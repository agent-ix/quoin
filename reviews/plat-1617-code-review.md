---
id: SR-20261115210124
title: "code-review review of PLAT-1617 verification report"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-cli/src/evidence/audit.rs, FR-196; PLAT-1617; PR #935"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-196"
    type: references
---

## Summary

Ticket: PLAT-1617. Reviewed origin/main...HEAD at a8e2d67b687e38e470f6d01be5d10b05b3647de2. Four concrete code/test defects were found.

## Verdict

**FAIL** — Required new unit tests fail, and a duplicated source conversion violates the code-review duplication rule.

## Assurance Context

AP-201 (spec/assurance/AP-201-finding-quality.md) applies to this evidence-report rendering change: concealed gaps have material impact. Reviewed source-status preservation, explicit missing evidence, binder identities, summary counts and deterministic output. AP-202 does not apply. Available context: FR-191, FR-196, NFR-040/041, direct source, generated reports and test results. Selected architecture, impact-analysis records, measurement runs, independent-adjudication records, structural-gap records and producer-reliance decisions: unavailable for this change. Stale selected context: none identified. Active exceptions: none identified. No measurement threshold or new policy was inferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The required quoin-dossier unit suite fails five new verification_report tests: render(..., &[]) adds criticality_unassigned that their expectations omit, and the unbacked test also expects no_evidence before unbacked_criterion, contrary to FR-191 gap order. Reproduced: 65 passed, 5 failed; the PR check ran only integration targets. | rust/crates/quoin-dossier/src/verification_report.rs:542 |
| FND-002 | high | cause_name duplicates the source-owned StaleCause::as_str vocabulary in quoin-finding-types/src/finding.rs:44-50. Searching the workspace for StaleCause implementations found the existing public conversion. Reuse it rather than maintaining a second valid-cause mapping; the duplicate also invents a spelling for Invalid, which the source intentionally has no representation for. | rust/crates/quoin-dossier/src/verification_report.rs:305 |
| FND-003 | medium | A stale store suite in evidence_detail.bindings suppresses unbacked_criterion even when both traced binders and retained record binders are empty. Reproduced through the public generator with static_status=untagged, binders=[], analysis_evidence=None and bindings=[old-suite]: the report emits evidence_stale but omits the mandatory unbacked_criterion gap. | rust/crates/quoin-dossier/src/verification_report.rs:357 |
| FND-004 | medium | Omitting the optional --module flag makes verification-report fail after the matrix succeeds: the requirement reader uses Closed([]) and returns no module loaded from the 0 requested module root(s). Reproduced on the inline quire-repo fixture: exit 3 without --module, exit 1 with --module set to that same fixture. Match the matrix source module-selection behavior when no explicit roots are supplied. | rust/crates/quoin-dossier/src/verification_report.rs:90 |

## Coverage

Repository: agent-ix/quoin, PR #935, HEAD a8e2d67b687e38e470f6d01be5d10b05b3647de2. No CI workflow changes or narrowed gates.

- make rust-lint: PASS (workspace fmt and clippy, all targets/features, -D warnings).
- make rust-deny: PASS; initial sandbox advisory-lock access failure was rerun successfully with escalation.
- quoin-dossier full unit suite: FAIL, 65 passed / 5 failed. Full log: /tmp/plat-1617-review/dossier-tests.log.
- dossier integration suites: PASS, 6 passed / 2 ignored across tc_1437_generators and tc_1617_verification_report.
- quoin-cli binary unit and dossier command integration: PASS, 154 + 17 passed; 3 ignored.
- Ignored real-tool verification-report validation and PDF export: PASS using /home/peter/dev/spec-artifacts-process/.worktrees/plat-1615-m2-process/spec_artifacts_process. Initial run with the ordinary sibling checkout failed because it lacks DossierDocument; the corrected module run passed.
- The initial broad touched-crate run stopped at unrelated anomaly tests because the nested worktree needs SPEC_ARTIFACTS_PROCESS_DIR. Focused suites were rerun with the documented fixture environment. The full workspace test/workflow gate is left to the lead; it cannot pass until the five new unit-test failures are fixed.

Production failure reproductions and logs are retained in /tmp/plat-1617-review. The reviewed worktree remains clean and frozen.

## Reviewer data

```yaml
clean: false
scope:
- id: FR-196-AC-1
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: A fixture with six matrix criteria gives six report rows with the same
    ids, methods and Evidence Status values in source order.
- id: FR-196-AC-2
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: The summary counts equal counts computed from those six source rows, and
    the document contains no percentage.
- id: FR-196-AC-3
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: A source finding with typed `failed-run` cause yields `evidence_failed`,
    names the criterion, shows that typed cause without inventing a run outcome, and
    never calls it passing. Changing only the finding summary gives identical classification.
- id: FR-196-AC-4
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: A `stale` criterion with a missing-run cause yields `evidence_stale`; a
    `suspect` criterion yields `evidence_suspect`; neither yields `evidence_failed`.
- id: FR-196-AC-5
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: '`undischarged` and `no run evidence` each yield `no_evidence`, and a criterion
    without a binder also yields `unbacked_criterion`.'
- id: FR-196-AC-6
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: A criterion backed only by retained Inspection evidence shows that record
    and receives no gap when the matrix says `bound`.
- id: FR-196-AC-7
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: A requirement with no criteria remains listed with `no_acceptance_criteria`
    and no fabricated criterion row.
- id: FR-196-AC-8
  path: spec/functional/FR-196-dossier-verification-report-content.md
  role: examined
  excerpt: An empty matrix displays the source `reason`, produces document-level `no_requirements`,
    and is `incomplete`; two runs over the same inputs give byte-identical Markdown.
- id: rust/Cargo.lock
  path: rust/Cargo.lock
  role: examined
  excerpt: '# This file is automatically @generated by Cargo.

    # It is not intended for manual editing.

    version = 4


    [[package]]

    name = "adler2"

    version = "2.0.1"'
- id: rust/crates/quoin-cli/src/dossier.rs
  path: rust/crates/quoin-cli/src/dossier.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! `quoin dossier generate | check | export` (FR-191, PLAT-1614).

    //!

    //! This module owns syntax and rendering only. The registry, the writer, the

    //! comparison and the `quire export` hand-off live in `quoin-dossier`; the'
- id: rust/crates/quoin-cli/src/evidence/audit.rs
  path: rust/crates/quoin-cli/src/evidence/audit.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The retained read-only `quoin evidence audit` adapter.


    use clap::{Arg, ArgAction, ArgMatches, Command};

    use ix_cli_kit::exit::Outcome;'
- id: rust/crates/quoin-dossier/Cargo.toml
  path: rust/crates/quoin-dossier/Cargo.toml
  role: examined
  excerpt: '# SPDX-License-Identifier: AGPL-3.0-or-later

    # Copyright (C) 2026 Agent-IX

    [package]

    name = "quoin-dossier"

    version.workspace = true

    edition.workspace = true

    rust-version.workspace = true'
- id: rust/crates/quoin-dossier/src/gap.rs
  path: rust/crates/quoin-dossier/src/gap.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The gap model (FR-191, "Gaps section").

    //!

    //! A gap is one missing input, reported rather than dropped or defaulted. The

    //! `gap_kind` list is closed and ordered; the position of a kind in it is the'
- id: rust/crates/quoin-dossier/src/lib.rs
  path: rust/crates/quoin-dossier/src/lib.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The machinery behind `quoin dossier` (FR-191, PLAT-1614): the document

    //! registry, the gap model and its deterministic order, the `DossierDocument`

    //! frontmatter and `Gaps` writer, `generate` / `check`, and the export

    //! hand-off to `quire export`.'
- id: rust/crates/quoin-dossier/src/srs.rs
  path: rust/crates/quoin-dossier/src/srs.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! SRS document generator (FR-192, PLAT-1437).

    //!

    //! Lists every requirement in scope with id, title, statement, criteria and

    //! recorded criticality.'
- id: rust/crates/quoin-dossier/src/verification_report.rs
  path: rust/crates/quoin-dossier/src/verification_report.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! Verification report generator (FR-196, PLAT-1617).

    //!

    //! A rendering of the evidence-backed matrix (`quoin_assurance::matrix`,

    //! FR-115). Evidence Status, method and bindings are copied from the matrix'
- id: rust/crates/quoin-dossier/tests/tc_1437_generators.rs
  path: rust/crates/quoin-dossier/tests/tc_1437_generators.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The SRS and architecture generators run through `generate` over a real

    //! module and repository (FR-192, FR-193, PLAT-1437): the fixture the

    //! `quire.*` routes of quoin-core use, read in place.

    '
- id: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs
  path: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The verification report through `generate` over a real repository

    //! (FR-196, PLAT-1617): the fixture the `quire.*` routes of quoin-core use,

    //! read in place, with the matrix a fixed `MatrixOutput`.

    '
- id: FR-191
  path: spec/functional/FR-191-dossier-command.md
  role: context_only
  excerpt: 'Quoin SHALL provide `quoin dossier generate`, which writes the selected
    dossier documents from the repository''s records.


    Quoin SHALL provide `quoin dossier check`, which compares the selected dossier
    documents with the files on disk and writes nothing.


    Quoin SHALL provide `quoin dossier export`, which exports the selected, committed
    and current dossier documents through `quire export`.


    The M1 documents are SRS (`srs`, id `DOS-SRS`), architecture

    (`architecture`, id `DOS-ARCH`), trace matrix (`trace-matrix`, id

    `DOS-TRACE`) and requirements mapping matrix (`rmm`, id `DOS-RMM`). The

    M2 docume'
- id: NFR-040
  path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md
  role: context_only
  excerpt: '`quoin dossier generate` and `quoin dossier check` SHALL give byte-identical
    Markdown for the same records and options, where the options include the order
    of `--clause-binding` and the index order of a pack.


    `quoin dossier generate` and `quoin dossier check` SHALL order output by the dossier
    order and gap order of FR-191, except where FR-195 keeps a supplied order.


    `quoin dossier generate` and `quoin dossier check` SHALL NOT read the wall clock
    or the network.


    `quoin dossier generate` and `quoin dossier check` SHALL NOT write a criticality,
    risk, compliance or acceptability value that no s'
- id: NFR-041
  path: spec/non-functional/NFR-041-m2-dossier-states-only-recorded-values.md
  role: context_only
  excerpt: 'The six M2 dossier generators SHALL render only source-reported values
    and

    explicit gaps; they SHALL NOT calculate a new score, percentage, rank, software

    class, risk acceptability or release verdict.

    '
- id: rust/crates/quoin-dossier/src/verification_report.rs::tests
  path: rust/crates/quoin-dossier/src/verification_report.rs
  role: examined
  excerpt: assert!(generated.gaps.is_empty());
- id: rust/crates/quoin-dossier/src/verification_report.rs::cause_name
  path: rust/crates/quoin-dossier/src/verification_report.rs
  role: examined
  excerpt: "fn cause_name(cause: StaleCause) -> &'static str {\n    match cause {\n\
    \        StaleCause::FailedRun => \"failed-run\",\n        StaleCause::MissingRun\
    \ => \"missing-run\",\n        StaleCause::Invalid => \"invalid\",\n    }\n}"
- id: rust/crates/quoin-dossier/src/verification_report.rs::VerificationReportGenerator::generate
  path: rust/crates/quoin-dossier/src/verification_report.rs
  role: examined
  excerpt: "let corpus = quoin_quire::records::read(\n            &inputs.scope,\n\
    \            &quoin_quire::ModuleSelection::Closed(modules),\n            &REQUIREMENT_TYPES,\n\
    \        )"
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: rust/crates/quoin-dossier/src/verification_report.rs::tests
  related_ids:
  - FR-196-AC-3
  - FR-196-AC-5
  - FR-196-AC-6
  - FR-196-AC-7
  path: rust/crates/quoin-dossier/src/verification_report.rs
  lines: 542-648
  excerpt: assert!(generated.gaps.is_empty());
  finding: 'The required quoin-dossier unit suite fails five new verification_report
    tests: render(..., &[]) adds criticality_unassigned that their expectations omit,
    and the unbacked test also expects no_evidence before unbacked_criterion, contrary
    to FR-191 gap order. Reproduced: 65 passed, 5 failed; the PR check ran only integration
    targets.'
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: other
  artifact_id: rust/crates/quoin-dossier/src/verification_report.rs::cause_name
  related_ids:
  - FR-196-AC-3
  path: rust/crates/quoin-dossier/src/verification_report.rs
  lines: 305-310
  excerpt: "fn cause_name(cause: StaleCause) -> &'static str {\n    match cause {\n\
    \        StaleCause::FailedRun => \"failed-run\",\n        StaleCause::MissingRun\
    \ => \"missing-run\",\n        StaleCause::Invalid => \"invalid\",\n    }\n}"
  finding: cause_name duplicates the source-owned StaleCause::as_str vocabulary in
    quoin-finding-types/src/finding.rs:44-50. Searching the workspace for StaleCause
    implementations found the existing public conversion. Reuse it rather than maintaining
    a second valid-cause mapping; the duplicate also invents a spelling for Invalid,
    which the source intentionally has no representation for.
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-196-AC-5
  related_ids: []
  path: rust/crates/quoin-dossier/src/verification_report.rs
  lines: 357-359
  excerpt: '`undischarged` and `no run evidence` each yield `no_evidence`, and a criterion
    without a binder also yields `unbacked_criterion`.'
  finding: 'A stale store suite in evidence_detail.bindings suppresses unbacked_criterion
    even when both traced binders and retained record binders are empty. Reproduced
    through the public generator with static_status=untagged, binders=[], analysis_evidence=None
    and bindings=[old-suite]: the report emits evidence_stale but omits the mandatory
    unbacked_criterion gap.'
- fnd: FND-004
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: rust/crates/quoin-dossier/src/verification_report.rs::VerificationReportGenerator::generate
  related_ids:
  - FR-191
  path: rust/crates/quoin-dossier/src/verification_report.rs
  lines: 90-98
  excerpt: "let corpus = quoin_quire::records::read(\n            &inputs.scope,\n\
    \            &quoin_quire::ModuleSelection::Closed(modules),\n            &REQUIREMENT_TYPES,\n\
    \        )"
  finding: 'Omitting the optional --module flag makes verification-report fail after
    the matrix succeeds: the requirement reader uses Closed([]) and returns no module
    loaded from the 0 requested module root(s). Reproduced on the inline quire-repo
    fixture: exit 3 without --module, exit 1 with --module set to that same fixture.
    Match the matrix source module-selection behavior when no explicit roots are supplied.'
bindings:
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::six_states_give_six_rows_with_the_matrix_status_and_method
  ac_id: FR-196-AC-1
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::summary_counts_the_rendered_rows_and_states_no_percentage
  ac_id: FR-196-AC-2
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::a_typed_failed_run_is_evidence_failed_whatever_the_summary_says
  ac_id: FR-196-AC-3
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::no_failing_stale_or_suspect_row_claims_a_pass
  ac_id: FR-196-AC-3
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::stale_without_a_failed_run_and_suspect_have_their_own_gaps
  ac_id: FR-196-AC-4
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::undischarged_and_no_run_evidence_are_no_evidence_and_bound_has_no_gap
  ac_id: FR-196-AC-5
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::a_criterion_without_a_binder_is_unbacked_as_well
  ac_id: FR-196-AC-5
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::a_bound_criterion_backed_only_by_a_retained_record_shows_it_and_has_no_gap
  ac_id: FR-196-AC-6
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::a_retained_record_that_is_not_bound_still_gets_its_gap
  ac_id: FR-196-AC-6
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::a_requirement_without_criteria_stays_listed_with_no_row
  ac_id: FR-196-AC-7
  trace: correct
- test_id: rust/crates/quoin-dossier/src/verification_report.rs::an_empty_matrix_shows_its_reason_and_is_deterministic
  ac_id: FR-196-AC-8
  trace: correct
- test_id: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs::the_report_titles_rows_from_the_repository_and_flags_the_failed_run
  ac_id: FR-196-AC-1
  trace: correct
- test_id: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs::the_report_titles_rows_from_the_repository_and_flags_the_failed_run
  ac_id: FR-196-AC-3
  trace: correct
- test_id: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs::generating_twice_gives_byte_identical_files
  ac_id: FR-196-AC-8
  trace: correct
- test_id: rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs::the_generated_report_validates_and_exports_to_pdf
  ac_id: FR-196-AC-1
  trace: correct
```
