---
id: SR-20261113055214
title: "Base spec review of analysis-result counting through retained evidence (FR-152, FR-153, NFR-033, TC-002)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #849 (git diff origin/main...origin/ver-81-spec-retained-evidence-analysis at 480ba1c2): spec/functional/FR-152, spec/functional/FR-153, spec/non-functional/NFR-033, spec/test/TC-002, spec/functional/FR-151 (Downstream line), spec/spec.md, spec/functional/index.md, spec/non-functional/index.md, spec/test/index.md"
---

# Base spec review of analysis-result counting through retained evidence (FR-152, FR-153, NFR-033, TC-002)

## Summary

Ticket: VER-81. I checked the spec against the code, not against its prose. On the quoin side I read
`quoin-evidence/src/record.rs`, `store/graph.rs`, `store/records.rs`, `types/run.rs` and
`adapters/differential_report.rs`. I also read `quoin-core/src/ops/evidence/records.rs`,
`quoin-cli/src/evidence.rs` and `quoin-auditor/src/audit/{mod,ladder,stale,method}.rs`. On the
quire-rs side I read `origin/main` `src/evidence.rs`, `src/coverage.rs`, `src/coverage/reconcile.rs`
and FR-078. From quire-analyze I read the `origin/main` v2 schema. No assurance profile applies:
AP-201 and AP-202 are scoped to finding scoring and Jev lenses.

These claims about existing behaviour are confirmed:

- `record_run` binds only a `pass` entry, and only when its trace id equals an obligation id or a
  declared target id. A target that is also an obligation id stays an obligation id. An unknown id is
  reported as `unmatched`.
- `bind` replaces symbols and lineage per `(obligation, suite)`.
- The auditor reads the newest run of each suite by timestamp. It flags a bound symbol that is
  `fail` or `error` (`failed_run`). It flags a binding whose symbols are all skipped or absent
  (`vacuous_run`).
- quire-rs FR-078 is built: `BackingInput`, `RecordClaim`, `ActResult`, `CauseCode`,
  `strict_decision`, `trace` over Target/Record/Run, `record_binders`, `evidence_results` and
  `CoverageMatrixStatus::QualifiedRecord`.
- There is no `qualify` in quire-rs or quoin, and no qualifier or bridge code. The disposition
  table matches FR-151 as merged and the FR-078 categories.
- There are no ID collisions. No branch among the 300 origin refs holds FR-152, FR-153, NFR-033 or
  TC-002. PR #849 is the only open PR.

The central defect is FND-001. The binding source FR-152 introduces already exists as
`--discharges`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-152 adds `--link`, but `quoin evidence record` already has `--discharges` (FR-041). It does exactly this: stated obligation ids go onto every entry as trace ids, only a passing entry binds through the same `record_run` join, and an unknown id is reported `unmatched`. It is gated to the sbom adapter by one check (records.rs:217-227). The claim that "nothing in the existing chain supplies a trace id" is false, and the alternatives list omits it. The minimal change is to admit `differential-report` to `--discharges` and cite FR-041 as precedent. The differences `--link` invents are unexplained: supplied order instead of sorted and deduplicated, declared target ids in addition to obligation ids, and unknown ids stored as trace ids. | spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:57-66, :83-104; rust/crates/quoin-core/src/ops/evidence/records.rs:210-241; spec/functional/FR-041-sbom-inventory-evidence.md:73-85 |
| FND-002 | high | FR-152-AC-3 and TC-002 step 2 need an accepted report whose `analysisKind` or `requestDigest` equals an obligation id. `analysisKind` is the closed enum consistency, contradiction, implication, redundancy and dead-antecedent. `requestDigest` is a validated digest, and FR-151 adopts `validate_report_document`, which refuses an edited copy. The test therefore passes through refusal and never exercises the no-inference rule. Instead, choose the fixture's obligation or declared-target ids to equal the report's values. | FR-152-AC-3; spec/test/TC-002-analysis-result-counting.md:26-28; quire-analyze schemas/differential-report-v2.schema.json analysisKind |
| FND-003 | medium | FR-152-AC-2 requires the binding graph to be "byte-identical to before the run". `record_run` always calls `write_bindings` (record.rs:172), which writes `bindings.json` even when nothing bound (graph.rs:48-65). From the empty store in TC-002 step 1, the file goes from absent to present. Specify that no binding is added, or start the test from an existing graph. | FR-152-AC-2; rust/crates/quoin-evidence/src/record.rs:172 |
| FND-004 | medium | Nothing stops a link from naming an obligation whose declared method is not Analysis, and `record_run` binds it. The legacy audit's `method_conformance` is silent when `evidenceKind` is unset (method.rs), so `quoin evidence audit` reports a Test obligation as healthy on an SMT result. FR-152 must state whether such a link is refused or reported. | spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:83-104; rust/crates/quoin-auditor/src/audit/method.rs |
| FND-005 | medium | "Later run" is undefined, yet NFR-033 requires the result to be independent of order. The store orders runs by the caller-supplied `--timestamp` string, with ties broken by file name (records.rs:161). The auditor's `runs_by_suite` keeps whichever run comes last in its input (audit/mod.rs:72-75). FR-153-AC-5 also names no expected cause. FR-078's table and FR-116 make a superseded pass `stale`, with the superseding `failed`/`errored` still visible. State "later = newer run timestamp (FR-030)" and the expected categories. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:229-232; FR-153-AC-5; NFR-033 |
| FND-006 | medium | FR-153-AC-6 says "the passing one stays visible", but FR-078 has no channel for that. Every `ActResult` cause is non-qualified. If the bridge emits the passing `RecordClaim`, reconcile marks the row `qualified-record` (reconcile.rs:880-884), which contradicts "uncounted". Under FR-153-CON-1 (no new status) the clause cannot be implemented as written. Name where it is visible (the quoin audit output, for example) or drop it. | FR-153-AC-6; quire-rs src/coverage/reconcile.rs:880-884 |
| FND-007 | medium | FR-152-AC-9 and FR-153-AC-11 assert properties of "the change that implements this requirement" (a `git diff`, "the only analysis-specific code"). That is a process check on a diff, not product behaviour, and no artifact persists to verify it later. FR-152-CON-1 and FR-153-CON-1 already state the product constraint. Delete both ACs. | FR-152-AC-9; FR-153-AC-11 |
| FND-008 | low | FR-153 says "the entry's `evidenceKind` is unset (FR-151) and no run field establishes the act". FR-152 says the entry has "no `evidenceKind`". `evidenceKind` is a `RunRecord` field (types/run.rs:187), not a `RunEntry` field, and `quoin evidence record --kind` sets it for any adapter (records.rs:249). The conclusion holds because FR-116 says declaring `evidenceKind` establishes nothing, so cite FR-116 instead. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:198-202; spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:42-44 |
| FND-009 | low | For a `disagreement`, the link is never stored: the run has zero entries and `unrepresented` is not stored. FR-153-AC-4's "absent for the linked obligation" therefore gives the same result as an obligation never recorded. The AC cannot detect whether the link was handled. | FR-153-AC-4 |
| FND-010 | low | FR-153-AC-5 and TC-002 step 5 need a satisfied report and a refuted report with the same `requestDigest`. A deterministic capture cannot produce both, so FR-151-AC-14's capture rule does not supply them. State that the pair is hand-constructed and accepted by the validator, as FR-151-AC-5 does for `unsupported-platform`. | FR-153-AC-5; spec/test/TC-002-analysis-result-counting.md:34-35 |

## Verdict

Not mergeable as written. FND-001 replaces the spec's central addition with an existing option.
FND-002 is an AC that no valid test can build.

What is sound: the recorder-stated binding fits FR-033 and ADR-100's no-inference rule, because it
is FR-041's pattern. Counting only through FR-078 `qualified-record` is right. The disposition table
is right. Keeping the adapter free of `traceIds` and leaving quire-rs unchanged is right.

Examined (role examined): FR-152 statements, CON-1, CON-2 and AC-1 to AC-9; FR-153 statements,
CON-1 to CON-3, the disposition table and AC-1 to AC-11; NFR-033; TC-002; the FR-151 Downstream edit
and the index edits.

Context only: FR-030, FR-032, FR-033, FR-041, FR-116, ADR-100 and quire-rs FR-078.
