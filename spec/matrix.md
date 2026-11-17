---
id: TM-001
title: "Default module release pin coverage"
type: TestMatrix
relationships:
  - target: "ix://agent-ix/quoin/FR-016"
    type: "covers"
  - target: "ix://agent-ix/quoin/FR-202"
    type: "covers"
---

# TM-001: Default module release pin coverage

## Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| -------------- | ------------------- | ---------- | ------ |
| FR-202 | FR-202-AC-1, FR-202-AC-2, FR-202-AC-3, FR-202-AC-4, FR-202-AC-5, FR-202-AC-6, FR-202-AC-7, FR-202-AC-8, FR-202-AC-9, FR-202-AC-10, FR-202-AC-11, FR-202-AC-12, FR-202-AC-13, FR-202-AC-14, FR-202-AC-15, FR-202-AC-16 | TC-601, TC-602, TC-603, TC-604, TC-605, TC-606, TC-607, TC-608, TC-609, TC-610, TC-611, TC-612, TC-613, TC-614, TC-615, TC-616 | ✅ |

## Test Case Summary

| Test ID | Title | Type | Traces To | Status |
|---------|-------|------|-----------|--------|
| TC-014 | Default module manifest pins all ten public modules to release refs | Unit | FR-016-AC-1, FR-016-AC-2, FR-016-AC-4, FR-016-AC-5 | ✅ |
| TC-601 | FR-202 AC-1 threat-model dossier behavior | Integration | FR-202-AC-1 | ✅ |
| TC-602 | FR-202 AC-2 threat-model dossier behavior | Integration | FR-202-AC-2 | ✅ |
| TC-603 | FR-202 AC-3 threat-model dossier behavior | Integration | FR-202-AC-3 | ✅ |
| TC-604 | FR-202 AC-4 threat-model dossier behavior | Integration | FR-202-AC-4 | ✅ |
| TC-605 | FR-202 AC-5 threat-model dossier behavior | Integration | FR-202-AC-5 | ✅ |
| TC-606 | FR-202 AC-6 threat-model dossier behavior | Integration | FR-202-AC-6 | ✅ |
| TC-607 | FR-202 AC-7 threat-model dossier behavior | Integration | FR-202-AC-7 | ✅ |
| TC-608 | FR-202 AC-8 threat-model dossier behavior | Integration | FR-202-AC-8 | ✅ |
| TC-609 | FR-202 AC-9 threat-model dossier behavior | Integration | FR-202-AC-9 | ✅ |
| TC-610 | FR-202 AC-10 threat-model dossier behavior | Integration | FR-202-AC-10 | ✅ |
| TC-611 | FR-202 AC-11 threat-model dossier behavior | Integration | FR-202-AC-11 | ✅ |
| TC-612 | FR-202 AC-12 threat-model dossier behavior | Integration | FR-202-AC-12 | ✅ |
| TC-613 | FR-202 AC-13 threat-model dossier behavior | Integration | FR-202-AC-13 | ✅ |
| TC-614 | FR-202 AC-14 threat-model dossier ordering | Integration | FR-202-AC-14 | ✅ |
| TC-615 | FR-202 AC-15 threat-model dossier behavior | Integration | FR-202-AC-15 | ✅ |
| TC-616 | FR-202 AC-16 threat-model dossier behavior | Integration | FR-202-AC-16 | ✅ |
