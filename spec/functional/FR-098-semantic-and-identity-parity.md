---
id: FR-098
title: "Preserve semantic and identity parity across every port"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-098: Preserve semantic and identity parity across every port

## Description

Quoin SHALL preserve the observable semantics of every ported capability,
including each refusal and non-success path, and SHALL produce byte-identical
canonical serializations from the Rust implementation and the implementation
it replaces, compared byte for byte, except for the explicit PLAT-989 RFC 8785
and record-identity transition defined by FR-100-AC-10/11 and NFR-025. No other
difference is exempt. No digest is computed or compared anywhere in the port.

## Inputs

- The retained TypeScript entry point for the capability under port.
- The `quoin-core` operation replacing it.
- Golden corpora: `tests/fixtures`, this repository's own `spec/`, `plan/` and
  `reviews/` trees, and one external spec corpus named by repository in the
  checked-in differential manifest.
- Every canonical record in each reachable store, where the
  reachable set is the evidence store root under the caller-selected
  configuration root together with each store root named in the checked-in store
  inventory, and an absent inventory refuses the replay rather than narrowing
  it.

## Outputs

- A native fixture replay report per candidate revision comparing canonical
  stdout, exit status and normalized diagnostic shape.
- A record replay report naming the population replayed and the count of
  records whose bytes differ.
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
- Quoin SHALL obtain RFC 8785 encoding from `quire-canonical` and implement
  store identity and atomic-write operations only in `quoin-store`.
- Outside the enumerated PLAT-989 transition, that path SHALL produce the same
  bytes as the retained implementation for the same input. Record identifiers
  are opaque, time-ordered UUIDs (v7) assigned when a record is written, and a
  record is stored as `<recordId>.json`; an identifier is never derived from
  content. Existing stored records keep their current identifiers and file
  names, which are read as opaque strings and never recomputed; legacy digest
  members inside them are ignored on read and never written.
- Quoin SHALL port the retained strict JSON parser's refusal boundary — its
  refusal of a UTF-8 byte-order mark, of non-fatal UTF-8 and of trailing content
  — and SHALL compare those refusals as part of parity, because they guard
  canonicalization rather than following it.
- Before a store-backed capability cuts over, Quoin SHALL read every record in
  every reachable store through Rust, write it back, and SHALL refuse the
  cutover if the written bytes differ from the stored bytes outside the exact
  PLAT-989 transition mapping enumerated under NFR-025.
- Quoin SHALL test JCS canonicalization over adversarial Unicode, number-format
  and key-order inputs.
- Quoin SHALL port each `fast-check` property suite under `tests/props` to a
  `proptest` suite asserting the same property.
- If Rust disagrees with a retained expected fixture outside the explicit
  PLAT-989 transition on any golden input, then
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
diagnostic tuple, any unlisted byte difference during replay outside the
PLAT-989 transition, an unreadable store and a
differential run whose compared population is empty each block cutover.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-098-CON-1 | Cutover SHALL NOT change an existing identity domain except the enumerated PLAT-989 assurance identity transition under NFR-025. | Data Integrity | Test |
| FR-098-CON-2 | Quoin SHALL delegate RFC 8785 encoding to quire-canonical and retain store identity and atomic-write logic only in quoin-store. | Architecture | Test |
| FR-098-CON-3 | Withdrawn: the differential replay over captured records is gone with the retained implementation it compared against; no parity evidence is recorded, so there is nothing for an empty population to be recorded as (id not reused). | Reporting | Withdrawn |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | Native Rust tests replay retained expected fixtures over the golden corpora and report the compared population count with each run. | Test (TC-1618) |
| FR-098-AC-2 | For every schema in the corpus, Rust returns the retained expected verdict, and diagnostics agree as normalized instance-location, keyword and schema-path tuples. | Property (TC-1619) |
| FR-098-AC-3 | Withdrawn: the replay of captured records through the retained implementation is gone; records are checked by behavior under FR-063 and FR-064 (id not reused). | Withdrawn (TC-1620) |
| FR-098-AC-4 | JCS canonicalization produces identical bytes for adversarial Unicode, number-format and key-order inputs in both implementations. | Property (TC-1621) |
| FR-098-AC-5 | Each ported refusal path returns the same non-success classification from the Rust implementation as from the retained one. | Property (TC-1622) |
| FR-098-AC-6 | Every `tests/props` property has a `proptest` counterpart asserting the same property, and a property holding in TypeScript and violated in Rust fails the run. | Test (TC-1623) |
| FR-098-AC-7 | Withdrawn: there is no differential run whose population could be zero, and no cutover gate it could satisfy (id not reused). | Withdrawn (TC-1624) |
| FR-098-AC-8 | Validation asserts no `format` keyword in either implementation, and the compared diagnostic set excludes schema-authoring strict-mode diagnostics. | Test (TC-1692) |
| FR-098-AC-9 | A document the retained strict JSON parser refuses — byte-order mark, non-fatal UTF-8, trailing content — is refused by the Rust implementation with the same classification, before canonicalization runs. | Test (TC-1693) |
| FR-098-AC-10 | Record identifiers produced by the Rust implementation are opaque UUIDs not derived from content, files are named `<recordId>.json`, and legacy `sha256:<hex>` identifiers and `sha256-<hex>.json` file names in existing stores are read as opaque strings and left unchanged; the transition is reported under NFR-025. | Test (TC-1694) |
| FR-098-AC-11 | After a capability cuts over, the native fixture suite spawns no retained implementation for that capability and compares against committed fixtures. | Test (TC-1695) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md).
- **Downstream**: [FR-099](./FR-099-rust-catalog-and-validation-capability.md), and [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) consume this parity evidence; [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains it.
