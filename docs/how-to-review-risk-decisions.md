---
title: Review recorded risk decisions
type: how-to
description: Compare risk estimates and decisions with the project's authored policy and evidence.
---

# Review recorded risk decisions

From a project root, inspect the recorded risk chain and compare its decisions with the authored policy.

## Before you start

Use Quoin 0.38.2. The selected project scope must contain its authored risk records, policy, requirements, and anomaly documents.

Follow these steps:

1. Read the hazard-to-decision chain.
2. Review the policy advice and discrepancies.

## Read the decision chain

```sh
quoin risk-summary .
```

Each row connects a hazard and situation to its estimate, control, evidence, and decision. Use `--format json` when another tool will consume the report.

## Review advice and disagreements

```sh
quoin risk advise .
quoin risk discrepancies .
```

Advice reports policy cells, evidence gaps, and undecided items. The discrepancy report names decisions that disagree with the authored policy. It does not change records or decide whether a risk is acceptable.

To use the discrepancy report as a gate, add `--strict`. Quoin exits nonzero when it reports one or more findings.

## Result

The reports show authored estimates, findings, and decisions. They do not change the risk records.
