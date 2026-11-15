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
  `evidence_detail`, and optional `analysis_evidence`. The requirement
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
  MatrixCriterion does not carry a newest run outcome or timestamp; the
  report states only the source findings and bound suite identities.

## Behavior

- The report SHALL create one row per criterion in `MatrixOutput`, in source
  requirement and criterion order, and SHALL copy its method and Evidence
  Status without recomputing either.
- The report SHALL list the criterion's traced bindings and retained
  Inspection, Analysis and gate evidence that the matrix supplies, preserving
  their source identities.
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
| FR-196-AC-3 | A source finding with typed `failed-run` cause yields `evidence_failed`, names the criterion, shows that typed cause without inventing a run outcome, and never calls it passing. Changing only the finding summary gives identical classification. | Test |
| FR-196-AC-4 | A `stale` criterion with a missing-run cause yields `evidence_stale`; a `suspect` criterion yields `evidence_suspect`; neither yields `evidence_failed`. | Test |
| FR-196-AC-5 | `undischarged` and `no run evidence` each yield `no_evidence`, and a criterion without a binder also yields `unbacked_criterion`. | Test |
| FR-196-AC-6 | A criterion backed only by retained Inspection evidence shows that record and receives no gap when the matrix says `bound`. | Test |
| FR-196-AC-7 | A requirement with no criteria remains listed with `no_acceptance_criteria` and no fabricated criterion row. | Test |
| FR-196-AC-8 | An empty matrix displays the source `reason`, produces document-level `no_requirements`, and is `incomplete`; two runs over the same inputs give byte-identical Markdown. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md), and the typed source cause
  in PLAT-1760 (which blocks PLAT-1617).
- **Downstream**: PLAT-1617 implements this document.
