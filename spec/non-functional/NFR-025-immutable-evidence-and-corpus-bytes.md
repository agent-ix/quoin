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
  - target: "ix://agent-ix/quoin/FR-103"
    type: "constrains"
---

# NFR-025: Evidence and accepted-corpus bytes are immutable across the port

## Statement

Quoin SHALL leave every existing stored evidence byte and every accepted-corpus
byte unchanged across the port, including cutovers, reverts and deletion changes.
Record identifiers are opaque and are not derived from content; existing
identifiers are preserved unchanged, and the PLAT-989 identity transition below
is the only bounded exception.

## Scope

- Applies to: every record under the Quoin evidence store root — evidence,
  measurement, intervention, operational and change-assurance records — together
  with each store root named in the checked-in store inventory, and every
  accepted-corpus byte reachable from the governed corpus pins. An absent store
  inventory refuses the replay rather than narrowing its population.
- Applies to: records the Rust implementation newly writes, which must be
  byte-identical to what the retained implementation would have written for the
  same input, except for the enumerated PLAT-989 transition outputs below.
- Identity transition: newly written experiment and operational assurance
  records take an opaque UUID (v7) `recordId` and a `<recordId>.json` name, not
  an identifier derived from content. Only the resulting `recordId` values,
  basenames and references to those identifiers are exempt from retained-writer
  byte parity. Record schemas and all other fields retain their existing
  contract. This transition does not authorize rewriting an existing stored
  file; existing identifiers are read as opaque strings.
- Transition accounting: the before/after replay must enumerate the transition
  population separately and list each observed path and affected citation.
  Unlisted differences are failures; every reachable record remains in the
  reported replay population.
- Operational context: each cutover change, each revert of a cutover, each
  deletion change, and the corpus consolidation.
- Not applied to: the contents of the `corpus/` submodule, which belong to
  `agent-ix/qa-corpus` and are read rather than guaranteed by this repository;
  and generated build output, which carries its own provenance.

## Rationale

A record cites another record by its opaque identifier. Because no identifier
is derived from content, a serialization change cannot orphan a citation; what
the port must not do is rewrite a stored byte, and git, which addresses
content, shows any such rewrite as a diff. Byte agreement of a replay is a hard
gate before a store-backed cutover, outside the enumerated PLAT-989 transition.

The accepted corpus has the same property for a different reason: it is the
population that every measurement figure in this repository is quoted against.
Rewriting a corpus byte silently redefines every historical rate without
invalidating the report that carries it.

The guarantee must hold in both directions. A cutover that is reverted must
restore the retained invocation without having left a rewritten byte behind, or
a claimed reversibility is not reversibility.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Unlisted record identity changes across a cutover (outside the enumerated PLAT-989 transition) | 0 | 0 | Test |
| Existing evidence-file bytes changed by a cutover | 0 | 0 | Test |
| Accepted-corpus bytes changed across the port | 0 | 0 | Test |
| Stores whose read-and-re-serialize round trip is not byte-identical | 0 | 0 | Test |
| Record-replay population covered before a store-backed cutover | 100% of reachable records | 100% | Test |

## Verification

Before each store-backed cutover, the replay enumerates every record in every
reachable store, writes it back through the retained and the Rust
implementations, compares the two outputs byte for byte, and reports the full non-empty population it covered. It
asserts zero differences outside the enumerated PLAT-989 assurance transition;
for that transition it reports each observed path and affected citation and refuses any unlisted difference. A separate round-trip check reads
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
| NFR-025-AC-1 | A record replay over every reachable store names its non-empty full population, reports zero byte differences outside the enumerated PLAT-989 transition, and separately lists each observed transition path/citation with no unlisted difference. | Test (TC-1665) |
| NFR-025-AC-2 | A planted single-byte difference outside the exact enumerated transition mapping makes the replay fail. | Test (TC-1666) |
| NFR-025-AC-3 | Reading and re-serializing every store without regenerating identity fields returns byte-identical records, including historical assurance records. | Test (TC-1667) |
| NFR-025-AC-4 | Comparing the evidence and accepted-corpus trees before and after a cutover, after its revert, and after the deletion change finds zero changed bytes. | Test (TC-1668) |
| NFR-025-AC-5 | A replay whose population is zero, or whose store inventory is absent, is reported as inconclusive and does not satisfy the cutover gate. | Test (TC-1669) |
| NFR-025-AC-6 | For the same input over a named non-empty population, newly written records are byte-identical to retained-writer output except for the explicitly enumerated assurance recordId values, derived filenames and references to them; every other byte remains equal and all transition differences are reported. | Test (TC-1704) |

## Dependencies

- **Upstream**: [FR-098](../functional/FR-098-semantic-and-identity-parity.md) and [FR-100](../functional/FR-100-rust-evidence-measurement-change-assurance.md), which implement the single canonicalization path.
- **Downstream**: [FR-103](../functional/FR-103-corpus-consolidation.md), whose consolidation this bounds.

> **PLAT-989 authorization (2026-10-05):** The operator approved the bounded
> identity transition stated in Scope and reflected in the metrics, replay and
> retained-writer criteria above. Legacy identities are left as they are and read as opaque strings. The
> prohibition on modifying existing evidence or governed corpus bytes remain
> in force.
