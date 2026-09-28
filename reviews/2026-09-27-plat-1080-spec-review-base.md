---
id: SR-164
title: "Base spec review of FR-115 evidence-backed test matrix (quoin#646)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@b2b8df657fda0fd86caf75aff45436174223fdac; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-021-launch-ix-flow-runs.md, spec/matrix.md, spec/log.md, spec/functional/index.md (read for contract: rust/crates/quoin-auditor/src/audit/{mod,ladder}.rs, rust/crates/quoin-finding-types/src/{finding,report}.rs, rust/crates/quoin-evidence/src/{record.rs,types/binding.rs}, rust/crates/quoin-core/tests/tc_447_assurance_boundary.rs, agent-ix/quire-rs@af5ec21 spec/functional/FR-050 AC-47..51)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

# Base spec review: FR-115 (PR quoin#646)

## Summary

Ticket: PLAT-1080 (epic PLAT-1076). Base checklist over the markdown-only diff:
the new FR-115 (11 ACs, 3 CONs, CR-001), the FR-021 CR note, and the matrix,
log and index rows. IDs are well formed and unique. FR-115 validates clean
under `quire validate`. The `matrix.md` failures quire reports are all
pre-existing and none is on the new row. The status mapping is total, because
AC-4 ends in "else", but two load-bearing claims do not hold against the
auditor FR-115 builds on. First, `bound` is defined as "a passing run backs
this criterion", yet the auditor never looks at `fail`/`error` outcomes.
Second, `evidence_detail` names fields the supplied inputs do not carry, for
cases that have no backing `Finding` at all. The CLI's assembly of the auditor
inputs is also left open, which leaves "bound relative to HEAD" undecided.

## Findings

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-001 | high | `bound` promises "a passing run backs this criterion", but `AuditReport.healthy` does not guarantee that. The auditor's ladder checks only `Outcome::Skip` (vacuity) and never `Fail`/`Error`. A binding minted by an earlier passing run survives. If the latest run of that suite, at HEAD, reports the bound symbol as `fail`, the obligation completes the ladder and lands in `healthy`, so FR-115 renders `bound` for a red build. CON-2 forbids `matrix.build` from adding the check itself. The spec must either name the FR-032 failed-run gap as a blocking dependency, as CR-001 does for quire, or stop claiming "passing". | spec/functional/FR-115-evidence-backed-test-matrix.md:56, spec/functional/FR-115-evidence-backed-test-matrix.md:29, spec/functional/FR-115-evidence-backed-test-matrix.md:78, rust/crates/quoin-auditor/src/audit/ladder.rs:394-405, spec/functional/FR-032-evidence-auditor.md:28 | wrong-requirement |
| FND-002 | high | The `evidence_detail` schema is not total and cannot be sourced. It is "present for `undischarged`" and names "the backing `Finding`'s `kind`, `suite`, `commit`, and `summary` verbatim". `Finding` has no `suite` or `commit` field. `Binding.commit` is the commit of the FIRST discharging run, not the run that made the evidence stale, and the inputs carry no runs. An `undischarged` row from an `unevaluated` entry, or from absence from every collection, has no `Finding` to name. When an obligation has two findings (`unknown-method` + `undischarged`, `stale-evidence` + `vacuous-evidence`) or several bindings, nothing selects which one to use. No AC exercises `evidence_detail`. | spec/functional/FR-115-evidence-backed-test-matrix.md:133-136, rust/crates/quoin-finding-types/src/finding.rs:39-110, rust/crates/quoin-evidence/src/types/binding.rs:76-93 | wrong-requirement |
| FND-003 | medium | "Bound relative to HEAD" is delegated to "whichever `head_commit` the caller passed into the auditor". However, the Behavior bullet for `quoin matrix` only says "running the auditor". It does not require passing `head_commit` = the checked-out HEAD, or the mock-inspection/catalog/independence inputs `quoin evidence audit` uses. When `head_commit` is absent, the auditor skips `behind_head` entirely (`ladder.rs` filters on `Some(head)`), so months-old runs read `bound`. No AC pins these inputs. | spec/functional/FR-115-evidence-backed-test-matrix.md:112-117, spec/functional/FR-115-evidence-backed-test-matrix.md:155-160, rust/crates/quoin-auditor/src/audit/ladder.rs:148-153 | missing-requirement |
| FND-004 | medium | AC-8 is not falsifiable as written. It requires byte-identical output across two calls, "covered by a ... protocol test ... asserting exit class and diagnostic code only". A test that asserts only the exit class and diagnostic code cannot detect non-deterministic success output. In `tc_447_assurance_boundary.rs` that rule applies to refusals, not to verdict bytes. | spec/functional/FR-115-evidence-backed-test-matrix.md:181, rust/crates/quoin-core/tests/tc_447_assurance_boundary.rs:22-24 | wrong-requirement |
| FND-005 | low | The new matrix row uses `⚠️ Spec-ahead-of-code`. spec-artifacts-process v0.26.0 (CR-031) retired `⚠️` as a status marker (`StatusMarker.json`, `TestMatrix` skeleton: "`⚠️` IS NOT VALID"). It validates here only because this table's `Coverage` column is not pattern-asserted. Adding a 41st retired marker widens the gap. Use `🚧`, as FR-112 does with `🚧 Pending`. | spec/matrix.md:166 | wrong-requirement |
| FND-006 | low | CR-001 links PLAT-1077 as `https://github.com/agent-ix/quoin/issues/1077`, which does not exist. PLAT-1077 is the quire-rs `CoverageMatrix` implementation (quire-rs#494), not "the repin". The quoin-side repin has no named owner. "Not yet vendored" misdescribes a git-rev pin, which is not a vendored copy. The log entry says "Closes ... PLAT-1080", but this spec-only PR implements nothing. | spec/functional/FR-115-evidence-backed-test-matrix.md:196-203, spec/log.md:32 | wrong-requirement |
| FND-007 | low | The FR-021 CR note says its "test coverage remain[s] exactly as stated", but the FR-021 matrix row cites `"matrix runs the flow and propagates a non-zero exit code"` as FR-021 evidence. That row goes stale when AC-11 lands. `help.rs` still advertises `matrix` as "Build or update a requirements test matrix", which contradicts CON-3. No AC covers the help text. | spec/functional/FR-021-launch-ix-flow-runs.md:62-71, spec/matrix.md:62, rust/crates/quoin-cli/src/help.rs:86 | wrong-requirement |

## Coverage and validation

- `quire validate --scope . <5 touched files>` (quire 0.33.0, engine 0.47.1)
  exits 1. Every error is in `spec/matrix.md` at lines 200, 692, 981-994,
  1431 and 1457. All are pre-existing: header mismatches, retired `⚠️` in TC
  Status cells, and a malformed TC-1926 row. None is on line 166, the only
  line this PR adds to that file. FR-115 reports no diagnostics. The two EARS
  warnings on FR-021 (lines 18 and 22) are on untouched text.
- The matrix row for FR-115 states that nothing is implemented yet. By
  convention no TC rows exist, which is acceptable for spec-ahead-of-code.
- No test embeds a touched file. The only generic `spec/functional` reader
  (`quoin-jev` eval_v2 `load_docs`) walks the directory, and adding a document
  does not change any fixed assertion.

## Verdict

Not mergeable as is. FND-001 and FND-002 are HIGH. FND-003 and FND-004 are
MEDIUM and are fixable in the same round. The leader-accepted design (pure
mapping, the `suspect` fold, precedence) is sound. These findings are about
what the spec promises and the output schema, not the design.

## New findings (disposition pass 1)

Reviewed at agent-ix/quoin@e609508fcf96f3b97d5ec817c5ff051e56653c34.

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-008 | low | The fix removed the dead PLAT-1077 link but added four more of the same kind. PLAT-1086 is linked as `https://github.com/agent-ix/quoin/issues/1086`, which does not exist: `gh issue view 1086 -R agent-ix/quoin` cannot resolve it, and the Linear ticket has no GitHub attachment. PLAT-1086 exists only in Linear, where it correctly blocks PLAT-1080. Use the bare id or the Linear URL. | spec/functional/FR-115-evidence-backed-test-matrix.md:97, spec/functional/FR-115-evidence-backed-test-matrix.md:258, spec/functional/FR-115-evidence-backed-test-matrix.md:286, spec/functional/FR-115-evidence-backed-test-matrix.md:318 | wrong-requirement |

## Dispositions

Round 1, reviewed at agent-ix/quoin@e609508fcf96f3b97d5ec817c5ff051e56653c34.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e609508 |
| FND-002 | fixed | e609508 |
| FND-003 | fixed | e609508 |
| FND-004 | fixed | e609508 |
| FND-005 | fixed | e609508 |
| FND-006 | fixed | e609508 |
| FND-007 | fixed | e609508 |
