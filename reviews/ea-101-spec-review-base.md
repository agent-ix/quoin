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
