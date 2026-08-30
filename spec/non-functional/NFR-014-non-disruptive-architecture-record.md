---
id: NFR-014
title: "The semantic architecture record is non-disruptive"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "constrains"
---

# NFR-014: The semantic architecture record is non-disruptive

## Statement

Until a separately reviewed enforcement, publication, or migration ticket is activated, the
semantic-module architecture change SHALL preserve current Quoin and Quire behavior, existing module
validity, current Avro consumers, and all feature-work integration surfaces.

## Scope

- Applies to issue #289 and its documentation, requirement, test, review, and plan artifacts.
- Excludes future compiler, code-generation, publication, manifest evolution, and migration tickets.

## Rationale

The architecture is a prerequisite for a large program running beside active feature development.
Recording the target model must not make the target model operational by implication.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Production source, manifest, schema, generated package, or migration files changed | 0 | 0 | Test (TC-1154) |
| Existing tests regressed | 0 | 0 | Test (TC-1154) |
| Architecture PR merged without named Quoin/Quire maintainer review | 0 | 0 | Inspection (TC-1155) |

## Verification

A changed-path guard rejects production, manifest, schema, generated-package, and migration changes.
The full existing quality gates run unchanged. The pull request stops for named maintainer review
before merge because this record becomes normative guidance for later work.

## Dependencies

- **Upstream**: [FR-046](../functional/FR-046-record-semantic-data-planes.md),
  [FR-049](../functional/FR-049-preserve-dynamic-and-generated-modules.md),
  [FR-050](../functional/FR-050-reconcile-quire-decisions.md)
