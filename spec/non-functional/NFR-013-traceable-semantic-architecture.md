---
id: NFR-013
title: "Semantic architecture decisions are traceable and standalone-readable"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "constrains"
---

# NFR-013: Semantic architecture decisions are traceable and standalone-readable

## Statement

While the semantic-module architecture is reviewed or maintained, it SHALL remain understandable
from the repository alone with every normative ownership, authority, and compatibility claim traced
to a local requirement or an identity-pinned external decision.

## Scope

- Applies to the semantic-module architecture index, normative records, ADRs, and reconciliation
  ledger introduced for issue #289.
- External references include `filament-core-data` and `quire-rs` decisions.

## Rationale

Chat history, issue discussion, and a mutable branch tip are useful provenance but not durable
architecture. A future maintainer must be able to identify what is decided, what is provisional,
what is merely evidence, and which owner can change it.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Normative architecture claims with a requirement or decision reference | 100% | 100% | Test (TC-1151) |
| External decisions with repository, path, status, and revision/date | 100% | 100% | Test (TC-1151) |
| Broken links from the architecture index | 0 | 0 | Test (TC-1152) |
| Undeclared provisional or unresolved decisions presented as normative | 0 | 0 | Test (TC-1153) |

## Verification

The architecture contract test walks the index, resolves local links, and inspects the external
decision ledger for repository, path, status, and revision/date fields. Review verifies that the
record can be interpreted without issue or conversation context.

## Dependencies

- **Upstream**: [FR-046](../functional/FR-046-record-semantic-data-planes.md) through
  [FR-050](../functional/FR-050-reconcile-quire-decisions.md)
