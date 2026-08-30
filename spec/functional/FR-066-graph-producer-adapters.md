---
id: FR-066
title: "Lossless adapters for governed graph producers"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-019"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-062"
    type: "requires"
  - target: "ix://agent-ix/quire-rs/FR-067"
    type: "requires"
  - target: "ix://agent-ix/quire-code-rs/FR-011"
    type: "requires"
---
# FR-066: Lossless adapters for governed graph producers

## Description

Quoin SHALL register versioned `quire-assurance-v1` and
`quire-code-graph-quality-v1` adapters. The first SHALL validate a Quire
assurance export into the authoritative input consumed by FR-062. The second
adapter SHALL transcribe one quire-code-rs graph-quality observation and its
retained raw scorer bytes into one FR-044 measurement collection.

Neither adapter runs its producer or judges its results.

## Inputs

The Quire adapter receives the assurance JSON, the accepted format/module/schema
premises, and its file identity.

The graph-quality adapter receives:

- one `graph-quality-observation-v1` JSON record;
- the exact raw scorer file named and digested by that record; and
- an invocation attestation carrying `subject`, `scope`, `timestamp`,
  `environment`, and FR-044's current verification-stack attestation.

The invocation attestation supplies facts absent from the producer record. It
contains no observation value, population count, result, or verdict.

## Outputs

The Quire adapter returns the accepted source and module premises plus the
artifact, obligation, symbol, relation, and relation-observation collections
without renaming a field or relation kind.

The graph-quality adapter returns one current-schema `MeasurementCollection`:

- `toolIdentity` is the contract identity `agent-ix/quire-code-rs`;
- `toolVersion`, configuration, source, corpus, plan, and definition identities
  copy their producer fields;
- subject, scope, timestamp, environment, and verification stack copy the
  invocation attestation;
- `rawEvidence` retains the complete producer record and exact scorer bytes as
  base64 with media type and verified SHA-256 digest; and
- normalized observations use metric `graph_quality` and dimensions
  `(measure, dimension, key)`.

Population totals and census entries become count observations. For a measured
population, each confusion-matrix component, unresolved count, ambiguous count,
and recall ratio becomes its own observation. A non-measured population retains
its valid census facts and emits one `not_computed` quality-state observation
naming `empty`, `unreadable`, or `unsupported`; it never emits result rows.

## Behavior

- Adapter selection SHALL be exact and versioned. An unknown adapter names the
  available adapters and performs no fallback.
- The Quire adapter SHALL apply FR-067's schema and accepted-premise checks
  before exposing any graph record.
- The graph-quality adapter SHALL validate the producer schema, recompute its
  canonical observation id, and verify the raw scorer bytes against the named
  digest before constructing a collection.
- The graph-quality adapter SHALL require an active `graph_quality`
  MeasurementPlan whose id and definition version equal the producer record.
- The adapter SHALL sort and deduplicate normalized dimensions and reject two
  producer facts that map to the same `(measure, dimension, key)` rather than
  apply last-write-wins.
- Collection validation, atomic write, idempotence, and same-id collision
  refusal SHALL reuse FR-044 unchanged.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-066-CON-1 | An adapter SHALL execute no Quire, extractor, scorer, suite, Git, or network operation. | Architecture | Inspection |
| FR-066-CON-2 | An adapter SHALL neither synthesize a missing producer or attestation field nor derive a threshold, verdict, or score absent from the input. | Integrity | Test |
| FR-066-CON-3 | The Quire adapter SHALL expose the producer graph rather than re-read frontmatter or reconstruct relationships. | Architecture | Inspection |
| FR-066-CON-4 | Existing normalized run-evidence adapters and direct MeasurementCollection intake SHALL remain available. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-066-AC-1 | The registry exposes exactly versioned Quire-assurance and quire-code graph-quality adapters, and an unknown name fails with the available names before reading records. | Test (TC-1293) |
| FR-066-AC-2 | A valid Quire export is exposed field-for-field with its source/module premises, while each unknown format, module version, or schema digest is refused before any graph record. | Test (TC-1294) |
| FR-066-AC-3 | FR-062 receives the adapter's artifacts, obligations, symbols, relations, observations, and availability states without a frontmatter read or relation-kind translation. | Integration (TC-1295) |
| FR-066-AC-4 | Graph-quality intake validates schema version 1, recomputes the canonical observation id, and refuses changed or unknown fields before constructing a collection. | Test (TC-1296) |
| FR-066-AC-5 | The exact scorer bytes reproduce the declared digest and are retained with media type and base64 bytes; a missing or mismatched attachment leaves no collection. | Test (TC-1297) |
| FR-066-AC-6 | Missing subject, scope, timestamp, environment, verification-stack field, or immutable producer identity is refused independently rather than invented. | Test (TC-1298) |
| FR-066-AC-7 | Intake refuses an absent, inactive, wrong-id, or wrong-definition graph-quality plan and names both expected and observed premises. | Test (TC-1299) |
| FR-066-AC-8 | Population totals and all language, node-kind, relation-kind, and resolver-tier census values map bijectively to sorted count observations. | Property (TC-1300) |
| FR-066-AC-9 | A measured record maps every confusion component, unresolved count, ambiguous count, and recall ratio bijectively; duplicate normalized keys are refused. | Property (TC-1301) |
| FR-066-AC-10 | Empty, unreadable, and unsupported records retain census facts, emit one distinct `not_computed` state, and emit no result observation. | Test (TC-1302) |
| FR-066-AC-11 | Repeated identical intake is byte-idempotent, a same-id/different-content collision cannot replace the collection, and existing direct collection intake remains unchanged. | Integration (TC-1303) |
| FR-066-AC-12 | Static boundaries prove both adapters execute no producer or network operation and the Quire adapter imports no frontmatter reader (CON-1, CON-3). | Inspection (TC-1304) |

## Dependencies

- **Upstream**: [FR-044](./FR-044-plan-governed-measurements.md),
  [FR-062](./FR-062-read-only-evidence-graph-analysis.md), Quire FR-067/068,
  and quire-code-rs FR-011/012 plus MP-001.
- **Downstream**: [FR-067](./FR-067-graph-portfolio-reporting.md) renders the
  accepted graph and measurement records.
