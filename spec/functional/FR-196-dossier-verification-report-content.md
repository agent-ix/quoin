---
id: FR-196
title: "The dossier verification report renders matrix evidence for every criterion"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
---

# FR-196: The dossier verification report renders matrix evidence for every criterion

## Description

When `quoin dossier generate` selects `verification-report` (document id
`DOS-VR`), the verification report SHALL render every live acceptance
criterion and the evidence status that `quoin matrix --json` gives it.

## Inputs

- `quoin matrix --repo <dir> --json`, produced by
  `quoin_assurance::matrix::build` in
  `rust/crates/quoin-assurance/src/matrix.rs` ([FR-115](./FR-115-evidence-backed-test-matrix.md)).
  The dossier calls the same library path; it does not evaluate evidence.
- Each `MatrixCriterion` supplies `id`, `method`, `evidence_status`,
  `evidence_detail`, optional `analysis_evidence` and, when the criterion has a retained run file, a run identity: the newest retained run file's timestamp, tool name, tool version, outcome as recorded, source commit as recorded and file path (the `quoin-assurance` source contract for this document; each value is omitted when the run file does not state it, and the whole identity is omitted when no run file is retained). The requirement
  documents supply the title and recorded criticality.
- A typed `failed-run` cause on a source `stale-evidence` finding, supplied by
  the matrix source work in PLAT-1760. `kind: stale-evidence` and a prose
  summary alone cannot distinguish a failed run from a missing run. The
  generator cannot register until PLAT-1760 is complete.

## Outputs

- `dossier/DOS-VR.md`: `# Verification report`, a `## Summary` count table
  by source Evidence Status, one `## <requirement id>: <title>` section per
  requirement in dossier order, and the final `## Gaps` section.
- Each requirement section has a `Criticality:` line and a table
  `Criterion | Method | Evidence | Source findings | Evidence status`.
  Each criterion also shows its run identity (below), and its source findings and bound suite identities.

## Behavior

- The report SHALL create one row per criterion in `MatrixOutput`, in source
  requirement and criterion order, and SHALL copy its method and Evidence
  Status without recomputing either.
- The report SHALL list the criterion's traced bindings and retained
  Inspection, Analysis and gate evidence that the matrix supplies, preserving
  their source identities.
- The report SHALL render, for each criterion, five run-identity values from the newest retained run file for that criterion: run timestamp, tool name and version, outcome as the run file records it, source commit as the run file records it, and the run-file path. A value the source does not state SHALL read `not recorded`.
- The report SHALL NOT compute, normalise or choose a run outcome.
- The report SHALL show the recorded run outcome verbatim.
- When a criterion has no retained run file, or its run identity omits any of the five values, the report SHALL produce one `run_identity_unrecorded` gap naming the criterion and the missing values. A criterion whose evidence status is satisfied only by Inspection or Analysis records does not need a run file and SHALL NOT receive this gap on that account.
- The summary SHALL count the rendered rows by the five source Evidence Status
  values; it SHALL NOT compute a percentage.
- When a source finding has typed cause `failed-run`, the report SHALL produce
  an `evidence_failed` gap naming the criterion and show that source cause,
  never a passing result or an invented run outcome.
- The generator SHALL NOT detect a failed run by parsing a finding summary.
- A `stale` criterion without a typed failed-run cause SHALL produce an
  `evidence_stale` gap; a `suspect` criterion SHALL produce an
  `evidence_suspect` gap.
- When a criterion is `undischarged` or `no run evidence`, the report SHALL
  produce a `no_evidence` gap.
- When a criterion has no traced test or retained record binder, the report SHALL
  also produce `unbacked_criterion`.
- When a requirement has no criteria, the report SHALL leave it visible and
  produce `no_acceptance_criteria` without inventing a criterion row.
- An empty matrix population SHALL show the source `reason` verbatim and
  produce a document-level `no_requirements` gap, as FR-194 does; each listed
  requirement without criteria also receives `no_acceptance_criteria`.
- A missing method or criticality SHALL appear as `not stated` or `unassigned`,
  respectively; the report SHALL NOT choose a method or criticality.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-196-AC-1 | A fixture with six matrix criteria gives six report rows with the same ids, methods and Evidence Status values in source order. | Test |
| FR-196-AC-2 | The summary counts equal counts computed from those six source rows, and the document contains no percentage. | Test |
| FR-196-AC-3 | A source finding with typed `failed-run` cause yields `evidence_failed`, names the criterion, shows that typed cause and the recorded run outcome verbatim, never calls it passing and computes no outcome. Changing only the finding summary gives identical classification. | Test |
| FR-196-AC-4 | A `stale` criterion with a missing-run cause yields `evidence_stale`; a `suspect` criterion yields `evidence_suspect`; neither yields `evidence_failed`. | Test |
| FR-196-AC-5 | `undischarged` and `no run evidence` each yield `no_evidence`, and a criterion without a binder also yields `unbacked_criterion`. | Test |
| FR-196-AC-6 | A criterion backed only by retained Inspection evidence shows that record and receives no gap when the matrix says `bound`. | Test |
| FR-196-AC-7 | A requirement with no criteria remains listed with `no_acceptance_criteria` and no fabricated criterion row. | Test |
| FR-196-AC-8 | An empty matrix displays the source `reason`, produces document-level `no_requirements`, and is `incomplete`; two runs over the same inputs give byte-identical Markdown. | Test |
| FR-196-AC-9 | A criterion with a retained run file shows the run's timestamp, tool name and version, outcome, source commit and file path exactly as recorded, and the outcome is not recomputed (a run file recording an outcome the matrix status disagrees with shows both unchanged). | Test |
| FR-196-AC-10 | A criterion whose newest run file omits the source commit shows `not recorded` for it, shows the other four values, and produces one `run_identity_unrecorded` gap naming the criterion and the source commit. | Test |
| FR-196-AC-11 | A criterion with no retained run file shows `not recorded` for all five values and one `run_identity_unrecorded` gap; a criterion backed only by retained Inspection evidence receives no such gap. | Test |
| FR-196-AC-12 | With two retained run files for one criterion, the report shows the identity of the newer one only. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md), and the typed source cause
  in PLAT-1760 (which blocks PLAT-1617).
- **Downstream**: PLAT-1617 implements this document.
