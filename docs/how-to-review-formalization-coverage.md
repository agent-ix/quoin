---
title: Review formalization coverage
type: how-to
description: Check which requirements and criteria have approved formalization records.
---

# Review formalization coverage

From a project root, inspect approval state and coverage for formalized declarations.

## Before you start

Use Quoin 0.38.2. Provide the verifier and its allowed-signers file when the report reads signed approval state.

Follow these steps:

1. Check the declaration approval state.
2. Check formalization coverage.
3. Review missing and unapproved rows.

## Check declaration state

```sh
quoin formal-rules status \
  --repo . \
  --engineering-assurance /absolute/path/to/engineering-assurance \
  --allowed-signers /absolute/path/to/allowed_signers
```

The report identifies declarations and the approval state derived from their signed commits.

## Check requirement coverage

```sh
quoin formal-rules coverage \
  --repo . \
  --engineering-assurance /absolute/path/to/engineering-assurance \
  --allowed-signers /absolute/path/to/allowed_signers \
  --json
```

Review the requirement and criterion rows alongside their approval state. Formalization is reported as evidence and does not replace test or verification evidence.

## Result

The command emits one coverage report for the selected repository. Missing or unapproved declarations remain visible in the report.
