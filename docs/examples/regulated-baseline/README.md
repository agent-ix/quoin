---
type: how-to
description: Run the baseline tutorial with synthetic records and a throwaway signing key.
---

# Run the baseline tutorial with test data

Use this example to follow [the regulated-baseline tutorial](../../regulated-baseline.md). It includes a fictional desk sensor, synthetic evidence, a software bill of materials, a change-assurance record, and an authored review decision.

The sample test result is fabricated. Recording it demonstrates how Quoin imports a result file; it does not show that a test ran. The records do not define a real product, safety policy, or release decision.

Follow these steps:

1. [Set up the temporary project](#1-set-up-the-temporary-project).
2. [Review the authored records](#2-review-the-authored-records).
3. [Record the sample evidence and inventory](#3-record-the-sample-evidence-and-inventory).
4. [Generate release history](#4-generate-release-history).
5. [Prepare a release review](#5-prepare-a-release-review).

## 1. Set up the temporary project

Run these commands from the Quoin repository root. They copy the sample, create an isolated module store, and configure a disposable signing key.

Install Quoin, Quire, and Engineering Assurance before running the example. Engineering Assurance must be available as `engineering-assurance` on `PATH` for the release-history step.

```sh
EXAMPLE_HOME=$(mktemp -d)
cp -R docs/examples/regulated-baseline/project "$EXAMPLE_HOME/project"
cd "$EXAMPLE_HOME/project"
export IX_HOME="$EXAMPLE_HOME/ix"

git init
git config user.name "Example Reviewer"
git config user.email "example-reviewer@example.invalid"
ssh-keygen -t ed25519 -N '' -C example-reviewer -f "$EXAMPLE_HOME/test-signing-key"
printf 'example-reviewer %s\n' "$(cat "$EXAMPLE_HOME/test-signing-key.pub")" > "$EXAMPLE_HOME/allowed_signers"
git config gpg.format ssh
git config user.signingkey "$EXAMPLE_HOME/test-signing-key"
git config gpg.ssh.allowedSignersFile "$EXAMPLE_HOME/allowed_signers"

quoin module ensure-defaults
git add .gitattributes binding.json change-assurance-record.json release-history-request.template.json sample-evidence-index.json sample-sbom.json sample-test-results.xml spec sufficiency-decisions.json
git commit -S -m "Create synthetic baseline example"
```

The allowed-signers file trusts only the throwaway public key. Remove `EXAMPLE_HOME` when you finish. Never use a production key for this exercise.

## 2. Review the authored records

Validate the product records with Quire. The explicit module paths make it use the module store created above.

```sh
QUIRE_MODULES="$IX_HOME/filament/modules"
quire validate --scope "$PWD" \
  --module "$QUIRE_MODULES/spec-artifacts-process" \
  --module "$QUIRE_MODULES/engineering-assurance" \
  --module "$QUIRE_MODULES/spec-artifacts-iso" \
  'spec/functional/**/*.md' \
  'spec/validation/**/*.md' \
  'spec/anomalies/**/*.md' \
  'spec/baselines/**/*.md'
```

Inspect the synthetic argument with its authored decision and evidence index:

```sh
quoin assurance \
  --repo . \
  --argument AA-001 \
  --decisions sufficiency-decisions.json \
  --evidence sample-evidence-index.json \
  --as-of 2026-11-25T00:00:00Z
```

The command reports the sample claim as supported by the indexed result. That output describes the supplied example data.

## 3. Record the sample evidence and inventory

The evidence command transcribes the included JUnit XML. It does not run a test.

```sh
quoin evidence record \
  --repo . \
  --suite example-checks \
  --tool sample-runner \
  --adapter junit \
  --results sample-test-results.xml \
  --timestamp 2026-11-25T00:00:00Z

quoin soup promote sample-sbom.json
quoin matrix --repo .
quoin evidence audit --repo .
quoin soup report --binding binding.json --as-of 2026-11-25T00:00:00Z
quoin anomaly check spec
quoin anomaly report spec
```

The anomaly report leaves `AN-001` unassessed. The output does not make a safety decision.

## 4. Generate release history

Copy the sample change-assurance record into the retained record tree and commit it. The history command lists records added since the previous release; this first sample has no previous release tag.

```sh
mkdir -p spec/change-assurance/records/CA-001
cp change-assurance-record.json spec/change-assurance/records/CA-001/1.json
git add spec/change-assurance/records
git commit -S -m "Add synthetic change-assurance record"
git rev-parse HEAD
```

Copy `release-history-request.template.json` to `release-history-request.json`. Set `repository` to the absolute path printed by `pwd`, and set `baseline_commit` to the full hash printed by `git rev-parse HEAD`. Then run the request through stdin:

```sh
engineering-assurance release-history --schema request
engineering-assurance release-history --request - < release-history-request.json
```

The result names the synthetic record at `spec/change-assurance/records/CA-001/1.json`.

## 5. Prepare a release review

The following commands print the request schemas for signed approval tags and release verification:

```sh
engineering-assurance approval-tag --schema request
engineering-assurance release-verify --schema request
```

Use those schemas with the reviewed sample record, baseline commit, generated history, and temporary allowed-signers file when following the release steps in the tutorial. A person must review and authorize each signed approval. The example key is only for this local exercise.

## Expected output

Quoin records one synthetic passing result and one software inventory item. The anomaly report marks `AN-001` unassessed. The release-history output names `CA-001`, revision 1, at its committed path.
