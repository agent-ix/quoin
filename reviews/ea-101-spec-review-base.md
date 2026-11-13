---
id: SR-20261113092139
title: "Base spec review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #861 (git diff origin/main...ea-101-snapshot-spec at 4325f1fb): spec/functional/FR-166, spec/functional/FR-167, spec/functional/FR-168, spec/non-functional/NFR-036, spec/functional/index.md, spec/non-functional/index.md"
---

# Base spec review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)

## Summary

Ticket: EA-101. This review is scoped to the PR diff. I read the new requirements against the current text of FR-115,
FR-032, FR-039, FR-144, FR-143, FR-162, FR-163 and FR-063 on quoin main, and against engineering-assurance FR-051 and
FR-052 on the paired branch. The cross-reference check covers names only.

These parts are sound. There is no digest, hash, manifest, pin or file listing anywhere. The snapshot is bound
only by git's commit hash, and FR-167 forbids the other forms. `--baseline-commit` is checked by shape alone
(40 or 64 lowercase hex characters) and is never resolved. Refusing an existing `--out` fits engineering-assurance
FR-051, which requires the release commit to add `snapshot_path`. Because the output is deterministic, the refusal
costs only a manual delete before a rerun. The history file is read for its named members only, so no copy of its
schema is made. The member names and ordering match FR-052's output. `evidence_freshness` counts criteria over
FR-115's closed, total five-status vocabulary, and `open_findings` is FR-115's `evidence_detail.findings`, so
neither is invented. Neither member contains paywalled text or roadmap content. AssuranceProfiles AP-201 and AP-202
are scoped to finding scoring and Jev lenses, so neither applies to this spec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-166 builds carried anomalies through the `quoin anomaly report` code path. That path reads git history: FR-162 says it "SHALL read ... the git history of those files" to get FR-136 `fixingChange` and FR-163 `rationaleChange`. That conflicts with FR-166's own "SHALL NOT run git" and with AC-2. Fix: name only the selection and record-reading part, and state that the snapshot never derives `fixingChange` or `rationaleChange`. FR-167 explains only why `rationaleChange` is left out. | spec/functional/FR-166-release-snapshot-command.md:61; spec/functional/FR-162-anomaly-report.md (Output, Derived git values); FR-163 Attribution by git |
| FND-002 | high | FR-167-AC-6 expects `open_findings` `[]` for an empty evidence store. FR-032-AC-5 reports every unbound obligation as `undischarged`, and FR-115-AC-3 keeps `evidence_detail` computed under `no run evidence`. FR-167 also defines `open_findings` as every auditor finding. A correct implementation therefore lists one `undischarged` finding per criterion and fails AC-6. Either the AC or the definition must change. | FR-167-AC-6; FR-032-AC-5; FR-115-AC-3 |
| FND-003 | medium | The history file carries its own `baseline_commit` (EA FR-052 Outputs). FR-168 does not read it, and FR-166 takes `--baseline-commit` separately without comparing the two. A history computed for another range silently supplies the wrong `change_records`. Fix: refuse when the two strings differ (a plain comparison, with no git), or take the baseline from the history file. | spec/functional/FR-168-release-snapshot-change-records.md:33-35; EA FR-052 Outputs |
| FND-004 | medium | FR-166-AC-6 requires snapshot rows to "equal" what the three reports render. The snapshot renames keys (`safetyImpact` becomes `safety_impact`, `releaseRationale` becomes `release_rationale`) and keeps only some columns: no anomaly `path`, `assessment`, `affected` or derived git values, and no SOUP advisory or policy status. It also cannot pass `--records` or `--support`, so "for the same inputs" cannot be set up. State the projection the test compares. | FR-166-AC-6; FR-167 member table; FR-162 Row |
| FND-005 | medium | FR-166 Inputs leave out FR-144's `--records <dir>` and FR-143's `--support <file>`. Without `--support`, supplied support data never reaches the snapshot. A record whose `end_of_support` is `unknown` then loses its past-end-of-support group, and the SOUP rows differ from the report the release relied on. | spec/functional/FR-166-release-snapshot-command.md:43-50; FR-144 Behavior; FR-143 |
| FND-006 | medium | `soup.not_assessed` keeps only scans reported not run, refused or vacuous. FR-144 also appends after its rows the unmatched advisories, unmatched policy findings, and `licence-policy`/`cargo-deny` scans reported unordered. An unresolved advisory that matches no record would vanish from the snapshot. | spec/functional/FR-167-release-snapshot-format.md (soup row); FR-144 Behavior |
| FND-007 | medium | `open_findings` is ordered "by `obligation` and then by `kind`", which is not a total order. FR-039-AC-14 and AC-16 allow up to 25 `surviving-mutant` findings per obligation. Ties then fall to whatever order the implementation picks, which goes against NFR-036's byte identity. Add tiebreakers on `path`, `line`, `symbol` and `summary`. | spec/functional/FR-167-release-snapshot-format.md (Behavior); FR-039-AC-14, AC-16; NFR-036 |
| FND-008 | medium | NFR-036 sets "path values in the output" to 0. FR-167 requires `path` in `open_findings` and in `change_records`. Taken literally, the metric fails on every real snapshot, so a test written to it fails or is weakened. Restate it as no absolute, cwd-dependent or host-dependent path. | spec/non-functional/NFR-036-release-snapshot-is-deterministic-and-never-decides.md (Measurement row 2); FR-167 |
| FND-009 | low | The snapshot states "the evidence state of the baseline" (EA FR-051), but every source is read from the `--repo` working tree. FR-162, for example, selects on working-tree `status`. FR-166 never states the caller's precondition that the tree is a clean checkout of `--baseline-commit`. Quoin cannot check this, so it should be stated. | spec/functional/FR-166-release-snapshot-command.md:25-35; EA FR-051 |

## Verdict

Changes required. FND-001 and FND-002 are contradictions with existing requirements, and as written either
blocks a correct implementation. The other findings are real gaps in edge cases or testability. On the quoin side,
the ticket's acceptance criteria for contents, determinism, ownership split and version history are otherwise met.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | The reworded FR-166-AC-6 requires that "the snapshot has no member those reports do not define". FR-167 puts `group` in each SOUP row, but `group` is FR-144's ordering group, not a member of its JSON. FR-167 also puts `unassessed` in each anomaly row, but FR-162 calls it a row flag and FR-162-AC-3's row-key list has no `unassessed` key. A correct snapshot therefore fails AC-6. Exempt these two members by name and state where each value comes from. | FR-166-AC-6; spec/functional/FR-167-release-snapshot-format.md (soup, anomalies rows); FR-144; FR-162-AC-3 |
| FND-011 | medium | FR-166 now limits its use of FR-162 to Selection and Row. That leaves out FR-162's "Unreadable documents" rule, under which a document with a mapping error is listed in `unreadable` and the command exits non-zero. The snapshot has no `unreadable` member and FR-166 has no refusal for this case, so a carried anomaly whose document fails mapping disappears from the snapshot without notice. Fix: refuse with a named code, or carry `unreadable`. | spec/functional/FR-166-release-snapshot-command.md:72; FR-162 Unreadable documents |
| FND-012 | low | `detail`, the identifying text FR-144 states for a `not_assessed` item, is a member of the closed schema and the last sort key. FR-144 defines no such text field, so two implementers could produce different bytes. Define `detail` for each `kind`, for example the advisory id plus the purl, or the FR-142 reason word. | spec/functional/FR-167-release-snapshot-format.md:46-49 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-166 now limits anomaly use to FR-162 Selection/Row and FR-163 As authored, forbids deriving `fixingChange`/`rationaleChange` and calling the git-reading code, and adds AC-2 and FR-167-AC-9 for this (49a3bd0f). |
| FND-002 | fixed | FR-167-AC-6 now expects one `undischarged` finding per criterion, consistent with FR-032-AC-5 and FR-115-AC-3. |
| FND-003 | fixed | FR-168 reads `baseline_commit` and refuses `release_snapshot_history_baseline_mismatch` on a byte-for-byte string difference, resolving neither value (AC-6). EA FR-052 at PR #226 head 14a7af2 echoes "the request's value, byte for byte", so the two specs agree. |
| FND-004 | fixed | AC-6 now compares member by member, maps camelCase to snake_case, and names the shared inputs. Its new "no undefined member" clause raises FND-010. |
| FND-005 | fixed | `--records` and `--support` were added to Inputs, with AC-8 covering both. |
| FND-006 | fixed | `not_assessed` now carries every item FR-144 appends, as `{kind, tool, detail}` with six kinds, ordered, and covered by AC-4. FND-012 is raised on `detail`. |
| FND-007 | fixed | `open_findings` is now totally ordered by obligation, kind, path, line, symbol and summary, with an absent value sorting before a present one; AC-3 tests ties on line and on summary. |
| FND-008 | fixed | The NFR-036 metric now counts absolute-path and host-dependent values and exempts the repository-relative `path` members. |
| FND-009 | fixed | FR-166 Description now states the caller precondition: a clean checkout of the baseline commit. |
| FND-010 | fixed | FR-166-AC-6 now exempts `group` (FR-144's ordering group for the row) and `unassessed` (true exactly when `safety_impact` is `unassessed`, the FR-162 row flag), and compares each against its stated definition. FR-167 now states both sources (dfff14e0, a844d37c). |
| FND-011 | fixed | FR-166 now refuses with `release_snapshot_anomaly_unreadable`, naming the path and the mapping code, when a document in scope has FR-136 `anomaly.no-record`. FR-166-AC-9 covers this. |
| FND-012 | fixed | FR-167 now defines `detail` for each `kind`: the advisory id or rule id, then `\|`, then the purl (empty when none) for the unmatched kinds, and the FR-144/FR-142 state word for the four `scan_` kinds. |
