---
id: FR-098
title: "Preserve semantic and identity parity across every port"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-098: Preserve semantic and identity parity across every port

## Description

Quoin SHALL preserve the observable semantics of every ported capability,
including each refusal and non-success path, and SHALL produce byte-identical
canonical serializations and identical digests from the Rust implementation and
the implementation it replaces.

## Inputs

- The retained TypeScript entry point for the capability under port.
- The `quoin-core` operation replacing it.
- Golden corpora: `tests/fixtures`, this repository's own `spec/`, `plan/` and
  `reviews/` trees, and the pinned external spec corpus cited in
  `src/quire/exec.ts`.
- Every digest and canonical record reachable in the evidence, measurement and
  change-assurance stores.

## Outputs

- A differential report per candidate revision comparing canonical stdout, exit
  status and normalized diagnostic shape.
- A digest replay report naming the population replayed and the count of
  mismatches.
- A parity refusal naming the first differing input.

## Behavior

- `quoin-difftest` SHALL feed one request document to both the retained
  TypeScript entry point and the `quoin-core` operation and SHALL compare the
  canonical-JSON result, the exit status and the normalized diagnostic shape.
- Quoin SHALL treat a validation verdict as contractual and SHALL treat
  diagnostic message text as non-contractual, comparing diagnostics as the
  normalized tuple of instance location, schema keyword and schema path.
- Quoin SHALL preserve every refusal and non-success path a ported capability
  had, including refusals for malformed input, stale records, tampered records
  and unavailable hosts.
- Quoin SHALL implement canonical JSON serialization, JCS canonicalization and
  blake3 digest computation in exactly one crate, and SHALL produce the same
  bytes as the retained implementation for the same input.
- Before a store-backed capability cuts over, Quoin SHALL replay every digest in
  every reachable store through the retained and the Rust implementation, and
  SHALL refuse the cutover if any digest differs.
- Quoin SHALL test JCS canonicalization separately from digest computation over
  adversarial Unicode, number-format and key-order inputs.
- Quoin SHALL port each `fast-check` property suite under `tests/props` to a
  `proptest` suite asserting the same property.
- If the retained and Rust implementations disagree on any golden input, then
  Quoin SHALL refuse the cutover and SHALL name the differing input, the
  operation and both results.
- Quoin SHALL resolve the JSON Schema validator version used across the
  workspace to one pin, recorded in `rust/Cargo.toml`.

## Error Conditions

A differing canonical result, a differing exit status, a differing normalized
diagnostic tuple, any digest mismatch during replay, an unreadable store and a
differential run whose compared population is empty each block cutover.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-098-CON-1 | Cutover SHALL NOT change an existing identity domain. | Data Integrity | Test |
| FR-098-CON-2 | Canonicalization and digest logic SHALL exist in exactly one crate. | Architecture | Test |
| FR-098-CON-3 | A differential run over an empty population SHALL NOT be recorded as parity evidence. | Reporting | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | `cargo test -p quoin-difftest` compares retained and Rust results over the golden corpora and reports the compared population count with each run. | Test (TC-1618) |
| FR-098-AC-2 | For every schema in the corpus, retained and Rust validation return the identical verdict, and their diagnostics agree as normalized instance-location, keyword and schema-path tuples. | Property (TC-1619) |
| FR-098-AC-3 | Replaying every digest in every reachable store through both implementations yields zero mismatches, and a planted single-byte canonicalization change makes the replay fail. | Test (TC-1620) |
| FR-098-AC-4 | JCS canonicalization produces identical bytes for adversarial Unicode, number-format and key-order inputs in both implementations. | Property (TC-1621) |
| FR-098-AC-5 | Each ported refusal path returns the same non-success classification from the Rust implementation as from the retained one. | Property (TC-1622) |
| FR-098-AC-6 | Every `tests/props` property has a `proptest` counterpart asserting the same property, and a property holding in TypeScript and violated in Rust fails the run. | Test (TC-1623) |
| FR-098-AC-7 | A differential run whose compared population is zero is reported as inconclusive and refuses to satisfy a cutover gate. | Test (TC-1624) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md).
- **Downstream**: [FR-099](./FR-099-rust-catalog-and-validation-capability.md), [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) and [FR-101](./FR-101-retire-replaced-executable-paths.md) consume this parity evidence; [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains it.
