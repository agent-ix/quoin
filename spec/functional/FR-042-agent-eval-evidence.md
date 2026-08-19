---
id: FR-042
title: "Agent-eval reports as run evidence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
---

# FR-042: Agent-eval reports as run evidence

## Description

An eval drives the **real agent through the real CLI**, which makes it the most convincing
verification this project has — and, until this, the least recorded.

`quire coverage` reconciles matrix rows against test symbols in code. Eval scenarios are data in
`evals/scenarios/index.mjs`; they mint no symbol and can never back a row. Measured in `SR-008`:
**71 unbacked rows** across `spec/evals.md`, `FR-028` and `FR-038`, every one a criterion whose ✅
rested on somebody having run the evals.

For `FR-038` they had been run — four scenarios, 4/4, live, with two genuine failures found and fixed
along the way. The harness wrote `evals/reports/latest.json` and the repository dropped it.

`quoin` SHALL read a `cli-agent-evals` report through the FR-033 registry, so an eval suite becomes a
run record subject to the same freshness, staleness and vacuity checks as any other.

### The harness's verdict, not a second opinion

A scenario passing one run of three is not a pass. `ok` is the suite's own answer over `repeats`, and
recomputing it here could disagree with the report a human already read.

`passRate` is kept as a score because **"flaky" and "failing" are different facts** and `outcome`
alone cannot carry the difference. It is not a mutation score — `agent-ix/quoin#138` is why `score`
needs a metric discriminator before any threshold reads one.

### An empty report is refused

A report with no scenario results is a suite that ran nothing. Recording it would manufacture
evidence from a file proving only that the harness started.

### What this does not yet close

**The record binds no obligation, and that is stated rather than hidden.**

The store binds a run entry by matching its trace ids against **obligation ids**. A junit test carries
`FR-001-AC-1` in its own name, so that works. An eval report carries the **scenario** id — `TC-EV-054`
— which is a Test Case row, and the join from `FR-038-AC-1` to `TC-EV-054` lives in the FR's own
Acceptance Criteria table, which the store does not read.

So this closes the first half of `SR-008` FND-001: the run is now **recorded** where before it was
discarded. The second half — binding it to the obligations the matrix says it discharges — needs
either the harness to report which criteria each scenario covers, or the store to resolve a Test Case
id through coverage's declared targets. Filed as `agent-ix/quoin#144`.

`SR-008` FND-002 — that `Eval` is not a declared verification method at all — is module data in
`spec-artifacts-process` and remains open under `agent-ix/quoin#142`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-042-AC-1 | A real report yields one entry per scenario, keyed on the scenario id. | Test (TC-240) |
| FR-042-AC-2 | The scenario id is its own trace id; no second identity is invented. | Test (TC-241) |
| FR-042-AC-3 | The outcome is the harness's `ok`, and `passRate` is kept so flaky is distinguishable from failing. | Test (TC-242) |
| FR-042-AC-4 | A report with no scenario results is refused. | Test (TC-243) |
| FR-042-AC-5 | The adapter is selected by `--adapter agent-eval` and by the harness name. | Test (TC-244) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-042-CON-1 | quoin SHALL NOT run the eval suite. The consumer's CI does (ADR-0011 invariant 1). | Design | Inspection |
| FR-042-CON-2 | quoin SHALL NOT recompute a scenario's verdict from its runs. The harness states it. | Design | Test (TC-242) |
| FR-042-CON-3 | quoin SHALL NOT claim a binding it cannot make. An unmatched trace id is reported, not assumed. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the registry), [FR-030](./FR-030-evidence-store.md) (where the run lands)
- **Downstream**: `agent-ix/quoin#144` (binding a scenario to the criteria it discharges), `agent-ix/quoin#142` (a catalog entry for `Eval`)
