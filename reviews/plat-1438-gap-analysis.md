---
id: SR-20261115210131
title: gap-analysis — PLAT-1438 dossier trace matrix and RMM
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; origin/main...331bc4bf9d7caebd80c1966df14a5d5e2d0634f7; rust/,
  spec/; PLAT-1438
review_set: subset
relationships:
- target: ix://agent-ix/quoin/FR-194
  type: references
- target: ix://agent-ix/quoin/FR-195
  type: references
---

## Summary

Ticket: PLAT-1438. Planless gap-analysis attempted the whole-repository matrix and coverage census at `331bc4bf9d7caebd80c1966df14a5d5e2d0634f7`. The computed binding results are inconsistent with the inspected source tags, so full-repository assurance is unavailable; a separately identified manual check covers the 15 FR-194/FR-195 acceptance criteria and the changed implementation surface.

## Verdict

**CONDITIONAL** — no changed-scope trace or stub defect was confirmed manually, but the installed toolchain cannot provide a trustworthy full-repository trace verdict for this run. This is not a repository PASS and does not claim all global gaps are closed.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies conservatively because these generated report views communicate missing design and evidence. The report keeps both status axes, criticality and explicit gaps; RMM open states are matched exhaustively, and row order is preserved. AP-202 does not apply: no semantic lens is integrated or evaluated.

The selected authored requirements and source-to-renderer boundaries were available. No measurement threshold or comparison changed. No selected architecture assessment, impact record, producer-reliance decision, structural coverage disposition or independent measurement record was supplied for this change; these are unavailable context, not evidence of safety. Available test logs are author-produced conformance evidence. Active exceptions: none found. No stale selected record was relied on.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Semantic review: skipped; the optional semantic audit was not authorized. Ordinary code-review and test-assertion checks remain part of the separate code-review artifact.

Reconciliation attempted: `quoin matrix --repo /home/peter/dev/quoin/.worktrees/plat-1438-generate-trace-matrix-and-rmm --json` with `/home/peter/.npm-global/bin/quoin` 0.35.0, and `quire coverage --scope /home/peter/dev/quoin/.worktrees/plat-1438-generate-trace-matrix-and-rmm --json` with `/home/peter/.npm-global/bin/quire` 0.45.0 (engine 0.60.0), machine `terra`. Both exited 0. The release notice in session names 0.30.0 / 0.40.0 (engine 0.56.0); these installed tools are newer, and no older binary was substituted. spec-artifacts-process resolves to v0.31.0.

The raw matrix reports 2,126 criteria: 18 tagged, 1,930 untagged, 141 method-without-symbol and 37 retired; all 2,126 have `no run evidence`. The coverage report nevertheless counts 2,917 Rust candidates, 2,373 tagged and 1,968 bound, while all 15 target criteria appear untagged. Its 287 untracked symbols and unrelated declaration diagnostics are retained in `coverage.json`, not attributed to this PR. There is no execution-evidence claim from this empty store.

This inconsistency prevents treating the global matrix as a verified defect list. The smallest probe at `trace-probe/` contains one requirement and one canonical Rust trace marker; `quire matrix --scope /tmp/plat-1438-review/trace-probe --format json` correctly returns tagged. Thus the failure is specific to the repository census or installed module/tool combination; a version mismatch alone is not established as its cause. `matrix.json`, `coverage.json`, `coverage.stderr` and `trace-probe.json` retain the evidence. No tooling repair was attempted on the frozen branch.

Manual fallback: each of the 15 target criteria has an explicit `#[trace]` marker on a real asserted unit or CLI test. Examined bindings are listed below. The changed source adds two document generators and two host source adapters, all owned by FR-194/FR-195 and FR-191. Within this changed surface: four behaviors inventoried, zero untraced behaviors, zero source stubs, zero test stubs. Whole-tree source ownership and all 1,930 reported global untagged criteria are not certified by this scoped manual fallback.

The ignored real-tool PDF/DOCX case is named and carries a dependency/lane reason; the other target criteria also have ordinary nonignored tests. Its execution remains unavailable.

## Examined scope

```yaml
scope:
- id: FR-194-AC-1
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: The set of rows equals the set of `quoin matrix --json` criteria for the
    fixture, one to one, in dossier order
- id: FR-194-AC-2
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: A criterion with evidence status `undischarged` appears with a `no_evidence`
    gap
- id: FR-194-AC-3
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: An empty population renders the `reason` text, a `no_requirements` gap
    and status `incomplete`
- id: FR-194-AC-4
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: A criterion with no test tag appears with an `unbacked_criterion` gap
- id: FR-194-AC-5
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: Each row shows design cell, method, static status and evidence status as
    `MatrixOutput` reports them, and criterion statement and criticality as the requirement
    documents state them
- id: FR-194-AC-6
  path: spec/functional/FR-194-dossier-trace-matrix-content.md
  role: examined
  excerpt: A requirement with no `criticality` shows `unassigned` on each of its rows
    and one `criticality_unassigned` gap
- id: FR-195-AC-1
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: The rows equal the `quoin discharge --rmm --json` rows for the fixture
    binding, in the source's order
- id: FR-195-AC-2
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: A pack with two clause sets gives two tables in index order
- id: FR-195-AC-3
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: With `--doc rmm` and no `--clause-binding` and no `--pack` the document
    has a `no_clause_binding` gap, status `incomplete`, the command exits 1 and not
    2, and no `--as-of` is needed
- id: FR-195-AC-4
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: A fixture whose binding and pack carry clause text and whose clauses are
    marked restricted in `quire clauses` renders only clause ids and the source row
    members; no clause text appears anywhere in the file or in a gap detail
- id: FR-195-AC-5
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: Rows with `compliance` `undischarged`, `attestation_not_current`, `temporary_exception_expired`
    and `undetermined` each produce one `clause_open` gap
- id: FR-195-AC-6
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: '`As of` and `unusedFacts` appear in the document as the source reports
    them'
- id: FR-195-AC-7
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: '`--doc rmm --clause-binding <file>` without `--as-of` exits 2 and writes
    nothing'
- id: FR-195-AC-8
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: Rows with `compliance` `compliant`, `tailored` and `not_applicable`, including
    `tailored` and `not_applicable` rows with empty `evidenceRefs`, produce no `clause_open`
    gap
- id: FR-195-AC-9
  path: spec/functional/FR-195-dossier-rmm-content.md
  role: examined
  excerpt: Swapping two `--clause-binding` options swaps the two tables and changes
    nothing else in the file
- id: FR-191
  path: spec/functional/FR-191-dossier-command.md
  role: examined
  excerpt: '| 2 | Usage error or invalid input, including a missing `--version` and
    a missing `--as-of` for a supplied RMM or SOUP binding (FR-195, FR-197). |'
- id: rust/crates/quoin-assurance/src/matrix.rs
  path: rust/crates/quoin-assurance/src/matrix.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The evidence-backed test matrix (FR-115).

    //!

    //! quire''s `coverage_matrix` says, per criterion, whether a test is **tagged**

    //! for it. It cannot say whether that test ran and passed, whether the run is

    //! current, or whether the binding behind it still holds. The binding graph

    //! and the auditor''s report already answer those three questions per

    //! obligation, so this module is a join over them and nothing mor'
- id: rust/crates/quoin-assurance/src/matrix/render.rs
  path: rust/crates/quoin-assurance/src/matrix/render.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! Markdown rendering for the evidence-backed test matrix.


    use std::fmt::Write as _;


    use super::{EvidenceDetail, MatrixCriterion, MatrixOutput, MatrixRequirement,
    UNASSIGNED};


    /// Render the matrix as markdown.

    ///

    /// One `##` section per requirement, in the payload''s order, each holding one

    /// table. An empty matrix renders its reason, never an empty table.

    #[must_use]

    pub fn render_markdown(matrix: &MatrixOutpu'
- id: rust/crates/quoin-cli/src/dossier.rs
  path: rust/crates/quoin-cli/src/dossier.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! `quoin dossier generate | check | export` (FR-191, PLAT-1614).

    //!

    //! This module owns syntax and rendering only. The registry, the writer, the

    //! comparison and the `quire export` hand-off live in `quoin-dossier`; the

    //! exit status comes from the error code or the report, never from a number

    //! chosen here.


    use std::path::PathBuf;


    use clap::{Arg, ArgAction, ArgMatches, Command};

    use ix_cli_kit::exit::Outcom'
- id: rust/crates/quoin-cli/src/evidence/audit.rs
  path: rust/crates/quoin-cli/src/evidence/audit.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n// Copyright (C) 2026 Agent-IX\n\
    \n//! The retained read-only `quoin evidence audit` adapter.\n\nuse clap::{Arg,\
    \ ArgAction, ArgMatches, Command};\nuse ix_cli_kit::exit::Outcome;\nuse quoin_core::protocol::Response;\n\
    \nuse super::{json_arg, repo_arg, required};\nuse crate::core_runtime::invoke;\n\
    \npub(super) fn command() -> Command {\n    Command::new(\"audit\")\n        .about(\"\
    Audit the evidence store: staleness, vacuity\")\n        .arg(repo_arg())\n  \
    \      .arg(modu"
- id: rust/crates/quoin-cli/tests/tc_1438_dossier_matrices.rs
  path: rust/crates/quoin-cli/tests/tc_1438_dossier_matrices.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n// Copyright (C) 2026 Agent-IX\n\
    \n//! Cross-check dossier documents against the matrix and discharge commands\n\
    //! over the same real fixture inputs (FR-194, FR-195).\n\n#![allow(\n    clippy::unwrap_used,\n\
    \    clippy::expect_used,\n    clippy::indexing_slicing,\n    clippy::panic,\n\
    \    reason = \"in a test, a panic IS the failure report; the production lints\
    \ stand\"\n)]\n\nuse std::fs;\nuse std::path::{Path, PathBuf};\nuse std::process::{Command,\
    \ Output};\n\nuse ix_tr"
- id: rust/crates/quoin-dossier/src/engine.rs
  path: rust/crates/quoin-dossier/src/engine.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! `generate` and `check` (FR-191).

    //!

    //! Every selected document is computed in memory before anything is written

    //! or compared, so a run that exits 2 or 3 has touched nothing.


    use std::fs;

    use std::io::Write as _;

    use std::path::{Path, PathBuf};


    use tempfile::TempPath;


    use time::OffsetDateTime;

    use time::format_description::well_known::Rfc3339;


    use crate::error::{DossierError, DossierErrorCode};

    use crate::g'
- id: rust/crates/quoin-dossier/src/lib.rs
  path: rust/crates/quoin-dossier/src/lib.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The machinery behind `quoin dossier` (FR-191, PLAT-1614): the document

    //! registry, the gap model and its deterministic order, the `DossierDocument`

    //! frontmatter and `Gaps` writer, `generate` / `check`, and the export

    //! hand-off to `quire export`.

    //!

    //! The SRS, architecture, trace matrix and RMM generators register in

    //! [`builtin_registry`] in fixed bundle order. The host supplies the matrix

    //! and disc'
- id: rust/crates/quoin-dossier/src/markdown.rs
  path: rust/crates/quoin-dossier/src/markdown.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n// Copyright (C) 2026 Agent-IX\n\
    \n//! Escaping for text a generator copies out of a source document, so one value\n\
    //! can never end its table cell, its heading or its section (FR-192, FR-193).\n\
    \nuse quoin_quire::records::{Contract, Corpus};\n\nuse crate::error::SourceFailure;\n\
    \n/// One table cell: a `|` would end the cell and a newline the row.\npub(crate)\
    \ fn cell(text: &str) -> String {\n    text.replace('\\\\', \"\\\\\\\\\")\n  \
    \      .replace('|', \"\\\\|\")\n        .r"
- id: rust/crates/quoin-dossier/src/registry.rs
  path: rust/crates/quoin-dossier/src/registry.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The document registry (FR-191).

    //!

    //! A generator is registered once per document kind. Registry order is the

    //! fixed bundle order and the order of every report, so the SRS,

    //! architecture, trace-matrix and RMM generators of later tickets register in

    //! that order and nothing else decides it.


    use std::path::PathBuf;


    use crate::error::{DossierError, DossierErrorCode, SourceFailure};

    use crate::gap::Gap;


    //'
- id: rust/crates/quoin-dossier/src/rmm.rs
  path: rust/crates/quoin-dossier/src/rmm.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The dossier requirements mapping matrix (FR-195).


    use std::fmt::Write as _;


    use quoin_assurance::{RequirementsMapping, RmmCompliance};

    use serde::Serialize;

    use serde_json::Value;


    use crate::markdown::{cell, line};

    use crate::{DocumentSpec, Gap, GapKind, Generated, Generator, Inputs, RmmSource,
    SourceFailure};


    pub(crate) struct RmmGenerator(pub(crate) fn(&Inputs) -> Result<RmmSource, SourceFailure>);


    impl Gene'
- id: rust/crates/quoin-dossier/src/sources.rs
  path: rust/crates/quoin-dossier/src/sources.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! Source adapters for the trace matrix and RMM (FR-194, FR-195).


    use quoin_assurance::{PackRequirementsMapping, RequirementsMapping, matrix::MatrixOutput};


    use crate::{Inputs, SourceFailure};


    /// The source assembly supplied by the host. The CLI supplies the same

    /// library paths it uses for `matrix` and `discharge`, including their evidence

    /// joins and configured module roots.

    #[derive(Clone, Copy)]

    pub struct'
- id: rust/crates/quoin-dossier/src/srs.rs
  path: rust/crates/quoin-dossier/src/srs.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! SRS document generator (FR-192, PLAT-1437).

    //!

    //! Lists every requirement in scope with id, title, statement, criteria and

    //! recorded criticality.


    use std::fmt::Write as _;


    use quoin_quire::records::Document;

    use serde_json::Value;


    use crate::error::SourceFailure;

    use crate::gap::{Gap, GapKind};

    use crate::markdown::{cell, line, prose, unusable_input};

    use crate::order::dossier_cmp;

    use crate::registry::{Doc'
- id: rust/crates/quoin-dossier/src/testing.rs
  path: rust/crates/quoin-dossier/src/testing.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n// Copyright (C) 2026 Agent-IX\n\
    \n//! Fixtures the unit tests of this crate share: a configurable generator, a\n\
    //! registry builder, run inputs and a directory listing.\n\n#![allow(\n    clippy::unwrap_used,\n\
    \    clippy::expect_used,\n    clippy::indexing_slicing,\n    clippy::panic,\n\
    \    reason = \"in a test, a panic IS the failure report; the production lints\
    \ stand\"\n)]\n\nuse std::fs;\nuse std::path::{Path, PathBuf};\n\nuse crate::error::SourceFailure;\n\
    use crate:"
- id: rust/crates/quoin-dossier/src/trace_matrix.rs
  path: rust/crates/quoin-dossier/src/trace_matrix.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX


    //! The evidence-backed dossier trace matrix (FR-194).


    use std::fmt::Write as _;


    use quoin_assurance::matrix::{EvidenceStatus, MatrixOutput, design_cell};

    use quoin_quire::records::Document;


    use crate::markdown::{cell, line, prose, unusable_input};

    use crate::order::dossier_cmp;

    use crate::srs::{criteria_of, criticality_of};

    use crate::{DocumentSpec, Gap, GapKind, Generated, Generator, Inputs, SourceFailure};


    pub(c'
- id: rust/crates/quoin-dossier/tests/tc_1437_generators.rs
  path: rust/crates/quoin-dossier/tests/tc_1437_generators.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n// Copyright (C) 2026 Agent-IX\n\
    \n//! The SRS and architecture generators run through `generate` over a real\n\
    //! module and repository (FR-192, FR-193, PLAT-1437): the fixture the\n//! `quire.*`\
    \ routes of quoin-core use, read in place.\n\n#![allow(\n    clippy::unwrap_used,\n\
    \    clippy::expect_used,\n    clippy::indexing_slicing,\n    clippy::panic,\n\
    \    reason = \"integration-test bodies: a panic here is a failing test, which\
    \ is the intended signal\"\n)]\n\nuse st"
bindings:
- test_id: rows_preserve_the_real_matrix_builder_values_and_join_authored_statements
  ac_id: FR-194-AC-1
  trace: correct
- test_id: rows_preserve_the_real_matrix_builder_values_and_join_authored_statements
  ac_id: FR-194-AC-2
  trace: correct
- test_id: rows_preserve_the_real_matrix_builder_values_and_join_authored_statements
  ac_id: FR-194-AC-4
  trace: correct
- test_id: rows_preserve_the_real_matrix_builder_values_and_join_authored_statements
  ac_id: FR-194-AC-5
  trace: correct
- test_id: rows_preserve_the_real_matrix_builder_values_and_join_authored_statements
  ac_id: FR-194-AC-6
  trace: correct
- test_id: an_empty_matrix_preserves_its_reason_as_a_gap
  ac_id: FR-194-AC-3
  trace: correct
- test_id: criterion_rows_use_dossier_order_within_their_requirement
  ac_id: FR-194-AC-1
  trace: correct
- test_id: criterion_statements_join_by_criterion_id_across_source_documents
  ac_id: FR-194-AC-5
  trace: correct
- test_id: binding_order_and_every_source_member_are_preserved
  ac_id: FR-195-AC-1
  trace: correct
- test_id: binding_order_and_every_source_member_are_preserved
  ac_id: FR-195-AC-6
  trace: correct
- test_id: binding_order_and_every_source_member_are_preserved
  ac_id: FR-195-AC-9
  trace: correct
- test_id: pack_context_and_index_precede_tables_in_index_order
  ac_id: FR-195-AC-2
  trace: correct
- test_id: compliance_alone_decides_open_gaps_even_with_empty_evidence
  ac_id: FR-195-AC-5
  trace: correct
- test_id: compliance_alone_decides_open_gaps_even_with_empty_evidence
  ac_id: FR-195-AC-8
  trace: correct
- test_id: no_binding_is_incomplete_exit_one_without_as_of
  ac_id: FR-195-AC-3
  trace: correct
- test_id: a_binding_or_pack_requires_as_of_before_reading_or_writing
  ac_id: FR-195-AC-7
  trace: correct
- test_id: dossier_rows_equal_the_evidence_backed_matrix_command
  ac_id: FR-194-AC-1
  trace: correct
- test_id: dossier_rows_equal_the_evidence_backed_matrix_command
  ac_id: FR-194-AC-5
  trace: correct
- test_id: an_empty_matrix_population_renders_the_source_reason
  ac_id: FR-194-AC-3
  trace: correct
- test_id: dossier_rows_equal_the_discharge_command_member_for_member
  ac_id: FR-195-AC-1
  trace: correct
- test_id: dossier_rows_equal_the_discharge_command_member_for_member
  ac_id: FR-195-AC-6
  trace: correct
- test_id: dossier_rows_equal_the_discharge_command_member_for_member
  ac_id: FR-195-AC-8
  trace: correct
- test_id: a_restricted_pack_uses_index_order_and_never_renders_clause_text
  ac_id: FR-195-AC-2
  trace: correct
- test_id: a_restricted_pack_uses_index_order_and_never_renders_clause_text
  ac_id: FR-195-AC-4
  trace: correct
```
