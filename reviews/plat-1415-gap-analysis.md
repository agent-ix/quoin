---
id: SR-20261112024906
title: "Gap analysis of quoin PR 829 against FR-136"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@91a5f857; spec/functional/FR-136-anomaly-record-mapping.md, rust/crates/quoin-anomaly/**, rust/crates/quoin-cli/src/anomaly.rs, rust/crates/quoin-cli/tests/tc_136_anomaly_command.rs; context spec-artifacts-process#140 FR-014, FR-015, FR-011"
review_set: subset
---

# Gap analysis of quoin PR 829 against FR-136

## Summary

Ticket: PLAT-1415 (with PLAT-1416). This is a planless audit: plan completion was not assessed. Every acceptance criterion from FR-136-AC-1 to AC-12 was checked against the tests tagged to it and against the code. The audit also covered the Behavior statements of FR-136 and the spec-artifacts-process FR-014/FR-015 obligations that FR-136 owns: the legal-move set, transition legality and the one missing-module diagnostic per document. Fixtures came from spec-artifacts-process#140 at aa3ef43.

## Verdict

**PASS WITH FINDINGS (four medium, three low).** The following match the code: AC-4 (pending entry), AC-5 (reassessment), AC-7 (refs, including the `[]` JSON), AC-9 (no fixture copies in the repo) and AC-12 (signature letters). The FR-014 legal-move set is exact. Every FR-136 AC except AC-9 (Inspection) has a tagged test. The gaps below are coverage gaps or behaviour gaps; two of them hide the high defects in code review SR-20261112024905.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-11 is only partly covered. The AC-11 test asserts code, severity, reason and status line only for `illegal-transition` and `missing-module`. It never checks `anomaly.no-record` (AC-2) or `anomaly.reassessment-missing` (AC-5), and the no-record line is in fact wrong (SR-20261112024905 FND-002) | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:398-420 |
| FND-002 | medium | AC-1 and AC-3 to AC-5 only test the root layout. Every history test commits `AN-001.md` at the repository root. No fixture repository places the anomaly in a subdirectory, as every real `spec/` layout does, so the empty-history defect passes the suite (SR-20261112024905 FND-001) | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:99-121 |
| FND-003 | medium | "Report not applicable" is never surfaced. FR-136 says "If the file is not under git, then Quoin SHALL report `statusHistory` not applicable". `quoin anomaly check` prints nothing in that case, and no command exposes `statusHistory` at all. AC-6 is met only at the library level (`read_history` returns `NotApplicable`); a user cannot see it | rust/crates/quoin-anomaly/src/check.rs:205-221; rust/crates/quoin-cli/src/anomaly.rs:78-109 |
| FND-004 | medium | FR-136 Description: "Quoin repeats none of that mapping". The code repeats it, together with the vocabularies, and the code carries no marker or ticket for the quire-rs#393 stand-in. Spec and code disagree, and the spec has no recorded exception (detail in SR-20261112024905 FND-004) | rust/crates/quoin-anomaly/src/record.rs:4-17 |
| FND-005 | low | AC-3's not-emitted list names `assessed` to `closed` with `wont-fix` and `identified` to `closed` with `duplicate`. Every pair runs on the `closed-fixed` base, so the `closed-wont-fix` and `closed-duplicate` fixtures are never used | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:228-256 |
| FND-006 | low | Two assertions are weak. The first loop of the AC-8 test, `all(\|r\| r.impact != "Affects" \|\| id != "FR-999")`, is true for FR-001 and HAZ-001 whatever the code does. The AC-1 typed-author test injects `date: 1999-01-01` but never asserts that `authored_at` ignores it | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:189-200,368-376 |
| FND-007 | low | The second clause of AC-10, "and `quire validate` still reports its own per-row diagnostics", is asserted by neither AC-10-tagged test | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:382-396; rust/crates/quoin-cli/tests/tc_136_anomaly_command.rs:76-107 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136-AC-1, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "four statusHistory entries in order, each with author, authored date, signature and signatureCode, rename keeps entries; no typed author or date read"}
- {id: FR-136-AC-2, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "schema-only failure still yields record, no field diagnostic; mapping error yields anomaly.no-record and statusHistory still checked"}
- {id: FR-136-AC-3, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "illegal-transition emitted/not emitted by reachability over fixture histories"}
- {id: FR-136-AC-4, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "first entry of any status legal; pending final entry judged"}
- {id: FR-136-AC-5, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "resolved, assessed, resolved with unchanged assessment yields reassessment-missing naming the reopen commit"}
- {id: FR-136-AC-6, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "file outside git reports statusHistory not applicable and no transition diagnostic"}
- {id: FR-136-AC-7, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "refs on FR-001, FR-001-AC-1 and a hazard; [] for unnamed hazard; None row in no listing"}
- {id: FR-136-AC-8, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Requirement row targeting FR-999 absent from every listing; no dangling-reference diagnostic"}
- {id: FR-136-AC-9, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "documents are those shipped by spec-artifacts-process; no copy in quoin (Inspection: passes, fixtures read in place)"}
- {id: FR-136-AC-10, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "two Hazard rows, module missing: exactly one anomaly.missing-module; quire validate still reports per-row"}
- {id: FR-136-AC-11, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "every diagnostic in AC-2, AC-3, AC-5, AC-10 carries code, severity error, reason, status line"}
- {id: FR-136-AC-12, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "G,U,X,Y signed; N unsigned; B,R,E unknown"}
- {id: FR-014, path: "spec-artifacts-process spec/functional/FR-014-anomaly-archetype-and-state-lifecycle.md", role: context_only, excerpt: "single legal moves and reachability over squashed history, owned by quoin FR-136"}
- {id: FR-015, path: "spec-artifacts-process spec/functional/FR-015-anomaly-safety-impact-assessment.md", role: context_only, excerpt: "one missing-module diagnostic per document naming spec-objects-safety"}
```
