---
id: FR-061
title: "First-party GitHub Actions release operational producer"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-016"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-059"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-060"
    type: "requires"
---

# FR-061: First-party GitHub Actions release operational producer

## Description

When given a retained GitHub Actions release workflow and retained API exports from
one of its real runs, Quoin SHALL provide a first-party producer that derives one
standing-capability record and one exercise record and submits both through the
FR-060 intake boundary.

## Inputs

- The exact version-controlled GitHub Actions workflow YAML used by the run.
- Exact GitHub workflow-run and workflow-jobs JSON responses retained from the
  GitHub Actions API after a separately executed release run.
- A versioned producer definition naming the workflow path, release job, control
  identity, deployed subject and scope, supported transition, authorized roles,
  coverage, limitations, owner, gaps, actions, accepted event, and clock deadline.
- Immutable Quoin version, producer source revision, definition version, and
  environment identity.

## Outputs

- One `release` standing-capability record and one linked `release` exercise
  record accepted through FR-060, or an actionable refusal with no partial pair.
- Raw-evidence references to the unmodified workflow and API responses with
  computed media type, byte size, and content digest.

## Behavior

- The producer SHALL parse the workflow structurally and require the configured
  release job and accepted manual-dispatch event to exist.
- The producer SHALL require the workflow-run export to identify the configured
  workflow path, accepted event, immutable source revision, and completed run.
- The producer SHALL refuse the input unless the jobs export contains exactly one
  started and completed instance of the configured release job.
- The producer SHALL derive capability availability and surface from the workflow.
- The producer SHALL derive exercise actor, trigger, source revision, start/completion times,
  outcome, before/after state, observations, and clock state from the retained API
  responses rather than caller-supplied observations.
- The producer SHALL map a successful release job to `succeeded`.
- The producer SHALL retain every GitHub failure, cancellation, skip, or other non-success conclusion as an
  explicit non-success exercise that cannot discharge the obligation.
- The producer SHALL link both records by control id, subject, scope, definition,
  and capability-record id.
- The producer SHALL submit both records as one pair so neither record is retained
  if validation or persistence of the other fails.
- The producer SHALL compute raw-evidence metadata from the exact input bytes.
- The producer SHALL NOT accept caller-supplied digests, byte sizes, timestamps, conclusions, or
  clock status.
- The producer SHALL NOT contact GitHub, dispatch a workflow, publish a release,
  invoke a process, or alter any operational control. It consumes already-retained
  evidence and submits records through FR-060.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-061-AC-1 | Quoin's retained release workflow and a real completed workflow-run/jobs export produce one linked standing-capability and exercise pair whose workflow, revision, actor, timing, outcome, and observations match the source artifacts. | Test (TC-1244) |
| FR-061-AC-2 | Missing or malformed YAML/JSON, a workflow/path/event/revision mismatch, or an absent, duplicate, unstarted, or incomplete release job is refused without either operational record. | Test (TC-1245) |
| FR-061-AC-3 | Capability state is derived from the parsed workflow and exercise state and clock status are derived from the retained API exports; caller-supplied replacements cannot change them. | Test (TC-1246) |
| FR-061-AC-4 | Every non-success GitHub conclusion remains a named non-success exercise and cannot discharge the clocked release obligation. | Test (TC-1247) |
| FR-061-AC-5 | A real release-run integration captures workflow-run and jobs responses outside Quoin, then the Quoin producer consumes the retained files without network or process execution and atomically persists the pair through FR-060. | Test (TC-1248) |

## Constraints

- Real-run fixtures and transcripts SHALL satisfy the repository's content-rights
  policy; constructed API responses SHALL NOT be labelled as real producer evidence.
- The producer is a retained-evidence adapter, not a GitHub client or release runner.
- The initial producer supports GitHub Actions release workflows only; FR-059 and
  FR-060 remain engine-independent for other deployment surfaces and producers.

## Dependencies

- [FR-059](./FR-059-operational-evidence-records.md) defines both output shapes and
  their linkage invariants.
- [FR-060](./FR-060-operational-evidence-intake-report.md) owns validation,
  all-or-nothing persistence, clocked discharge, and reporting.
