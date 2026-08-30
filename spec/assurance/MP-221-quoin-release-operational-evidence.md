---
id: MP-221
title: Quoin release operational evidence
type: MeasurementPlan
status: active
owner: engineering assurance
stage: gate
metric: release_operational_evidence
definition_version: github-actions.release-operational-v1
relationships: []
---

# Quoin release operational evidence

## Decision Use

Decide whether Quoin has both a standing manual release capability and a real
successful exercise of that capability for the v0.22.5 npm release.

## Population

The version-controlled Quoin `release.yml` workflow and GitHub Actions run
33280266874, manually dispatched against the v0.22.5 source revision.

## Measure Definition

Retain one standing-capability record for the release workflow and one linked
actual exercise record for its unique Publish job. Discharge requires the exact
subject and scope, actual mode, successful outcome, and completion within the
declared ten-minute operational clock.

## Collection Procedure

Retrieve the workflow-run and jobs REST responses outside Quoin and retain their
exact payload bytes beside the workflow bytes from the run's immutable source
revision. Quoin only parses those retained artifacts and never contacts GitHub,
dispatches a workflow, or publishes a package.

## Environment and Sampling

The observed environment is the public `agent-ix/quoin` GitHub Actions
repository. This is one successful release exercise, not a reliability sample
or an estimate of future release success.

## Interpretation

The pair demonstrates that the configured manual release surface existed and
that its Publish job succeeded for v0.22.5 within the declared clock. It does not
establish release availability for other revisions or ecosystems.

## Comparison and Enforcement

Accept only the configured workflow path, `workflow_dispatch` event, immutable
source revision, completed run, and exactly one started and completed Publish
job. Preserve non-success conclusions as non-discharging evidence.
