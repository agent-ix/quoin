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
including each refusal and non-success path, and SHALL produce RFC 8785
canonical serializations whose bytes are fixed by the canonical encoder alone.
No digest is computed or compared anywhere in the port.

The retained implementation, and every recorded golden corpus, fixture or
expected result captured from it, is gone. There is no recorded oracle left to
replay, so the criteria that existed only to replay one (FR-098-AC-1, AC-2,
AC-5 and AC-6) are withdrawn in place, and the "refuse the cutover" clause that
compared a Rust result with a recorded one is moot and removed. Behaviour that
is observable on live code stays: refusal paths, the strict JSON parser
boundary, the single canonical encoder, opaque record identifiers, and the rule
that no suite starts a retained implementation.

## Inputs

- The `quoin-core` operation under test.
- Every canonical record in each reachable store, where the
  reachable set is the evidence store root under the caller-selected
  configuration root together with each store root named in the checked-in store
  inventory.

## Outputs

- Canonical stdout, exit status and normalized diagnostic shape from the
  `quoin-core` operation.

## Behavior

- Quoin SHALL treat a validation verdict as contractual and SHALL treat
  diagnostic message text as non-contractual, comparing diagnostics as the
  normalized tuple of instance location, schema keyword and schema path.
- Quoin SHALL preserve every refusal and non-success path a ported capability
  had, including refusals for malformed input, stale records, tampered records
  and unavailable hosts.
- Quoin SHALL obtain RFC 8785 encoding from `quire-canonical` and implement
  store identity and atomic-write operations only in `quoin-store`.
- Record identifiers
  are opaque random UUIDs (v4) assigned once when a record is written, stored,
  and compared as exact strings, and a record is stored as `<recordId>.json`;
  an identifier is never derived from content.
- Quoin SHALL port the retained strict JSON parser's refusal boundary — its
  refusal of a UTF-8 byte-order mark, of non-fatal UTF-8 and of trailing content
  — and SHALL refuse them before canonicalization runs, because they guard
  canonicalization rather than following it.
- Quoin SHALL resolve the JSON Schema validator version used across the
  workspace to one version, in `rust/Cargo.toml`.
- Quoin SHALL assert no `format` keyword during validation, matching the
  retained validator, which declares no format package; enabling format
  assertion is a deliberate semantic change requiring its own requirement.
- Quoin SHALL exclude the retained validator's schema-authoring strict-mode
  diagnostics from the compared diagnostic set, because they analyse the schema
  rather than the instance and have no counterpart in the Rust validator.
- After a capability cuts over, no integration suite SHALL execute a retained
  implementation.

## Error Conditions

A differing exit status, a differing normalized diagnostic tuple, any byte
difference during replay and an unreadable store each block cutover.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-098-CON-1 | Cutover SHALL NOT change an existing identity domain. | Data Integrity | Test |
| FR-098-CON-2 | Quoin SHALL delegate RFC 8785 encoding to quire-canonical and retain store identity and atomic-write logic only in quoin-store. | Architecture | Test |
| FR-098-CON-3 | Withdrawn: the differential replay over captured records is gone with the retained implementation it compared against; no parity evidence is recorded, so there is nothing for an empty population to be recorded as (id not reused). | Reporting | Withdrawn |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | Withdrawn: it replayed retained expected fixtures over golden corpora; no recorded oracle remains to replay (id not reused). | Withdrawn (TC-1618) |
| FR-098-AC-2 | Withdrawn: it compared Rust verdicts with a retained expected verdict per schema; no recorded oracle remains (id not reused). | Withdrawn (TC-1619) |
| FR-098-AC-3 | Withdrawn: the replay of captured records through the retained implementation is gone; records are checked by behavior under FR-063 and FR-064 (id not reused). | Withdrawn (TC-1620) |
| FR-098-AC-4 | Withdrawn: it compared JCS bytes against a recorded jcs-oracle corpus; no recorded oracle remains (id not reused). | Withdrawn (TC-1621) |
| FR-098-AC-5 | Withdrawn: it compared each refusal classification with the retained implementation's; no retained implementation remains (id not reused). | Withdrawn (TC-1622) |
| FR-098-AC-6 | Withdrawn: it required a `proptest` counterpart of every retained `fast-check` suite and a TypeScript-versus-Rust comparison; the TypeScript suites are gone (id not reused). | Withdrawn (TC-1623) |
| FR-098-AC-7 | Withdrawn: there is no differential run whose population could be zero, and no cutover gate it could satisfy (id not reused). | Withdrawn (TC-1624) |
| FR-098-AC-8 | Validation asserts no `format` keyword in either implementation, and the compared diagnostic set excludes schema-authoring strict-mode diagnostics. | Test (TC-1692) |
| FR-098-AC-9 | A document the retained strict JSON parser refuses — byte-order mark, non-fatal UTF-8, trailing content — is refused by the Rust implementation before canonicalization runs. | Test (TC-1693) |
| FR-098-AC-10 | Withdrawn: the identifier format is stated once in the Behavior clause, and no legacy identifier form remains (id not reused). | Withdrawn (TC-1694) |
| FR-098-AC-11 | After a capability cuts over, no integration suite spawns a retained implementation. | Test (TC-1695) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md).
- **Downstream**: [FR-099](./FR-099-rust-catalog-and-validation-capability.md), and [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) consume this parity evidence; [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains it.
