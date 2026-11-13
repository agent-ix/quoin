---
id: SR-20261113083117
title: "Integrity review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #857 (git diff origin/main...plat-1528-spec-quoin at 56f23309): spec/functional/FR-162, FR-163, FR-164, spec/non-functional/NFR-035 and both index edits"
---

# Integrity review of the anomaly report (FR-162, FR-163, FR-164, NFR-035)

## Summary

Ticket: PLAT-1528. Method: spec-review/spec-integrity-analysis, scoped to `git diff origin/main...HEAD` of agent-ix/quoin#857 at 56f23309.
Read against quoin FR-136 and FR-147 as merged, spec-artifacts-process FR-014 and FR-015 on main, and FR-016 from
the companion PR agent-ix/spec-artifacts-process#147.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | rationaleChange is derived 'for each row' but no requirement says it is a row key: FR-162's Row list and FR-162-AC-3's key list name releaseRationale and fixingChange and omit rationaleChange, and no AC checks where it appears in the JSON or Markdown. The dossier export reads the JSON, so its location must be fixed. Add rationaleChange to FR-162's Row list and AC-3 (or say it is nested in releaseRationale). | spec/functional/FR-163-anomaly-report-release-rationale.md:43 |
| FND-002 | medium | fixingChange (and FR-163's rationaleChange) has four states, present, absent, absent with reason pending, and not applicable, but FR-162 fixes no JSON or Markdown encoding for them. The 'not stated'/null rule applies only to values 'absent in the record', and neither value is a record key (FR-136, FR-015). Two implementers would encode pending differently, and the dossier export depends on it. State the encoding once (for example null, or a `state` of absent, pending or not-applicable) and add an AC. | spec/functional/FR-162-anomaly-report.md:58 |
| FND-003 | low | signatureCode (G/U/E/N...) is computed by git against the local keyring and allowed-signers config, so the same working tree, history and options give different bytes on two machines (U on one, E on another). Name the verification environment as an input to the determinism rule, or scope the claim to one machine. | spec/functional/FR-162-anomaly-report.md:84 |
| FND-004 | low | 'The last default-branch commit that holds the section' is not the tip when the section was removed on the default branch: a working tree that re-adds the old text then matches that old commit and reports a committed rationaleChange for an uncommitted section. Compare against the section at the default-branch tip (absent there means pending). | spec/functional/FR-163-anomaly-report-release-rationale.md:45 |

## Verdict

Changes requested on output shape: the derived git values have no fixed row location or absent-state encoding.

Checked and clean: no id collision (FR-162..164 and NFR-035 are on no other origin branch; #855 holds FR-160/161 and NFR-034, #852 FR-145..148/NFR-031, #856 no spec ids); both index entries match titles; relationship edges and Dependencies sections agree; `quire validate --scope .` on the four files exits 0 (only an unrelated spec-artifacts-app unused-import warning); the FR-136 refs divergence (report groups a dangling target, refs omits it) is stated and tested by FR-162-AC-8.

Examined (role examined): FR-162-AC-1, FR-162-AC-2, FR-162-AC-3, FR-162-AC-4, FR-162-AC-5, FR-162-AC-6, FR-162-AC-7, FR-162-AC-8, FR-162-AC-9, FR-162-AC-10, FR-162-AC-11, FR-162-AC-12, FR-162-AC-13, FR-163-AC-1, FR-163-AC-2, FR-163-AC-3, FR-163-AC-4, FR-163-AC-5, FR-163-AC-6, FR-163-AC-7, FR-163-AC-8, FR-163-AC-9, FR-164-AC-1, FR-164-AC-2, FR-164-AC-3, FR-164-AC-4, FR-164-AC-5, FR-164-AC-6, FR-164-AC-7, FR-164-AC-8, FR-164-AC-9, FR-164-AC-10, FR-164-AC-11, FR-164-AC-12, NFR-035-AC-1, NFR-035-AC-2, NFR-035-AC-3, NFR-035-AC-4, NFR-035-AC-5, and every Behavior statement of FR-162, FR-163, FR-164 and NFR-035.
Context only: FR-136, FR-147, spec-artifacts-process/FR-014, spec-artifacts-process/FR-016.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new pending rule fires when 'the section is absent at that tip', which also holds for an anomaly with no section anywhere; the next rule makes that same anomaly `absent`. The commonest case (no rationale in tip or working tree) gets two states. Restrict pending to a working tree that has the section (for example: 'If the working tree has the section and its text differs from the tip, or the tip lacks it'). | spec/functional/FR-163-anomaly-report-release-rationale.md:45-46 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: In the JSON, every row key above, `fixingChange` and `rationaleChange` included, SHALL be a key of the element of `anomalies`; none is nested in another key. |
| FND-002 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: In the JSON, each SHALL be an object with the key `state`, one of `present`, `absent`, `pending` or `not-applicable`, and the keys `commit`, `author`, `authoredAt`, `signature` and `signatureCode`. |
| FND-003 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: The command SHALL render the same bytes for the same working tree, history, options and git signature-verification configuration. `signature` and `signatureCode` are what git reports against the local |
| FND-004 | fixed | Fixed in 27093372 / 1fa38d53 on origin/plat-1528-spec-quoin; after-text: If the working-tree section text differs from the section text at the tip of the default branch, or the section is absent at that tip, then Quoin SHALL report `rationaleChange` as `pending`. |
