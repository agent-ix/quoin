---
title: Attach and verify measurement evidence
type: how-to
description: Retain a producer file and check a plan against recorded collections.
---

# Attach and verify measurement evidence

From a project root, attach a producer file to the evidence store and check the stored collections against a measurement plan.

## Before you start

Use Quoin 0.38.2. Prepare a project with an active measurement plan and at least one recorded collection. Keep the source file under the repository root.

Follow these steps:

1. Attach the producer file.
2. Add its content reference to the measurement collection.
3. Check the plan verdict.

## Attach a file

Create or select the file to retain, then run:

```sh
mkdir -p build
printf 'sample result\n' > build/result.txt
quoin measurement attach --repo . --path build/result.txt --media-type text/plain
```

The command prints the byte length, media type, and SHA-256 reference. Add that reference to the collection's `attachments` list.

For the sample file above, the command reports `mediaType: text/plain` and a `sha256:` digest. The digest changes when the file bytes change.

## Check the plan

Run the independent checker for the plan id in your project:

```sh
quoin measurement verify --repo . --plan MP-001
```

The result reports the plan's calculated verdict and the reasons for any inconclusive or rejected result. When you supply `--claimed`, Quoin also checks that the claimed verdict matches its calculation.

## Result

A complete run reports the plan id, its verdict, and the supporting counts. An incomplete run names the reason and leaves the result inconclusive.

## Verify the attachment

Run the measurement checker again after recording the collection. It verifies the retained file against its content address. A changed or missing file makes the collection unusable.
