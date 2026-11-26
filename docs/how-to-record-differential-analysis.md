---
title: Record differential analysis results
type: how-to
---

Use this procedure to retain a report from `filament-analyze run-package` in a project's evidence store.

## Before you start

Install Quoin 0.37.0 and `filament-analyze`. Run the command from a Quoin module root with a `manifest.yaml` and a `spec/` directory. Run `filament-analyze run-package` with a checked package and save its report as `build/analysis-report.json` inside the project.

## Record the report

1. From the project root, record the report with a suite name, tool identity, and run kind:

```sh
quoin evidence record \
  --repo . \
  --module . \
  --suite SMT-ANALYSIS \
  --tool "filament-analyze 0.1.0" \
  --run-kind analysis \
  --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --results build/analysis-report.json \
  --json
```

Quoin validates the report and writes a run record under `spec/evidence/runs/`. It writes a separate projection under `spec/evidence/analysis-reports/`, paired with the run by suite and timestamp.

2. Use `--run-kind formal_rule` when the report judges selected formal claims. Prepare its matching `--formal-rule-selection` file from the same package selection and rule-status result. See [formal rule analysis](./formal-rule-analysis.md) for that workflow.

## Expected output

The command prints a JSON object with `kind: run` and the path of the recorded run. For example:

```json
{
  "kind": "run",
  "run_path": "./spec/evidence/runs/SMT-ANALYSIS/2026-01-01T00~3A00~3A00Z.json"
}
```
