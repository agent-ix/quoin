---
title: Capture release evidence
type: how-to
description: Build a release snapshot and export retained test evidence for review.
---

# Capture release evidence

From the project root, combine the release inputs into a snapshot and export a retained test run as an in-toto statement.

## Before you start

Use Quoin 0.38.2. Prepare the release-history file and clause-binding report. Make sure `spec/soup/` exists, then select the full baseline commit id and an explicit evaluation time.

Follow these steps:

1. Write the release snapshot.
2. Export a retained test result.
3. Review both files.

## Write the snapshot

```sh
quoin release snapshot \
  --repo . \
  --baseline-commit 0123456789abcdef0123456789abcdef01234567 \
  --history build/release-history.json \
  --binding binding.json \
  --as-of 2026-11-25T00:00:00Z \
  --out build/release-snapshot.json
```

The snapshot records the supplied inputs and the evidence state Quoin reads. The command runs no Git operations and refuses an existing output path.

## Export a test result

```sh
quoin evidence export-test-result \
  --evidence-dir spec/evidence \
  --subject build/subjects.json \
  --execution-provenance build/execution-provenance.json \
  --output build/test-result.intoto.json
```

The subject file names the released files and their SHA-256 digests. The provenance file identifies the recorded test execution.

## Result

The snapshot is written to the requested path. The export is an in-toto Test Result statement linked to retained evidence and the supplied subjects.
