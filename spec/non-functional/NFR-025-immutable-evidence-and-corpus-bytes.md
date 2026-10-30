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
identifiers are preserved unchanged.

## Scope

- Applies to: every record under the Quoin evidence store root — evidence,
  measurement, intervention, operational and change-assurance records — together
  with each store root named in the checked-in store inventory, and every
  accepted-corpus byte reachable from the governed corpus. An absent store
  inventory refuses the read-back rather than narrowing its population.
- Record identity: a record takes an opaque UUID (v4) `recordId` and a
  `<recordId>.json` name, not an identifier derived from content. Existing
  identifiers are read as opaque strings and no existing stored file is
  rewritten.
- Operational context: each cutover change, each revert of a cutover, each
  deletion change, and the corpus consolidation.
- Not applied to: the contents of the `corpus/` submodule, which belong to
  `agent-ix/qa-corpus` and are read rather than guaranteed by this repository;
  and generated build output, which carries its own provenance.

## Rationale

A record cites another record by its opaque identifier. Because no identifier
is derived from content, a serialization change cannot orphan a citation; what
the port must not do is rewrite a stored byte, and git, which addresses
content, shows any such rewrite as a diff. Byte agreement of a read-back is a hard
gate before a store-backed cutover.

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
| Existing record identity changes across a cutover | 0 | 0 | Test |
| Existing evidence-file bytes changed by a cutover | 0 | 0 | Test |
| Accepted-corpus bytes changed across the port | 0 | 0 | Test |
| Stores whose read-and-re-serialize round trip is not byte-identical | 0 | 0 | Test |
| Read-back population covered before a store-backed cutover | 100% of reachable records | 100% | Test |

## Verification

Before each store-backed cutover, a read-back enumerates every record in every
reachable store, reports the full non-empty population it covered, and asserts
zero changed bytes. A separate round-trip check reads and re-serializes every
stored record without replacing its identity fields and asserts byte identity;
it does not mint a new record or rewrite an old file. Across each cutover,
revert and deletion change, a tree comparison over the evidence and
accepted-corpus roots asserts zero changed bytes. A planted single-byte change
must fail the read-back, and a read-back whose population is zero is reported as
inconclusive rather than as clean.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-025-AC-1 | A read-back over every reachable store names its non-empty full population and reports zero changed bytes. | Test (TC-1665) |
| NFR-025-AC-2 | A planted single-byte change in a stored record makes the read-back fail. | Test (TC-1666) |
| NFR-025-AC-3 | Reading and re-serializing every store without regenerating identity fields returns byte-identical records, including historical assurance records. | Test (TC-1667) |
| NFR-025-AC-4 | Comparing the evidence and accepted-corpus trees before and after a cutover, after its revert, and after the deletion change finds zero changed bytes. | Test (TC-1668) |
| NFR-025-AC-5 | A read-back whose population is zero, or whose store inventory is absent, is reported as inconclusive. | Test (TC-1669) |
| NFR-025-AC-6 | Withdrawn: there is no retained writer whose output new records could be compared with; new records are checked by behavior under FR-063 and FR-064 (id not reused). | Withdrawn (TC-1704) |

## Dependencies

- **Upstream**: [FR-098](../functional/FR-098-semantic-and-identity-parity.md) and [FR-100](../functional/FR-100-rust-evidence-measurement-change-assurance.md), which implement the single canonicalization path.
- **Downstream**: [FR-103](../functional/FR-103-corpus-consolidation.md), whose consolidation this bounds.
