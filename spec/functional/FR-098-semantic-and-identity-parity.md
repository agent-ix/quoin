---
id: FR-098
title: "Preserve semantic and identity parity across every port"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
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
  `reviews/` trees, and one external spec corpus pinned by repository and
  revision in the checked-in differential manifest.
- Every digest and canonical record in each reachable store, where the
  reachable set is the evidence store root under the caller-selected
  configuration root together with each store root named in the checked-in store
  inventory, and an absent inventory refuses the replay rather than narrowing
  it.

## Outputs

- A native fixture replay report per candidate revision comparing canonical
  stdout, exit status and normalized diagnostic shape.
- A digest replay report naming the population replayed and the count of
  mismatches.
- A parity refusal naming the first differing input.

## Behavior

- The native fixture suite SHALL replay every retained request against the
  `quoin-core` operation and compare its canonical-JSON result, exit status
  and normalized diagnostic shape with the committed expected fixture captured
  from the retained implementation before cutover.
- Quoin SHALL treat a validation verdict as contractual and SHALL treat
  diagnostic message text as non-contractual, comparing diagnostics as the
  normalized tuple of instance location, schema keyword and schema path.
- Quoin SHALL preserve every refusal and non-success path a ported capability
  had, including refusals for malformed input, stale records, tampered records
  and unavailable hosts.
- Quoin SHALL implement canonical JSON serialization, JCS canonicalization,
  sha256 record-identity computation and blake3 digest computation in exactly
  one crate.
- That crate SHALL produce the same bytes as the retained implementation for
  the same input, including the `sha256:<hex>` record identifiers and
  `sha256-<hex>.json` file names the retained evidence records use.
- Quoin SHALL port the retained strict JSON parser's refusal boundary — its
  refusal of a UTF-8 byte-order mark, of non-fatal UTF-8 and of trailing content
  — and SHALL compare those refusals as part of parity, because they guard
  canonicalization rather than following it.
- Before a store-backed capability cuts over, Quoin SHALL capture every digest
  in every reachable store as retained expected output, replay that corpus
  through Rust, and SHALL refuse the cutover if any digest differs.
- Quoin SHALL test JCS canonicalization separately from digest computation over
  adversarial Unicode, number-format and key-order inputs.
- Quoin SHALL port each `fast-check` property suite under `tests/props` to a
  `proptest` suite asserting the same property.
- If Rust disagrees with a retained expected fixture on any golden input, then
  Quoin SHALL refuse the cutover and SHALL name the differing input, the
  operation and both results.
- Quoin SHALL resolve the JSON Schema validator version used across the
  workspace to one pin, recorded in `rust/Cargo.toml`.
- Quoin SHALL assert no `format` keyword during validation, matching the
  retained validator, which declares no format package; enabling format
  assertion is a deliberate semantic change requiring its own requirement.
- Quoin SHALL exclude the retained validator's schema-authoring strict-mode
  diagnostics from the compared diagnostic set, because they analyse the schema
  rather than the instance and have no counterpart in the Rust validator.
- After a capability cuts over, the native fixture suite SHALL execute no
  retained implementation and SHALL compare against committed expected
  fixtures only.

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
| FR-098-AC-1 | Native Rust tests replay retained expected fixtures over the golden corpora and report the compared population count with each run. | Test (TC-1618) |
| FR-098-AC-2 | For every schema in the corpus, Rust returns the retained expected verdict, and diagnostics agree as normalized instance-location, keyword and schema-path tuples. | Property (TC-1619) |
| FR-098-AC-3 | Replaying every captured digest in every reachable store through Rust yields zero mismatches, and a planted single-byte canonicalization change makes the replay fail. | Test (TC-1620) |
| FR-098-AC-4 | JCS canonicalization produces identical bytes for adversarial Unicode, number-format and key-order inputs in both implementations. | Property (TC-1621) |
| FR-098-AC-5 | Each ported refusal path returns the same non-success classification from the Rust implementation as from the retained one. | Property (TC-1622) |
| FR-098-AC-6 | Every `tests/props` property has a `proptest` counterpart asserting the same property, and a property holding in TypeScript and violated in Rust fails the run. | Test (TC-1623) |
| FR-098-AC-7 | A differential run whose compared population is zero is reported as inconclusive and refuses to satisfy a cutover gate. | Test (TC-1624) |
| FR-098-AC-8 | Validation asserts no `format` keyword in either implementation, and the compared diagnostic set excludes schema-authoring strict-mode diagnostics. | Test (TC-1692) |
| FR-098-AC-9 | A document the retained strict JSON parser refuses — byte-order mark, non-fatal UTF-8, trailing content — is refused by the Rust implementation with the same classification, before canonicalization runs. | Test (TC-1693) |
| FR-098-AC-10 | `sha256:<hex>` record identifiers and `sha256-<hex>.json` file names produced by the Rust implementation are byte-identical to the retained ones for the same record. | Test (TC-1694) |
| FR-098-AC-11 | After a capability cuts over, the native fixture suite spawns no retained implementation for that capability and compares against committed fixtures. | Test (TC-1695) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md).
- **Downstream**: [FR-099](./FR-099-rust-catalog-and-validation-capability.md), [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) and [FR-101](./FR-101-retire-replaced-executable-paths.md) consume this parity evidence; [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains it.
