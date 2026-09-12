---
id: FR-100
title: "Provide evidence, measurement and change-assurance capability in Rust"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-098"
    type: "requires"
---

# FR-100: Provide evidence, measurement and change-assurance capability in Rust

## Description

Quoin SHALL implement its evidence store, change-assurance, audit, advisory,
assurance, graph-analysis and measurement behaviour in Rust crates reached
through the `quoin-core` boundary, keeping one evidence store and one
measurement model and consuming shared assurance capability from
`engineering-assurance`.

## Inputs

- The existing evidence, measurement, intervention, operational and
  change-assurance records under the evidence store root.
- Producer-supplied evidence documents and their declared adapters.
- The governed corpus pins and the completed module set.
- `engineering-assurance`'s published capability for advisory floors, package
  audit, bounded producer execution, evaluation and evidence reporting, corpus
  accounting and canonical serialization.

## Outputs

- Canonical evidence, measurement and change-assurance records written to the
  single Quoin evidence store.
- Audit, advisory, assurance-case, graph-analysis and portfolio reports as
  versioned boundary result documents.
- A recorded three-part retention answer for each capability kept local.

## Behavior

- Quoin SHALL break the `evidence`-to-`change-assurance` dependency cycle and the
  `auditor`-to-`advisor` dependency cycle before any crate implementing those
  capabilities exists.
- `quoin-store` SHALL own canonical JSON serialization, JCS canonicalization,
  blake3 digest computation and atomic record replacement, and every other crate
  SHALL obtain them from it.
- `quoin-evidence` SHALL preserve the on-disk store layout, the
  `STORE_SCHEMA_VERSION` value and the canonical record serialization, so that
  reading and re-serializing every store in the ecosystem returns byte-identical
  records.
- `quoin-change-assurance` SHALL preserve record integrity checking, proof
  attestation and verification verdicts, including each refusal for a stale,
  tampered or mismatched record.
- `quoin-auditor` SHALL carry both audit and advisory behaviour in one crate,
  and `quoin-assurance` and `quoin-graph-analysis` SHALL preserve the assurance
  case view and the read-only graph analysis.
- The five measurement crates SHALL preserve the existing measurement states,
  failure partitions, and published rates with their unit, population and
  method.
- Quoin SHALL remain advisory and read-only over the governed corpus and SHALL
  write no corpus byte.
- Before implementing a capability for advisory floors, package audit, bounded
  producer execution, evaluation or evidence reporting, corpus accounting or
  canonical serialization outside `quoin-store`, Quoin SHALL consume
  `engineering-assurance`'s implementation; if `engineering-assurance` does not
  supply it, then Quoin SHALL file a gap ticket in `engineering-assurance`
  rather than growing a local substitute.
- For each capability retained locally, Quoin SHALL record a three-part answer —
  whether it has to be local, whether it is Rust, and whether it belongs in
  shared tooling —
  and SHALL record the reason when the third answer is no.
- The evidence store SHALL remain Quoin's, and Quoin SHALL NOT take a dependency
  on `engineering-assurance` for evidence retention.
- Quoin SHALL write the first-party non-Rust executable line count as a Quoin
  measurement collection into the existing evidence store on each enforcement
  run.

## Error Conditions

An unreadable store, a record whose digest does not match, a store schema version
the implementation does not support, a producer document that does not validate
against its declared adapter, and an absent `engineering-assurance` capability
each produce a distinguishable refusal and are never reported as an empty but
successful report.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-100-CON-1 | Quoin SHALL NOT create a second evidence store, evidence model or measurement model. | Architecture | Test |
| FR-100-CON-2 | Quoin SHALL NOT reimplement a capability `engineering-assurance` publishes. | Responsibility | Inspection |
| FR-100-CON-3 | The Rust implementation SHALL NOT write a governed-corpus byte. | Data Integrity | Test |
| FR-100-CON-4 | Canonicalization and digest behaviour SHALL NOT be reimplemented outside `quoin-store`. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-100-AC-1 | `cargo tree --manifest-path rust/Cargo.toml` shows no dependency cycle, and the two named TypeScript cycles are absent from the tree before the first domain crate is added. | Test (TC-1633) |
| FR-100-AC-2 | Reading and re-serializing every store reachable in the ecosystem through `quoin-store` returns byte-identical records, and `STORE_SCHEMA_VERSION` is unchanged. | Test (TC-1634) |
| FR-100-AC-3 | Each change-assurance refusal for a stale, tampered or mismatched record returns the same classification from the Rust implementation as from the retained one. | Property (TC-1635) |
| FR-100-AC-4 | Each measurement report produced by the Rust crates carries the same states, failure partitions, unit, population and method as the retained report for the same pins. | Test (TC-1636) |
| FR-100-AC-5 | A measurement run over a read-only governed corpus completes and the corpus working tree is unchanged afterwards. | Test (TC-1637) |
| FR-100-AC-6 | Every locally retained assurance capability carries a recorded three-part retention answer, and a capability with no recorded answer fails the gate. | Test (TC-1638) |
| FR-100-AC-7 | Each enforcement run writes the first-party non-Rust executable line count into the existing evidence store as a measurement collection with its unit and population. | Test (TC-1639) |
| FR-100-AC-8 | A static check finds canonicalization and digest computation implemented only in `quoin-store`, and fails when a second implementation is planted. | Test (TC-1640) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md), [FR-098](./FR-098-semantic-and-identity-parity.md) and [FR-099](./FR-099-rust-catalog-and-validation-capability.md); [FR-030](./FR-030-evidence-store.md), [FR-063](./FR-063-change-assurance-record-integrity.md) and [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md), whose behaviour it preserves.
- **Downstream**: [FR-101](./FR-101-retire-replaced-executable-paths.md); [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains it.
