---
title: Review clause discharge and mapping reports
type: how-to
description: Check recorded clause facts and render an inward or requirements-mapping report.
---

# Review clause discharge and mapping reports

From a project root, evaluate a clause binding against recorded facts and inspect the remaining work.

## Before you start

Use Quoin 0.38.2. Prepare a binding report and record facts for the clauses in that report. The [synthetic baseline example](examples/regulated-baseline/README.md) includes a binding report you can use to try the commands.

Follow these steps:

1. Report the clause partition.
2. Check inward completeness.
3. Render the mapping matrix.

## Report the clause partition

```sh
quoin discharge \
  --repo . \
  --binding binding.json \
  --as-of 2026-11-25T00:00:00Z \
  --json
```

The result separates direct facts, dispositions, open clauses, and unresolved applicability.

## Check inward completeness

```sh
quoin discharge \
  --repo . \
  --binding binding.json \
  --as-of 2026-11-25T00:00:00Z \
  --inward --json
```

The inward report counts the binding, discharged, disposition, unresolved, and open items.

## Render the mapping matrix

```sh
quoin discharge \
  --repo . \
  --binding binding.json \
  --as-of 2026-11-25T00:00:00Z \
  --rmm --json
```

The matrix reports each clause separately. Review applicability, compliance, evidence, and tailoring before using the report in a release decision.

## Result

Each command reports the facts available at the selected instant. Missing or unresolved facts remain open in the output.
