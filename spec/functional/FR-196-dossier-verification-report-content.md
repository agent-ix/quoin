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
- Each `MatrixCriterion` supplies `id`, `method`, `evidence_status`, `evidence_detail`, optional `analysis_evidence` and, for each suite in `evidence_detail.bindings`, a run identity (the `quoin-assurance` source contract for this document, below). The requirement documents supply the title and recorded criticality.
- Run-identity source contract: `quoin-assurance` SHALL read, from the retained runs the host supplies to `matrix.build` (`quoin-evidence` `RetainedRun`), the record members `suite`, `tool`, `timestamp` and, per entry, `symbol` and `outcome`, the store-relative file name of each record, and the checkout revision the host observed for that record (the FR-116 `performed_revision`). An entry is bound to a criterion when its symbol is one of the `Binding.symbols` of a binding of that criterion, the auditor's own bound join; `trace_ids` are not read, because `quoin evidence record` already folds them into the binding's symbols. For each suite bound to a criterion it SHALL select the retained run of that suite with the greatest `timestamp`, the file name breaking a tie, among the runs that hold at least one entry bound to the criterion. It SHALL expose for that run: the file name, the `timestamp`, the `tool` string as recorded (one value, tool and version as the adapter reported them), a tally of the recorded `outcome` values (`pass`, `fail`, `skip`, `error`) of the entries bound to the criterion, and the observed checkout revision verbatim when the host observed one for that record. It computes no outcome, and omits the run identity of a suite with no such run. The host SHALL send `matrix.build` only, per binding, the newest run holding a bound entry and in it only the entries a binding of its suite names, so the request stays within the `matrix.build` size ceiling however many runs the append-only store holds. The store refuses an empty `tool` or a non-canonical `timestamp` when a record is written and skips such a record when it is read, so a selected run always has both.
- A typed `failed-run` cause on a source `stale-evidence` finding, supplied by
  the matrix source work in PLAT-1760. `kind: stale-evidence` and a prose
  summary alone cannot distinguish a failed run from a missing run. The
  generator cannot register until PLAT-1760 is complete.

## Outputs

- `dossier/DOS-VR.md`: `# Verification report`, a `## Summary` count table
  by source Evidence Status, one `## <requirement id>: <title>` section per
  requirement in dossier order, and the final `## Gaps` section.
- Each requirement section has a `Criticality:` line and a table
  `Criterion | Method | Evidence | Source findings | Run identity | Evidence status`.
  Each criterion also shows, per bound suite, its run identity (below), and its source findings and bound suite identities.

## Behavior

- The report SHALL create one row per criterion in `MatrixOutput`, in source
  requirement and criterion order, and SHALL copy its method and Evidence
  Status without recomputing either.
- The report SHALL list the criterion's traced bindings and retained
  Inspection, Analysis and gate evidence that the matrix supplies, preserving
  their source identities.
- The report SHALL render, for each criterion and each suite bound to it, the identity of the run that the source selects: the run timestamp, the `tool` string exactly as recorded, the run-file path, the tally of recorded outcome values, and the source revision, which is the checkout revision the host observed for that run record. A value the source does not state, including the source revision when the host observed none, SHALL read `not recorded`, with no gap.
- The report SHALL NOT compute, normalise or choose a run outcome.
- The report SHALL show only the tally of the outcome values as recorded.
- A criterion with no selected run (no bound suite, no retained run holding an entry bound to it, or backing only by Inspection or Analysis records) SHALL show `not recorded` for the run values and receive only the gaps its evidence status already gives (`no_evidence`, `unbacked_criterion`).
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
| FR-196-AC-3 | A source finding with typed `failed-run` cause yields `evidence_failed`, names the criterion, shows that typed cause and the tally of recorded outcome values, never calls it passing and computes no outcome. Changing only the finding summary gives identical classification. | Test |
| FR-196-AC-4 | A `stale` criterion with a missing-run cause yields `evidence_stale`; a `suspect` criterion yields `evidence_suspect`; neither yields `evidence_failed`. | Test |
| FR-196-AC-5 | `undischarged` and `no run evidence` each yield `no_evidence`, and a criterion without a binder also yields `unbacked_criterion`. | Test |
| FR-196-AC-6 | A criterion backed only by retained Inspection evidence shows that record and receives no gap when the matrix says `bound`. | Test |
| FR-196-AC-7 | A requirement with no criteria remains listed with `no_acceptance_criteria` and no fabricated criterion row. | Test |
| FR-196-AC-8 | An empty matrix displays the source `reason`, produces document-level `no_requirements`, and is `incomplete`; two runs over the same inputs give byte-identical Markdown. | Test |
| FR-196-AC-9 | A criterion bound to one suite with a retained run shows that run's timestamp, `tool` string and run-file path exactly as recorded, a tally of the recorded outcome values of the entries bound to the criterion, and the source revision as `not recorded` when the record carries none, with no gap for that absence; a run whose entries disagree with the matrix status shows both unchanged and no computed outcome. | Test |
| FR-196-AC-10 | A selected run for which the host observed a checkout revision shows that revision verbatim as its source revision; a selected run with no observed revision shows `not recorded` and produces no gap. | Test |
| FR-196-AC-11 | A criterion with no retained run shows `not recorded` for the run values and gets `no_evidence` and no other gap for the missing run; a criterion backed only by retained Inspection evidence shows `not recorded` for the run values and gets no gap. | Test |
| FR-196-AC-12 | With two retained runs of one suite that hold an entry for the criterion, the report shows the run with the greater `timestamp`, the file name breaking a tie; with two bound suites it shows one run identity per suite. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md), and the typed source cause
  in PLAT-1760 (which blocks PLAT-1617).
- **Downstream**: PLAT-1617 implements this document.
