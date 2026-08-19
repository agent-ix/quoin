---
id: NFR-011
title: "Bundle-scale commands stay within a stated time budget"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-037"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "constrains"
---

# NFR-011: Bundle-scale commands stay within a stated time budget

## Statement

Every `quoin` command that walks a whole spec bundle SHALL complete within **5
seconds** on a bundle of 250 documents on ordinary developer hardware, and the
figure SHALL come from a measurement rather than an estimate.

## Scope

- Applies to: `quoin completeness`, `quoin advise`, `quoin evidence audit`,
  `quoin matrix` — the commands whose cost grows with bundle size.
- Operational context: a CI gate and an interactive terminal run.
- Excluded: `quire` subprocess time, which the engine's own NFRs govern.

## Rationale

quoin has **no performance measurement of any kind** — no benchmark, no timing
assertion, no threshold anywhere in the repository. That was defensible while it
was a scaffolding tool run by hand. It is not now: `quoin evidence audit` and
`quoin completeness` are merge gates, and a gate people wait on is a gate people
route around.

The number is a **budget, not a prediction**. Its purpose is that a change making
a bundle walk an order of magnitude slower fails a test instead of being absorbed
as "CI got slower". 5s over 250 documents is roughly twice the current cost of the
most expensive command, which leaves headroom without licensing a regression.

Stating it also makes the cost model explicit: these commands are **O(documents)**
with a single pass, and any change that makes one O(documents × obligations) is a
design change, not a tuning question.

## Measurement and Evaluation

| Metric                                                        | Target | Threshold | Method |
| ------------------------------------------------------------- | ------ | --------- | ------ |
| Wall-clock for a bundle walk over 250 documents               | < 2s   | < 5s      | Test   |
| Full passes over the document set per command invocation      | 1      | 1         | Test   |

## Verification

A generated bundle of 250 documents is walked by each bundle-scale command, and
the elapsed time asserted against the threshold. The pass count is asserted by
counting reads rather than inferred from the timing, so a regression from one
pass to two is caught even on hardware fast enough to stay inside the budget.

**Not met at the time of writing** — no such measurement exists. Stated ahead of
the implementation; the gap is `agent-ix/quoin#133`.

## Dependencies

- **Upstream**: [FR-037](../functional/FR-037-declared-vocabulary-completeness.md),
  [FR-032](../functional/FR-032-evidence-auditor.md) — the bundle-scale commands.
- **Downstream**: [NFR-012](./NFR-012-ecosystem-compatibility.md), whose corpus-wide
  sweeps multiply this cost by the repository count.
