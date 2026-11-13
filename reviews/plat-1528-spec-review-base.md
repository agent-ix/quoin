---
id: SR-20261113083116
title: "Spec review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #857 (git diff origin/main...plat-1528-spec-quoin at 56f23309): spec/functional/FR-162, FR-163, FR-164, spec/non-functional/NFR-035 and both index edits"
---

# Spec review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)

## Summary

Ticket: PLAT-1528. Method: spec-review/base, scoped to `git diff origin/main...HEAD` of agent-ix/quoin#857 at 56f23309.
Read against quoin FR-136 and FR-147 as merged, spec-artifacts-process FR-014 and FR-015 on main, and FR-016 from
the companion PR agent-ix/spec-artifacts-process#147.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The selection drops every closed anomaly whose resolution is wont-fix. spec-artifacts-process FR-014 makes assessed->closed and identified->closed with wont-fix the intended path for a defect deliberately not fixed, so a known defect that ships with the release vanishes from the report. That is the core content the FDA unresolved-anomalies list asks for (anomalies remaining in the released software and their effect on safety). FR-016 (companion PR) also forbids a Release Rationale on a closed anomaly, so such an anomaly can never carry the named rationale FR-163 reports. List closed anomalies whose resolution is wont-fix (with a matching FR-016 change to admit the section on closed wont-fix), or require wont-fix anomalies to stay open, stated in both repos. | spec/functional/FR-162-anomaly-report.md:46 |
| FND-002 | high | The unjustified-impacting gate is bypassed by closing the anomaly as wont-fix: an impacting defect that ships unfixed and has no rationale passes once its status is closed (FR-164 also says a closed anomaly never fails, whatever it carries, and AC-5 tests exactly that pass). Under FR-014 closing as wont-fix is legal from identified or assessed, so the gate checks the case least in need of it and lets through the case most in need. Fail unjustified-impacting for closed wont-fix impacting anomalies too (with the FR-016 change in FND-001), or add a mode that does. | spec/functional/FR-164-anomaly-report-gate.md:31 |
| FND-003 | medium | No valid fixture can set this up, and the invalid one that can is never reported. FR-014 forbids closed_by unless status is closed; FR-016 forbids the Release Rationale section when status is closed. A document carrying both is invalid, and if it is closed with a non-unassessed safety_impact FR-162 does not list it, so the row whose author AC-4 checks does not exist. Only a closed, unassessed, doubly invalid document reaches the report, and FR-162-AC-12 requires the module's shipped documents, which include no such fixture. Restate AC-4 against what a reachable row can carry (for example a commit author that differs from the typed author), or name the negative fixture the module must ship. | spec/functional/FR-163-anomaly-report-release-rationale.md:59 |
| FND-004 | medium | AC-1 compares output to the document's bytes, including a leading space, but the value the report reads is the record's typed-table cell (FR-016: rows mapped as authored through FR-011), and a Markdown table cell's surrounding whitespace is cell padding, not content. Whether a leading space survives is the extractor's behaviour, not quoin's, so the AC either tests the extractor or cannot be met. Compare against the record value (byte-equal to releaseRationale as extracted), and keep the case-preservation check. | spec/functional/FR-163-anomaly-report-release-rationale.md:56 |
| FND-005 | low | The title and Description call the listed set 'unresolved', but the selection includes anomalies at status resolved (fix landed, not closed); AC-1 lists one. A reader of the dossier would read 'unresolved' as excluding them. Name the set by its rule (not closed, or unassessed) or state that resolved-not-closed anomalies are included and why. | spec/functional/FR-162-anomaly-report.md:20 |
| FND-006 | low | The gate repeats half of FR-016's row check (non-empty after trimming) while saying it repeats none of it, and skips the other half (placeholders), so a `TBD`/`TBD` row passes the gate but fails quire validate. Either count any entry the record carries (leaving all shape checks to quire validate), or state that the gate is only meaningful after quire validate passes. | spec/functional/FR-164-anomaly-report-gate.md:32 |

## Verdict

Changes requested: two high findings share one root cause (closed wont-fix anomalies are invisible to the report and the gate), plus two medium AC defects in FR-163.

Checked and clean: the tool never writes, suggests or defaults a rationale or verdict (FR-163 Never written, FR-164 empty-failures wording, NFR-035 AC-2/AC-3) and no 'acceptable' wording is emitted; 'open' (status not closed) matches FR-147's definition exactly, and the extra closed-unassessed rows are a separate, stated selection rule, not a redefinition; the command is read-only and deterministic by statement and AC; the JSON top-level keys and gate shape are fixed for the dossier export; the grouping inputs (assessment kind/target/impact) exist in the FR-015 record mapping on main; fixingChange is taken as FR-136 derives it, matching FR-136's 'a reader of that output, including the later unresolved-anomaly report, takes the value from there'; FDA guidance is cited generically only; no compatibility layer; no hash, pin or digest ceremony (signature and signatureCode are git-derived attribution, not pins).

Examined (role examined): FR-162-AC-1, FR-162-AC-2, FR-162-AC-3, FR-162-AC-4, FR-162-AC-5, FR-162-AC-6, FR-162-AC-7, FR-162-AC-8, FR-162-AC-9, FR-162-AC-10, FR-162-AC-11, FR-162-AC-12, FR-162-AC-13, FR-163-AC-1, FR-163-AC-2, FR-163-AC-3, FR-163-AC-4, FR-163-AC-5, FR-163-AC-6, FR-163-AC-7, FR-163-AC-8, FR-163-AC-9, FR-164-AC-1, FR-164-AC-2, FR-164-AC-3, FR-164-AC-4, FR-164-AC-5, FR-164-AC-6, FR-164-AC-7, FR-164-AC-8, FR-164-AC-9, FR-164-AC-10, FR-164-AC-11, FR-164-AC-12, NFR-035-AC-1, NFR-035-AC-2, NFR-035-AC-3, NFR-035-AC-4, NFR-035-AC-5, and every Behavior statement of FR-162, FR-163, FR-164 and NFR-035.
Context only: FR-136, FR-147, spec-artifacts-process/FR-014, spec-artifacts-process/FR-016.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | Since closed wont-fix anomalies are now listed, AC-5's unqualified 'closed' anomaly is ambiguous: with a wont-fix fixture the group must list both, contradicting 'the open one only'. Say 'closed as fixed' (or any resolution other than wont-fix). | spec/functional/FR-162-anomaly-report.md:107 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: The command SHALL list every anomaly whose working-tree `status` is `closed` and whose `resolution` is `wont-fix`. |
| FND-002 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: Under `--fail-on unjustified-impacting`, the command SHALL fail for each anomaly whose `status` is not `closed`, or is `closed` with `resolution` `wont-fix`, whose `safety_impact` is `impacting` and w |
| FND-003 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: / FR-163-AC-4 / A `closed` `wont-fix` anomaly (a fixture the module ships under FR-016) whose `closed_by` and commit author differ from the rationale `author` yields an `author` equal to the typed val |
| FND-004 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: / FR-163-AC-1 / An open anomaly with a two-row `## Release Rationale` yields `releaseRationale` with both entries in document order, `author` and `rationale` byte-equal to the record's extracted value |
| FND-005 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: title: "quoin anomaly report: the anomalies a release still carries, grouped by affected hazard and requirement" |
| FND-006 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: An anomaly has a release rationale when its `releaseRationale` holds at least one entry whose `author` and `rationale` each satisfy the value rule of `agent-ix/spec-artifacts-process` FR-016: non-empt |
