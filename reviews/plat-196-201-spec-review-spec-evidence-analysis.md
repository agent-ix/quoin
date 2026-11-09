---
id: SR-16396
title: evidence review of FR-121 and FR-122 acceptance criteria
type: SpecReview
analysis: evidence
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# evidence review of FR-121 and FR-122 acceptance criteria

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

Checked each AC and CON for a verification method that can fail and a matrix row. All 15 ACs have a TC (TC-2013..TC-2027) and an audit row; CONs use non-Test methods and are rightly absent from the AC/TC table. Three defects in the criteria themselves.

## Scope

- `FR-121-AC-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-4` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-5` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-6` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-7` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-AC-8` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-1` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-2` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-121-CON-3` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122-AC-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-3` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-4` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-5` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-6` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-AC-7` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-1` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122-CON-2` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `matrix#TC-2027` (examined) — spec/matrix.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-122-AC-7 checks the FR-122 table against itself ('every key of the table above ... is mapped by a row of this requirement'), so it cannot fail; it must enumerate the keys of the module's emitted schemas. | spec/functional/FR-122-architecture-record-keys-mapping.md:87 |
| FND-002 | medium | FR-122-AC-6's 'an empty Status is refused as an empty required cell' names no code or reason, and FR-121 has no such refusal (an empty cell omits the field; required-ness is left to record validation). | spec/functional/FR-122-architecture-record-keys-mapping.md:86 |
| FND-003 | low | FR-121-AC-4's fixture (## Status and ## Provenance only) cannot validate against Hazard.json, which requires fields and, when status is accepted, assessment as well as provenance; the AC should say the fixture also carries ## Properties and ## Assessment. | spec/functional/FR-121-record-key-sections-mapping.md:120 |

## Details

- **FND-001** (FR-122-AC-7): As written, a key missing from both the table and the schema check passes. A failable form: every top-level property of each spec-objects-architecture object-type schema is mapped by FR-071/072/104/121/122 or a named quire-rs FR-075/FR-076 owner, and none is declared not-authored. The AC is Inspection yet carries TC-2027 (Static) in the AC/TC table, whose heading excludes non-test methods; `quire matrix` reports it `method-without-symbol`.

- **FND-002** (FR-122-AC-6): A test cannot assert this clause: there is no diagnostic code, reason or locus. Either define the refusal in FR-122 Behavior (code, reason, locus) or restate it as 'on_exceeded has no status and the record fails Threshold.json validation'.

- **FND-003** (FR-121-AC-4): spec-objects-safety origin/main Hazard.json: required [fields]; allOf if status=accepted then required [provenance, assessment].


## Verdict

Not mergeable until the high findings are fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | TC-2028 says it is executed by the quire-rs extractor against goldens, but FR-122-AC-7 is an ownership audit over the architecture schemas that no extraction run performs; the row should name what checks it. | spec/matrix.md:1418 |

## Dispositions

Round 1, against head 59833eb2.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
