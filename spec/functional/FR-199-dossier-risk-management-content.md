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
  and records in no chain. The generator uses the shared library operations;
  it does not repeat region lookup or chain-completeness rules.

## Outputs

- `dossier/DOS-RISK.md`: `# Risk management document`, one
  `## <hazard id>: <title>` section per source hazard in source order,
  `## Records in no chain`, `## Benefit-risk`, then `## Gaps`.
- Under each situation, tables show `Phase | Severity | Probability | Region`
  and `Control | Implementing requirements | Verifications`, followed by
  residual-risk decision, decider and row findings.

## Behavior

- The document SHALL render every source row's hazard, situation, harms,
  pre-control and post-control estimates, controls, implementing requirements,
  verifications and residual risks in source order.
- Each estimate's region SHALL equal the source `policy_region`; an absent
  region SHALL read `not stated` with the source reason, not a guessed cell.
- The document SHALL keep each FR-170 finding against its source row and
  produce a `risk_chain_incomplete` gap naming that row and finding.
- A residual risk with no recorded decision or decider SHALL produce
  `residual_risk_undecided` and SHALL show `not stated` for the missing field.
- A control without a source verification SHALL produce
  `control_unverified` naming the control.
- An absent acceptability policy SHALL produce `no_acceptability_policy`.
- The `Records in no chain` section SHALL show source estimates and residual
  risks that belong to no summary row; it SHALL NOT silently omit them.
- The document SHALL copy benefit-risk records as authored without calculating
  a score, rank, acceptability or residual-risk decision.
- If a source reader reports an unreadable safety record, the command SHALL
  surface a source failure under FR-191 instead of treating the omitted record
  as an empty chain.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-199-AC-1 | A fixture with a complete hazard, situation, harm, pre/post estimates, control, requirement, verification and decided residual risk renders each source value and no gap. | Test |
| FR-199-AC-2 | Removing a control verification yields `control_unverified` naming that control; no source row is lost. | Test |
| FR-199-AC-3 | Removing a residual-risk decider yields `residual_risk_undecided`, a `not stated` decider cell and no invented person or accepted decision. | Test |
| FR-199-AC-4 | A hazard with no situation remains listed with its FR-170 finding and `risk_chain_incomplete` gap. | Test |
| FR-199-AC-5 | Every rendered policy region equals the value of `quoin risk advise --json` for the same estimate; an unplaced estimate gets no region. | Test |
| FR-199-AC-6 | An estimate outside every chain appears under `Records in no chain`, and an absent policy yields `no_acceptability_policy`. | Test |
| FR-199-AC-7 | Two runs over the same source data produce byte-identical Markdown, and no generated field computes a score, rank or acceptability. | Test |
| FR-199-AC-8 | An unreadable safety record causes exit 3 with the source error and leaves no selected output file. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-169](./FR-169-risk-summary-command.md),
  [FR-170](./FR-170-risk-summary-row-findings.md), and
  [FR-145](./FR-145-risk-advice-command.md) for policy-cell values.
- **Downstream**: PLAT-1622 implements this document.
