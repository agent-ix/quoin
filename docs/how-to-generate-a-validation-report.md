---
type: how-to
description: Combine retained tool test results into a validation report for review.
---

# Generate a tool validation report

From a project root, combine retained test results into Markdown and JSON files for review.

## Before you start

Install Quoin 0.37.0. Collect the JSON run records produced by the normal test suites for the tools in scope. Keep each record's tool version, environment, test results, validation inputs, and anomaly set.

The examples use two sample records in `docs/examples/validation-report/`. Replace those paths with your retained run records.

## Steps

1. **Generate the report.** Pass each run record with its own `--results` option. Choose an output directory inside your project so Git can record the report later.

   ```sh
   quoin validation report \
     --results docs/examples/validation-report/quoin-run.json \
     --results docs/examples/validation-report/quire-cli-run.json \
     --out build/validation-report
   ```

   The command writes `validation-report.md` and `validation-report.json`. A complete report exits with code 0. An incomplete report still writes both files and exits with code 1.

2. **Review the result.** Read the report status, tool versions, test results, coverage rows, and known anomalies. The JSON file contains the same report data for automated review.

   `complete` means the required run evidence is present and its high-process-risk features have passing cases. The status does not decide whether a tool is acceptable for a customer's use.

3. **Record a person's sign-off.** Commit the report with an SSH signature and the required meaning trailer:

   ```sh
   git add build/validation-report/validation-report.md build/validation-report/validation-report.json
   git commit -S \
     -m "Validation report" \
     -m "Signed-off-meaning: tool-validation-accepted"
   ```

   Verify the recorded signature against your allowed signers file:

   ```sh
   quoin validation report \
     --verify-signoff \
     --repo . \
     --out build/validation-report \
     --allowed-signers /path/to/allowed_signers
   ```

   The verification result reports signature validity, signer mapping, principal match, and trailer presence. A person decides whether to accept the report.

## Expected output

The sample report command prints:

```text
validation report written to build/validation-report
```
