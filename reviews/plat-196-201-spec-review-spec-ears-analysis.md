---
id: SR-16395
title: EARS conformance review of FR-121 and FR-122
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# EARS conformance review of FR-121 and FR-122

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

Checked the FR statements and Behavior bullets for EARS form. Both Descriptions and most Behavior bullets conform (ubiquitous, event 'When', unwanted 'If ... then'). One low finding.

## Scope

- `FR-121` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-121#heading-rule` (examined) — spec/functional/FR-121-record-key-sections-mapping.md
- `FR-122#id-list-keys` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122#contract-fence` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md
- `FR-122#layout-size` (examined) — spec/functional/FR-122-architecture-record-keys-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several FR-121 Behavior bullets take the data as subject ('A key whose property is an object SHALL be authored as one table', 'A header cell SHALL match exactly one property') rather than stating a Quire response. | spec/functional/FR-121-record-key-sections-mapping.md:78-81 |

## Details

- **FND-001** (FR-121): The violation responses exist in later bullets, so behaviour is recoverable, but the bullets read as authoring rules. EARS form: 'Quire SHALL read a key whose property is an object from one table with one data row'.


## Verdict

No defects for this lens.

## Dispositions

Round 1, against head 59833eb2.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |

FR-121 lines 81-84 now state Quire responses. Lines 97-100 (`A cell authored ...`, `An empty cell SHALL omit`, `A property typed as an object ... SHALL NOT be authored`) still take the data as subject; they were not in the finding's cited range and are left as is.
