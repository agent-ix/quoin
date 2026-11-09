---
id: SR-16397
title: scope-boundary review of FR-121 and FR-122 record-key mappings
type: SpecReview
analysis: scope-boundary
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# scope-boundary review of FR-121 and FR-122 record-key mappings

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

Checked who owns each artifact FR-121/FR-122 create or depend on: fixtures, the quire-rs output schema, and the architecture keys claimed. Three defects.

## Scope

- `FR-121-CON-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122-CON-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-121#outputs` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `matrix#FR-121` (examined) — spec/matrix.md
- `matrix#FR-122` (examined) — spec/matrix.md
- `FR-121` (context_only) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122` (context_only) — spec/functional/FR-122-architecture-record-keys-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-121-CON-2 and FR-122-CON-2 make Quoin publish new golden fixtures under tests/fixtures/semantic-module/mapping/, which open PR #777 deletes as a divergent copy (FR-071-CON-2 becomes 'read from quire-fixtures, holding no copy') and whose ownership quire-rs CR-182 already moved to the quire-fixtures crate. | spec/functional/FR-121-record-key-sections-mapping.md:110 |
| FND-002 | medium | FR-121 adds output keys recordSources and availability.recordKeys, but quire-rs schemas/output/semantic-v1.schema.json closes availability (additionalProperties: false, kindAvailability per kind) and has no recordSources; no Dependency names that schema change. | spec/functional/FR-121-record-key-sections-mapping.md:59-61 |
| FND-003 | medium | FR-122's Description claims every key spec-objects-architecture declares beyond fields/operations/clauses/relations, but it omits featureOrder and the systems-kind members (owner, declaredType, sourceEnd, ...), owned by quire-rs FR-075 (#446/#448), and it re-owns the FR-076 typed relation keys. | spec/functional/FR-122-architecture-record-keys-mapping.md:20 |

## Details

- **FND-001** (FR-121-CON-2): quire-rs FR-076 (origin/main) records the relationships fixtures as relocated to quire-rs's quire-fixtures crate with ownership transferred (CR-182). quoin PR #777 (PLAT-1364) removes quoin's tests/fixtures/semantic-module/ and rewrites FR-071-CON-2 to read from quire-fixtures. This PR adds two constraints and fifteen matrix rows ('fixture to be published') that re-create a Quoin-owned fixture tree for the same mapping surface: a second copy, against the no-vendoring rule. Fix: fixtures authored in quire-rs quire-fixtures; Quoin reads them (or holds none); restate CON-2 and the matrix notes to match #777.

- **FND-002** (FR-121): availability.recordKeys is also a per-key map, a different shape from the per-kind kindAvailability entries FR-072/FR-104 use. The quire-rs output schema change belongs in FR-121's Downstream (and its owner), otherwise the first extractor run fails the record compatibility check that quire-rs FR-076-AC-9 applies to every record.

- **FND-003** (FR-122): Interface.json requires featureOrder; part/port/connection/allocation require owner, declaredType, multiplicity, direction, interfaceType, sourceEnd, targetEnd, flowDirection, sourceElement, targetElement. spec-objects-architecture FR-007 routes these through quire-rs FR-075 table lowering. Fix: narrow the Description to the keys FR-122 actually owns and name the other owners.


## Verdict

Not mergeable until the high findings are fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-122's Downstream list of spec-objects-architecture changes omits naming the relationships token in the module manifest (today: typed-table, sysml-fence, ocl-clause, generalization), which FR-122 and quire-rs FR-076 require before any typed relation key is produced. | spec/functional/FR-122-architecture-record-keys-mapping.md:107 |

## Dispositions

Round 1, against head 59833eb2.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 2, against head 497d1fe5.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | - |
