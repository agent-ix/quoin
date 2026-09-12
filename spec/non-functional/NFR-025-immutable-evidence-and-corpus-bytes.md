---
id: NFR-025
title: "Evidence and accepted-corpus bytes are immutable across the port"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-098"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-103"
    type: "constrains"
---

# NFR-025: Evidence and accepted-corpus bytes are immutable across the port

## Statement

Quoin SHALL leave every stored evidence byte and every accepted-corpus byte
unchanged across the port, so that each record's digest identity before a
cutover equals its digest identity after the cutover, after a revert, and after
the replaced path is deleted.

## Scope

- Applies to: every record under the Quoin evidence store root — evidence,
  measurement, intervention, operational and change-assurance records — and every
  accepted-corpus byte reachable from the governed corpus pins.
- Operational context: each cutover change, each revert of a cutover, each
  deletion change, and the corpus consolidation.
- Not applied to: newly written records produced after a cutover, which are new
  bytes rather than rewritten ones; and generated build output, which carries its
  own provenance under FR-097.

## Rationale

Quoin's digests are identifiers, not checksums. A record cites another record by
digest, and a change-assurance verdict is a statement about a digest. If a
canonicalization difference moves a digest, the citing records do not become
wrong in a way a test reports — they become unresolvable, and the damage is not
recoverable by re-running anything. That is why digest agreement is a hard gate
before the store-backed cutover rather than a regression checked afterwards.

The accepted corpus has the same property for a different reason: it is the
population that every measurement figure in this repository is quoted against.
Rewriting a corpus byte silently redefines every historical rate without
invalidating the report that carries it.

The guarantee must hold in both directions. A cutover that is reverted must
restore the retained invocation without having left a rewritten byte behind, or
the reversibility FR-101 claims is not reversibility.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Evidence records whose digest changed across a cutover | 0 | 0 | Test |
| Accepted-corpus bytes changed across the port | 0 | 0 | Test |
| Stores whose read-and-re-serialize round trip is not byte-identical | 0 | 0 | Test |
| Digest-replay population covered before a store-backed cutover | 100% of reachable records | 100% | Test |

## Verification

Before each store-backed cutover, the digest replay enumerates every record in
every reachable store, computes its digest through the retained and the Rust
implementations, and asserts zero mismatches while reporting the population it
covered. A separate round-trip check reads and re-serializes every store and
asserts byte identity. Across each cutover, revert and deletion change, a tree
comparison over the evidence and accepted-corpus roots asserts zero changed
bytes. A planted single-byte canonicalization difference must fail the replay,
and a replay whose population is zero is reported as inconclusive rather than as
clean.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-025-AC-1 | A digest replay over every reachable store reports zero mismatches and names the non-empty population it covered. | Test (TC-1665) |
| NFR-025-AC-2 | A planted single-byte canonicalization difference makes the digest replay fail. | Test (TC-1666) |
| NFR-025-AC-3 | Reading and re-serializing every store returns byte-identical records. | Test (TC-1667) |
| NFR-025-AC-4 | Comparing the evidence and accepted-corpus trees before and after a cutover, after its revert, and after the deletion change finds zero changed bytes. | Test (TC-1668) |
| NFR-025-AC-5 | A digest replay whose population is zero is reported as inconclusive and does not satisfy the cutover gate. | Test (TC-1669) |

## Dependencies

- **Upstream**: [FR-098](../functional/FR-098-semantic-and-identity-parity.md) and [FR-100](../functional/FR-100-rust-evidence-measurement-change-assurance.md), which implement the single canonicalization and digest path.
- **Downstream**: [FR-101](../functional/FR-101-retire-replaced-executable-paths.md) and [FR-103](../functional/FR-103-corpus-consolidation.md), whose cutovers and consolidation this bounds.
