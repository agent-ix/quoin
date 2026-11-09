---
id: SR-16398
title: gap analysis of PR 778 (FR-121, FR-122)
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-122-architecture-record-keys-mapping.md;
  spec/matrix.md; spec/functional/index.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# gap analysis of PR 778 (FR-121, FR-122)

## Summary

Ticket: PLAT-196, PLAT-201 (agent-ix/quoin PR #778, head 810bc55e).

Planless. Plan completion: not assessed. Spec-only diff, no production code. `quire matrix` lists all 15 ACs and 5 CONs untagged (FR-122-AC-7 method-without-symbol), as expected for Pending rows. One defect.

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
- `matrix#FR-121` (examined) — spec/matrix.md
- `matrix#FR-122` (examined) — spec/matrix.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Pending rows say the extractor 'lives in agent-ix/quire-rs' but name no quire-rs requirement, ticket or test that will execute TC-2013..TC-2027, unlike the FR-072/FR-104 Delegated rows, so nothing says how these rows become covered. | spec/matrix.md:143 |

## Details

- **FND-001** (matrix#FR-121): Pending is the right status today. Name the quire-rs owner (ticket or FR) in the rows so the delegation is traceable.


## Verdict

No defects for this lens.
