---
id: SR-20261123115004
title: "Spec review (integrity) — FR-167 required snapshot members (quoin#1023)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
---

## Summary

Ticket: PLAT-1796. Integrity review of the FR-167 edit in PR #1023 at
bbbf48096d95c1401654513cd756f89c56821d9a. `quire validate` on the file exits 0.

The statement, member table, Behavior and AC-13 agree with each other and with the code:
both members are required, a reader refuses either one missing or `null`, and the
`not_assessed` kind list drops `scan_unordered` in the prose, the Behavior SHALL and AC-13.
No backward-compatibility wording remains in FR-167, FR-166 or FR-201.

## Verdict

**PASS WITH FINDINGS** — one writer behaviour is stated only in prose.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | "a scan with an invalid retained timestamp refuses the snapshot" is prose with no SHALL statement or AC, and FR-166 names no refusal code for it, so the writer's exit class and code are unspecified (the code currently exits 1 as an operational failure). | spec/functional/FR-167-release-snapshot-format.md:52-53; spec/functional/FR-166-release-snapshot-command.md:83-94 |
