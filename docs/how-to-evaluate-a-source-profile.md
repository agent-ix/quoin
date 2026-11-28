---
title: Evaluate a selected source profile
type: how-to
description: Evaluate the clause sets selected for a project before using their results.
---

# Evaluate a selected source profile

From a project root, evaluate the clause sets selected by the project's source profile and review each result before using it.

## Before you start

Use Quoin 0.38.2. Configure `.ix/source-profile.yaml` with the profile and context values for your project. Install the modules that provide the selected clause sets.

Follow these steps:

1. Select the profile and context for the project.
2. Evaluate its clause sets.
3. Review each result.

## Run the evaluation

```sh
mkdir -p build/profile-evaluation
quoin profile evaluate --out-dir build/profile-evaluation
```

Quoin evaluates each selected clause set and writes one engine report per set in the output directory.

## Review the results

Open every report and check its binding, not-binding, and unresolved clauses. A missing input or unresolved clause remains a finding; it is not treated as a binding result.

## Result

The output directory contains one report for each selected clause set. Quoin writes no profile selection or context values.
