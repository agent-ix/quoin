---
id: FR-114
title: "Generic measurement-campaign execution"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-114: Generic measurement-campaign execution

## Description

`quoin measurement campaign run` SHALL execute the named members of an
engineering-assurance `CampaignDefinition` through its bounded producer
executor, retain one `MeasurementCollection` for each successful producer
invocation, and retain an EA `CampaignRun` accounting for every attempted
invocation. The run SHALL decide the definition's `all-required` rule over the
independent checker outcome of every required member. The campaign verdict SHALL
be `accept`, `reject` or `inconclusive`.

The campaign machinery is generic. A project's producer adapter turns bounded
process output into a collection; a separate, member-declared checker procedure
may verify facts that Quoin's plan-verdict checker cannot infer from metric
rows. The checker runs through EA's bounded executor with retained producer
results and raw artifacts as read-only inputs. Neither Quoin's runner nor campaign
checker contains project names, milestone names or native tool result rules.

A member's producer and its checker run with the checkout of the member's
declared source repository as the EA capability root and working directory.
Before any request is created, `run` checks that each source
checkout is the repository root of the member's declared source repository and has no tracked edits or staged changes. The request records that repository and its raw `git ls-tree -r -z --full-tree` inventory as its
source-tree binding. Quoin computes and compares no hash. Measurement plans and procedures are read from the Git blobs of that tree. Selected inputs are written at their declared relative paths beneath the
checkout, and a path that is tracked in the source tree, or lies beneath a tracked link, is not one of them. The
producer can see and write any file in the checkout.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-114-AC-1 | A campaign definition and run are read through EA's generated contract types. Duplicate or missing members, an undeclared dependency, a dependency cycle, and an unknown completion rule refuse intake. | Test (TC-1940) |
| FR-114-AC-2 | Every producer and declared domain-checker invocation uses EA's bounded `ProducerExecutor` with the member's clean source repository checkout as the capability root. A structurally invalid request has its typed preflight refusal retained. Every terminal state, including unavailable, timed out, malformed and refused execution, appears as an attempt. A successful producer invocation retains a separate `MeasurementCollection`. | Test (TC-1941, TC-1942) |
| FR-114-AC-3 | The domain checker reads the producer request, result and raw artifacts as staged read-only inputs and emits a receipt naming the member, plan and definition version it checked. A receipt naming another member, plan or definition version is reject, and a checker that is absent, incomplete or emits a malformed receipt is inconclusive. | Test (TC-1947, TC-1983) |
| FR-114-AC-4 | The `all-required` result accepts exactly when every required member has a completed, independently accepted attempt and all dependencies accept. Rejected members reject the campaign; missing or unavailable evidence is inconclusive. Optional members remain visible and cannot mask a required failure. | Test (TC-1945) |
| FR-114-AC-5 | The CLI emits one versioned campaign-verdict document. It exits nonzero for reject or inconclusive without losing that document. A non-TL fixture exercises the path before any TL campaign is used. | Test (TC-1946) |

## Constraints

- Campaign and procedure contracts belong to engineering-assurance and are
  generated through FCD; Quoin does not redeclare their wire types.
- Producer process supervision belongs to EA. Quoin does not spawn a second
  producer runner if EA cannot provide the declared containment profile.
- A producer's own `pass` value is never sufficient for campaign acceptance.
- Quoin computes, records and compares no digest, hash, seal or key for campaign
  evidence. The digest fields that EA's generated campaign and executor types
  require are tracked as blocked on QSL-1275 and are never populated by Quoin.

## Dependencies

- Engineering-assurance supplies the FCD-generated campaign contracts, a
  procedure resolver and the bounded producer executor.
- FR-044 supplies write-once collections; FR-108 supplies independent
  per-plan measurement verdicts.
