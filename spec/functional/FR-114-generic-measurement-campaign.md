---
id: FR-114
title: "Generic measurement-campaign execution and verification"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-114: Generic measurement-campaign execution and verification

## Description

`quoin measurement campaign run` SHALL execute the named members of an
engineering-assurance `CampaignDefinition` through its bounded producer
executor, retain one `MeasurementCollection` for each successful producer
invocation, and retain an EA `CampaignRun` accounting for every attempted
invocation. `quoin measurement campaign verify` SHALL independently check the
complete member inventory, exact definition and source identities, retained
collection and checker evidence, and the definition's `all-required` rule. The
campaign verdict SHALL be `accept`, `reject` or `inconclusive` with the outcome
of every required member.

The campaign machinery is generic. A project's producer adapter turns bounded
process output into a collection; a separate, member-declared checker procedure
may verify facts that Quoin's plan-verdict checker cannot infer from metric
rows. The checker runs through EA's bounded executor with retained producer
results and raw artifacts as sealed inputs. Neither Quoin's runner nor campaign
checker contains project names, milestone names or native tool result rules.

A member's producer and its checker run with the checkout of the member's
declared source repository as the EA capability root and working directory.
Before any request is created, `run` and `verify` check that each source
checkout is at its pinned revision, has no tracked edits or staged changes, and
yields a raw `git ls-tree -r -z --full-tree` inventory whose SHA-256 equals the
source-graph digest. The request records that repository and inventory as its
source-tree binding, and EA checks every tracked file's bytes against it before
launch. Selected inputs are written at their declared relative paths beneath the
checkout, and a path that is tracked in the source tree is not one of them. The
producer can see and write any file in the checkout.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-114-AC-1 | A campaign definition and run are read through EA's generated contract types. Duplicate or missing members, an undeclared dependency, a dependency cycle, a mismatched definition or source-graph digest, and an unknown completion rule refuse intake. | Test (TC-1940) |
| FR-114-AC-2 | Every producer and declared domain-checker invocation uses EA's bounded `ProducerExecutor` with the member's source repository checkout, at its pinned revision and clean, as the capability root; the exact request and result identities are recorded when EA minted them. A structurally invalid request has no invented identity, but its typed preflight refusal is retained. Every terminal state, including unavailable, timed out, malformed and refused execution, appears as an attempt. A successful producer invocation retains a separate `MeasurementCollection` whose ID and digest the attempt names. Resume never overwrites or erases an earlier attempt. | Test (TC-1941, TC-1942) |
| FR-114-AC-3 | The checker compares each named collection's retained bytes with its claimed digest and checks its source graph, subject, plan ID and definition version against the campaign member. It checks retained plan-verdict and domain-checker receipts against their claimed digests and their binding to the definition, source graph, member and raw artifacts. Missing evidence is inconclusive; contradictory or altered evidence is reject. | Test (TC-1943, TC-1944) |
| FR-114-AC-4 | The `all-required` result accepts exactly when every required member has a completed, independently accepted attempt and all dependencies accept. Rejected members reject the campaign; missing or unavailable evidence is inconclusive. Optional members remain visible and cannot mask a required failure. | Test (TC-1945) |
| FR-114-AC-5 | The CLI emits one complete versioned campaign-verdict document with the canonical attempt-inventory digest, member outcomes and reason codes. It exits nonzero for reject or inconclusive without losing that document. A non-TL fixture exercises the path before any TL campaign is used. | Test (TC-1946) |

## Constraints

- Campaign and procedure contracts belong to engineering-assurance and are
  generated through FCD; Quoin does not redeclare their wire types.
- Producer process supervision belongs to EA. Quoin does not spawn a second
  producer runner if EA cannot provide the declared containment profile.
- A producer's own `pass` value is never sufficient for campaign acceptance.
- The checker treats `CampaignRun.verdict` as a claim to compare with its own
  result. It computes the definition, source-graph and inventory identities
  from RFC 8785 canonical bytes and SHA-256, not from producer-supplied
  digest labels.

## Dependencies

- Engineering-assurance supplies the FCD-generated campaign contracts, a
  procedure resolver and the bounded producer executor.
- FR-044 supplies write-once collections; FR-108 supplies independent
  per-plan measurement verdicts.
