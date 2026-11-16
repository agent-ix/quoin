---
id: SR-20261115210125
title: "gap-analysis review of PLAT-1617 verification report"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; rust/crates/quoin-dossier, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-cli/src/evidence/audit.rs, FR-196; PLAT-1617; PR #935"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-196"
    type: references
---

## Summary

Ticket: PLAT-1617. Reviewed origin/main...HEAD at a8e2d67b687e38e470f6d01be5d10b05b3647de2. Repository-wide mechanical reconciliation exposes a tooling/model limitation: the eight FR-196 criterion bindings are unresolved despite explicit source tags. No missing-test defect is attributed to the PR.

## Verdict

**FAIL** — The measured matrix marks every FR-196 criterion untagged. The cause remains an external model/engine or repository traceability integration limitation; no assertion is made that the explicit tests are absent.

## Assurance Context

AP-201 (spec/assurance/AP-201-finding-quality.md) applies to this evidence-report rendering change: concealed gaps have material impact. Reviewed source-status preservation, explicit missing evidence, binder identities, summary counts and deterministic output. AP-202 does not apply. Available context: FR-191, FR-196, NFR-040/041, direct source, generated reports and test results. Selected architecture, impact-analysis records, measurement runs, independent-adjudication records, structural-gap records and producer-reliance decisions: unavailable for this change. Stale selected context: none identified. Active exceptions: none identified. No measurement threshold or new policy was inferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix 0.35.0; quire matrix/coverage 0.45.0 (engine 0.60.0); native PR binary 0.35.0-8-ga8e2d67b. All commands exit 0 on the reviewed repository. Each matrix has 210 requirements and 2,109 criteria: 1,913 untagged, 141 method-without-symbol, 37 retired, 18 tagged; all 2,109 have no run evidence. The unfed evidence store is reported once, not as per-criterion findings. The whole-repository baseline deficit is not attributed to this PR. All eight changed FR-196 criteria have explicit test tags in source but no computed binders; the limitation is retained in Coverage rather than asserted as eight implementation defects. No clean traceability claim is made.

The coverage engine also reports a Rust census of 2,917 candidates, 2,373 tagged and 1,968 bound symbols. Ambient modules, explicit ISO/process modules, a fresh native build and an isolated source snapshot agree on the missing FR-196 matrix bindings. This unexpected semantic result is recorded in tooling-reproduction.md; it is not silently replaced by a fabricated clean matrix. Coverage diagnostics include existing section-holds-no-table and symbol-id-collision reports. Stale/untracked tags outside the diff remain baseline context and were not attributed to PLAT-1617.

Repository source inventory: 589 Rust source files, public functions and module requirement citations in source-inventory.json. Changed dossier behaviors map to FR-191/FR-196 and NFR-040/041. No new unexplained public behavior or production stub was identified in the changed surface. Reverse-gap inspection examined generator registration, sources, gaps, rendering, shared requirement readers, CLI dispatch and tests; this is not a semantic correctness claim for every unchanged repository function. Source/test stubs introduced by this PR: zero. Optional semantic gap-analysis pass: skipped (not requested).

## Reviewer data

```yaml
clean: true
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
findings: []
bindings: []
```
