---
id: TM-002
title: "Formalize workflow coverage"
type: TestMatrix
relationships:
  - target: "ix://agent-ix/quoin/FR-189"
    type: "covers"
---

# TM-002: Formalize workflow coverage

## Test Case Summary

| Test ID | Title | Type | Traces To | Status |
|---------|-------|------|-----------|--------|
| TC-015 | A failed QSL check returns to drafting and records its diagnostic for retry | Integration | FR-189-AC-2 | ✅ |

## Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-189 | FR-189-AC-2 | TC-015 | ✅ Covered |
