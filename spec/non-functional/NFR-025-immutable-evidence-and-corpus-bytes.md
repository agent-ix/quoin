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

Quoin SHALL leave every existing stored evidence byte and every accepted-corpus
byte unchanged across the port, including cutovers, reverts and deletion changes.
Quoin SHALL preserve derived record identities across those changes except for
the explicitly bounded PLAT-989 assurance identity transition defined below.

## Scope

- Applies to: every record under the Quoin evidence store root — evidence,
  measurement, intervention, operational and change-assurance records — together
  with each store root named in the checked-in store inventory, and every
  accepted-corpus byte reachable from the governed corpus pins. An absent store
  inventory refuses the replay rather than narrowing its population.
- Applies to: records the Rust implementation newly writes, which must be
  byte-identical to what the retained implementation would have written for the
  same input, except for the enumerated PLAT-989 transition outputs below.
- PLAT-989 transition: experiment and operational assurance record identities
  change from SHA-256 over pretty storage bytes with a final newline to SHA-256
  over RFC 8785 bytes under FR-100. Only the resulting `recordId` values,
  `sha256-<hex>.json` basenames and references to those changed identities are
  exempt from retained-writer byte parity. Record schemas and all other fields
  retain their existing contract. This transition does not authorize rewriting
  an existing stored file, preserving old identities through compatibility code,
  or silently accepting an old identity as a valid new identity.
- Transition accounting: the before/after digest replay must enumerate the
  transition population separately and list each observed old/new identity,
  path and affected citation. Unlisted differences are failures; an empty
  transition population does not imply absence of records outside the stated
  inventory. Every reachable record remains in the reported replay population.
- Operational context: each cutover change, each revert of a cutover, each
  deletion change, and the corpus consolidation.
- Not applied to: the contents of the `corpus/` submodule, which belong to
  `agent-ix/qa-corpus` and are read rather than guaranteed by this repository;
  and generated build output, which carries its own provenance under FR-097.

## Rationale

Quoin's digests are identifiers, not checksums. A record cites another record by
digest, and a change-assurance verdict is a statement about a digest. If a
canonicalization difference moves a digest, the citing records do not become
wrong in a way a test reports — they become unresolvable, and the damage is not
recoverable by re-running anything. Outside the explicitly enumerated PLAT-989
transition, digest agreement is therefore a hard gate before a store-backed
cutover. PLAT-989 deliberately changes the assurance identity rule with operator
approval; its changed identities and references are disclosed instead of being
claimed as parity or hidden by a legacy-preservation mechanism.

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
| Unlisted record identity changes across a cutover (outside the enumerated PLAT-989 transition) | 0 | 0 | Test |
| Existing evidence-file bytes changed by a cutover | 0 | 0 | Test |
| Accepted-corpus bytes changed across the port | 0 | 0 | Test |
| Stores whose read-and-re-serialize round trip is not byte-identical | 0 | 0 | Test |
| Digest-replay population covered before a store-backed cutover | 100% of reachable records | 100% | Test |

## Verification

Before each store-backed cutover, the digest replay enumerates every record in
every reachable store, computes its digest through the retained and the Rust
implementations, and reports the full non-empty population it covered. It
asserts zero mismatches outside the enumerated PLAT-989 assurance transition;
for that transition it reports each observed old/new identity, path and affected
citation and refuses any unlisted difference. A separate round-trip check reads
and re-serializes every stored record without replacing its identity fields and
asserts byte identity; it does not mint a new record or rewrite an old file.
Across each cutover, revert and deletion change, a tree comparison over the
evidence and accepted-corpus roots asserts zero changed bytes. A planted
single-byte difference outside the exact transition mapping must fail the
replay, and a replay whose population is zero is reported as inconclusive rather
than as clean.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-025-AC-1 | A digest replay over every reachable store names its non-empty full population, reports zero mismatches outside the enumerated PLAT-989 assurance identity transition, and separately lists each observed transition identity/path/citation with no unlisted difference. | Test (TC-1665) |
| NFR-025-AC-2 | A planted single-byte canonicalization difference outside the exact enumerated PLAT-989 transition mapping makes the digest replay fail. | Test (TC-1666) |
| NFR-025-AC-3 | Reading and re-serializing every store without regenerating identity fields returns byte-identical records, including historical assurance records. | Test (TC-1667) |
| NFR-025-AC-4 | Comparing the evidence and accepted-corpus trees before and after a cutover, after its revert, and after the deletion change finds zero changed bytes. | Test (TC-1668) |
| NFR-025-AC-5 | A digest replay whose population is zero, or whose store inventory is absent, is reported as inconclusive and does not satisfy the cutover gate. | Test (TC-1669) |
| NFR-025-AC-6 | For the same input over a named non-empty population, newly written records are byte-identical to retained-writer output except for the explicitly enumerated PLAT-989 assurance recordId values, derived filenames and changed-identity references; every other byte remains equal and all transition differences are reported. | Test (TC-1704) |

## Dependencies

- **Upstream**: [FR-098](../functional/FR-098-semantic-and-identity-parity.md) and [FR-100](../functional/FR-100-rust-evidence-measurement-change-assurance.md), which implement the single canonicalization and digest path.
- **Downstream**: [FR-101](../functional/FR-101-retire-replaced-executable-paths.md) and [FR-103](../functional/FR-103-corpus-consolidation.md), whose cutovers and consolidation this bounds.

> **PLAT-989 authorization (2026-10-05):** The operator approved the bounded
> identity transition stated in Scope and reflected in the metrics, replay and
> retained-writer criteria above. Legacy identities and citations need not be
> preserved. Corruption detection, canonical integrity verification and the
> prohibition on modifying existing evidence or governed corpus bytes remain
> in force.
