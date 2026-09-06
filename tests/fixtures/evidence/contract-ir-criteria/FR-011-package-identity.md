---
id: FR-011
title: "Identify versioned contract packages and requirement revisions"
type: FR
relationships:
  - target: ix://agent-ix/quire-contract-ir/StR-001
    type: traces_to
---
# FR-011: Identify versioned contract packages and requirement revisions

## Description

The model shall identify a contract package, its schema version, every
requirement ID and revision, and the source document revision from which it was
derived.

## Inputs

Package namespace, schema major/minor, requirement identifier, positive current
requirement revision, and source-document identity including its exact revision.

## Outputs

Validated package and requirement identities suitable for serialization and
downstream citation.

## Behavior

Issue #6 introduced wire schema version `1.0`; FR-017 makes `1.1` current and
retains `1.0` only as the source of its registered migration. Crate version
`0.1.0` is independent. This requirement does not freeze the current schema
version or authorize an unregistered migration.
Identity validation rejects empty namespaces, malformed identifiers, a zero
schema major, zero source or requirement revisions, duplicate requirement identifiers, and
references to a different package. Changing a requirement revision changes every derived downstream
identity that cites it. A package contains one current positive revision per
requirement. Revision gaps are legal; when a caller advances a known prior
revision, the replacement shall be strictly greater than the prior value.

Issue #6 owns an implementation-language-independent, non-canonical JSON value
representation for its public identities and package structure. A valid value
round trips with structural equality. JSON member order and byte spelling are
explicitly non-normative here; FR-016 later owns canonical bytes and digests,
and FR-018/FR-020 later publish the complete schema and conformance interface.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-011-AC-1 | A package round trip through the issue #6 non-canonical JSON value representation preserves namespace, schema version, requirement ID, requirement revision, and source-document identity/revision with structural equality. | Test (TC-015) |
| FR-011-AC-2 | Incrementing one requirement revision changes its clause and dependency identities without changing unrelated requirement identities. | Test (TC-015) |
| FR-011-AC-3 | Empty or malformed package, source-document, requirement, clause, anchor, or dependency-path-segment identities; a zero schema major; zero source or requirement revisions; duplicate requirement or clause identifiers; non-increasing revision advances; and cross-package references fail with their registered structured diagnostic codes. Schema minor zero is valid. | Test (TC-015) |

## Dependencies

PGM-01-R01 governs compatibility and migration behavior.
