---
id: FR-199
title: "The dossier risk management document renders the recorded hazard-to-decision chain"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-169"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-170"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-145"
    type: "references"
---

# FR-199: The dossier risk management document renders the recorded hazard-to-decision chain

## Description

When `quoin dossier generate` selects `risk-management` (document id
`DOS-RISK`), the risk management document SHALL render the rows and findings
of `quoin risk-summary` without assigning a new risk value or decision.

## Inputs

- `quoin risk-summary <scope> --format json`, produced by
  `quoin_risk_advice::summarize` in
  `rust/crates/quoin-risk-advice/src/lib.rs`
  ([FR-169](./FR-169-risk-summary-command.md),
  [FR-170](./FR-170-risk-summary-row-findings.md)).
- `quoin_risk_advice::advise` from the same crate supplies policy-cell causes
  and the undecided decision queue. The common dossier `--scope` and
  `--module` select the same safety corpus for both source operations. The
  generator does not repeat region lookup or chain-completeness rules.
- The `risk-summary` JSON as FR-169 defines it, including its `hazard_title`,
  `hazard_text`, `situation_title`, `situation_text` and `harm_texts` row keys,
  the `rationale` key of each residual-risk entry, and its `benefit_risks` and
  `overall_evaluations` lists. The generator reads these authored values from
  that source and from no other, and never from the record files.

## Outputs

- `dossier/DOS-RISK.md`: `# Risk management document`, one
  `## <hazard id>` section per source hazard in source order,
  `## Benefit-risk records`, `## Overall residual risk evaluation`,
  `## Undecided decisions` from the source queue, then `## Gaps`.
- Each hazard section shows the hazard's authored title and `## Condition` text under its id. Under each situation, the situation's authored title and `## Description` text and each harm's id, title and `## Description` text appear under their ids; tables show `Phase | Severity | Probability | Region`
  and `Control | Implementing requirements | Retired requirements | Verifications`, followed by
  each residual-risk record's decision, decider and authored rationale, and row findings.
- `## Benefit-risk records` has one entry per source `benefit_risks` entry, in
  dossier id order, showing its id, `benefit`, decision, decider, rationale and
  referenced residual risks, and the line `none` when the list is empty.
- `## Overall residual risk evaluation` has one entry per source
  `overall_evaluations` entry, in dossier id order, showing its id, `product`,
  decision, decider, rationale and referenced records, and the line `none`
  when the list is empty.

## Behavior

- The document SHALL render every source row's hazard, situation, harms,
  pre-control and post-control estimates, controls, implementing requirements,
  verifications and residual risks in source order.
- Each estimate's region SHALL equal the source `policy_region`; an absent
  region SHALL read `not stated` and show any matching source
  `Advice.policy_cells.findings` by estimate id, not a guessed cell.
- The document SHALL render the authored title and text of each hazard, hazardous situation and harm, and each residual-risk record's authored rationale, verbatim under the record's id.
- An absent authored title, text or rationale SHALL read `not stated`.
- The document SHALL NOT invent, summarise or derive a title or text.
- The document SHALL render every `benefit_risks` entry, decided or not, and every `overall_evaluations` entry, decided or not, with the values named under Outputs; an absent field reads `not stated`.
- When the corpus holds no `overall_residual_risk_evaluation` record, the document SHALL produce one document-level `no_overall_evaluation` gap.
- When an `overall_evaluations` entry has decision `undecided`, the document SHALL produce one `overall_evaluation_undecided` gap naming that evaluation, so an open overall decision never reads as complete.
- The document SHALL add no acceptance state or label of its own to any residual-risk, benefit-risk or overall record: authored decision values and texts are shown verbatim, and the document assigns no score, class or acceptability value.
- The document SHALL keep each FR-170 finding against its source row.
- For a `control-without-verification` finding, the document SHALL produce
  one `control_unverified` gap naming the control.
- For a `residual-risk-undecided` finding, the document SHALL produce one
  `residual_risk_undecided` gap naming the risk.
- For every other row finding, the document SHALL produce one
  `risk_chain_incomplete` gap naming its row and finding.
- When source policy-cell findings report `policy-not-found`, the document SHALL
  produce one document-level `no_acceptability_policy` gap.
- The `Undecided decisions` section SHALL show the source
  `Advice.decision_queue` entries for residual and benefit-risk records
  without adding a decision; a decided record is not in the queue, and an undecided benefit-risk record appears both in its own section and in the queue.
- The document SHALL NOT claim to contain estimates or records that neither
  source report exposes.
- If a source reader reports an unreadable safety record, the command SHALL
  surface a source failure under FR-191 instead of treating the omitted record
  as an empty chain.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-199-AC-1 | A fixture with a complete hazard, situation, harm, pre/post estimates, control, requirement, verification, decided residual risk and decided overall residual risk evaluation renders each source value and no gap, and status `complete`; the hazard heading uses its id and the authored title and texts are shown verbatim, not invented. | Test |
| FR-199-AC-2 | Removing a control verification yields `control_unverified` naming that control; no source row is lost. | Test |
| FR-199-AC-3 | Removing a residual-risk decider yields `residual_risk_undecided`, a `not stated` decider cell and no invented person or accepted decision. | Test |
| FR-199-AC-4 | A hazard with no situation remains listed with its FR-170 finding and `risk_chain_incomplete` gap; a `control-without-verification` or `residual-risk-undecided` finding yields only its dedicated gap kind. | Test |
| FR-199-AC-5 | Every rendered policy region equals the value of `quoin risk advise --json` for the same estimate; an unplaced estimate gets no region. | Test |
| FR-199-AC-6 | A benefit-risk entry from `Advice.decision_queue` appears under `Undecided decisions`; an absent policy reported by `policy-not-found` yields `no_acceptability_policy`, while no unexposed record is invented. | Test |
| FR-199-AC-7 | Two runs over the same source data produce byte-identical Markdown, and no generated field computes a score, rank or acceptability. | Test |
| FR-199-AC-8 | An unreadable safety record causes exit 3 with the source error and leaves no selected output file. | Test |
| FR-199-AC-9 | A hazard, hazardous situation and harm that each carry an authored title and text render them verbatim under their ids, from the FR-169 `hazard_title`, `hazard_text`, `situation_title`, `situation_text` and `harm_texts` values; one with none reads `not stated` and no title or text appears that the source does not hold. | Test |
| FR-199-AC-10 | A decided residual risk renders its authored rationale and decider; one with no rationale reads `not stated` for it. | Test |
| FR-199-AC-11 | A decided `benefit_risk` record renders in `Benefit-risk records` with its id, `benefit`, authored decision, decider, rationale and referenced residual risks, and does not appear in `Undecided decisions`; an undecided one appears in both. | Test |
| FR-199-AC-12 | A corpus with a decided `overall_residual_risk_evaluation` record renders its id, `product`, authored decision, decider, rationale and referenced records, and produces neither `no_overall_evaluation` nor `overall_evaluation_undecided`. | Test |
| FR-199-AC-13 | A corpus with no `overall_residual_risk_evaluation` record produces exactly one document-level `no_overall_evaluation` gap, status `incomplete`, and the gap appears in the JSON report in gap order. | Test |
| FR-199-AC-14 | Changing only a residual-risk, benefit-risk or overall record's authored decision from `undecided` to `accepted` changes that record's decision and decider cells and the gap or queue entries that decision governs, and nothing else; no heading, column label or gap detail that the tool writes names acceptance, approval or acceptability, and no score, class or acceptability value appears. | Test |
| FR-199-AC-15 | An overall evaluation with decision `undecided` renders with that decision and produces one `overall_evaluation_undecided` gap naming it, status `incomplete`, and no `no_overall_evaluation` gap. | Test |
| FR-199-AC-16 | With two benefit-risk records and two overall evaluations in scope, each section lists them in dossier id order; with none of a kind, its section reads `none`. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-169](./FR-169-risk-summary-command.md),
  [FR-170](./FR-170-risk-summary-row-findings.md), and
  [FR-145](./FR-145-risk-advise-command.md) for policy-cell values and
  the undecided decision queue.
- **Downstream**: PLAT-1622 implements this document.
