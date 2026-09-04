---
id: NFR-017
title: "Non-disruptive semantic manifest evolution"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-073"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-074"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-075"
    type: "constrains"
  - target: "ix://agent-ix/quoin/NFR-014"
    type: "depends_on"
---

# NFR-017: Non-disruptive semantic manifest evolution

## Statement

The semantic-module contract SHALL keep every currently installed manifest and
every current corpus artifact valid at their current severity until the
advisory sweep report exists and a human promotes an enforcing module release.

## Scope

- Permitted: the `semantic` block and its schema, the mapping contract, golden fixtures, advisory diagnostics, the authoring pack text, Quoin tests, specs, plans, reviews.
- Prohibited: a required manifest key, an error-severity legacy diagnostic by default, any write to a corpus repository, publishing a generated package, changing a catalog pin.

## Rationale

The installed ecosystem is 237 repositories deep; a contract that invalidates
one default module or one corpus artifact on install is a migration, not a
specification. Advisory-first with a measured sweep is the standing disruption
control.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Default module manifests that fail to load after the change | 0 | 0 | Integration test loading every module in `default-modules.yaml` |
| Corpus artifacts whose validation severity rises above `warning` by default | 0 | 0 | `quoin semantic sweep` report over the fixture corpus (FR-074) |
| Corpus repository files written | 0 | 0 | Static changed-path test |
| Required manifest keys added | 0 | 0 | Static schema diff test |
| Legacy-form promotion without a recorded sweep report | 0 | 0 | Unit test of the install-time guard |

## Verification

Load the default module set before and after, run the corpus sweep in report
mode, diff the manifest schema `required` arrays, and test the promotion guard.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-017-AC-1 | Every default module manifest loads with no new diagnostic. | Test |
| NFR-017-AC-2 | The sweep over the corpus fixture set reports only `warning`-severity semantic findings by default. | Test |
| NFR-017-AC-3 | No corpus repository path appears in the change set. | Analysis |
| NFR-017-AC-4 | The manifest schema's `required` arrays are unchanged. | Analysis |

## Dependencies

- **Upstream**: [NFR-014](./NFR-014-non-disruptive-architecture-record.md), `agent-ix/quoin#288` corpus review
- **Downstream**: `agent-ix/quoin#291`, `agent-ix/quoin#292`
