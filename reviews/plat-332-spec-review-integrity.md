---
id: SR-16322
title: "Integrity review of PLAT-332 trace-mark spec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md; spec/functional/FR-120-trace-marks-in-test-result-output.md; spec/functional/FR-121-emitter-mechanism-constraints.md; spec/non-functional/NFR-028-inert-and-low-cost-emission.md; spec/non-functional/NFR-029-self-contained-result-evidence.md; spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md; spec/usecase/US-023-record-a-run-that-names-its-own-requirements.md; spec/adrs/index.md; spec/*/index.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-332. PR agent-ix/quoin#773 (spec-only, head f1662cc3), scoped to `git diff origin/main...HEAD`.
The integrity lens examined ADR-001, StR-009, US-023, FR-120, FR-121, NFR-028 and NFR-029, reading FR-033 and FR-032
for context. It records 4 findings (4 high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-001 forbids deriving ids after the run but leaves FR-033-AC-15 (post-run nextest annotator) normative and unamended | spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md:39-40,78-81 |
| FND-002 | high | FR-120's normalized JSON shape has no path field, but NFR-029 requires a file path in normalized JSON results | spec/functional/FR-120-trace-marks-in-test-result-output.md:42 |
| FND-003 | high | FR-120-AC-4 needs the set of symbols in other files, which the adapter may not read, and no other component is named | spec/functional/FR-120-trace-marks-in-test-result-output.md:65-67,73-75,84 |
| FND-004 | high | FR-121-AC-5 picks cargo-nextest, whose JUnit names/classnames/file cannot meet FR-120's symbol rules without a post-run rewrite | spec/functional/FR-121-emitter-mechanism-constraints.md:40 |

## Detail

- **FND-001** (ADR-001): ADR-001 decides that nothing downstream infers trace ids, and line 78 calls the FR-033-AC-15 quoin-suite-trace annotator a matcher in its sense. But FR-033 and AC-15 (TC-2010) stay normative, unamended, and present the annotator as the design, while the ADR says 'Supersedes nothing' and defers the matter to PLAT-333. FR-120 says it 'extends' FR-033 without amending it. The spec set now holds two contradictory requirements at once. The ADR's main argument (the adapter contract forbids the matcher's input) also does not reach the annotator: it is a separate tool that runs before the adapter (quoin-suite-trace annotate.rs TraceIndex::build joins the engine relation on (binary id, module path::symbol)). Fix: the ADR supersedes FR-033-AC-15, and FR-033 marks AC-15 and its section superseded or interim, with the end condition stated.

- **FND-002** (FR-120): NFR-029 applies to 'JUnit and normalized JSON result files' and requires each tagged test's symbol, file path, outcome and trace ids in the artifact. FR-120's normalized JSON row fixes the shape as {"entries":[{"symbol","outcome","traceIds"}]} with no path, and RunEntry has no path field either. A normalized-JSON emitter cannot satisfy both. Fix: add a path field to the normalized entry shape (and to FR-120-AC-3's scope).

- **FND-003** (FR-120-AC-4): To report a case unjoinable because its qualified name matches symbols in more than one file, something must know the static symbol set (quire coverage). The adapter cannot: FR-120-CON-2 and FR-033-CON-1 limit it to the result text. No other component is assigned, and FR-120's Rationale says reading `file` is 'the only quoin-side change'. That is false if AC-4 stands. 'Binds to none of them' also conflicts with the emit model, where the mark's ids bind the obligation whatever the symbol join says. Fix: name the component that owns the symbol join (matrix.build / auditor), state whether ids from an unjoinable entry still bind, and correct the Rationale.

- **FND-004** (FR-121-AC-5): FR-120 requires the emitter to write name = classname = the extractor's in-file qualified_name, plus file = repo path. nextest owns its JUnit writer: classname is the binary id and name is the crate-root module path (quoin-suite-trace keys on exactly (binary id, module path::symbol)), and it writes no file attribute. So a unit test in src/symbols/rust.rs reports as symbols::rust::tests::t, while the extractor records tests::t. Making nextest output meet FR-120 needs a post-run rewrite that maps names to files, which is the derivation ADR-001 rejects and close to what FR-121-AC-6 forbids. Fix: state which part of the Rust path writes name/classname/file (a custom harness, or a nextest feature confirmed to exist), or amend FR-120 for Rust.

## Verdict

Not mergeable as written: high findings open.
Clean units (examined, no finding from this lens): every other unit listed in the reviewer comment scope.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-120 gives matrix.build (FR-115) a path-aware symbol join and an ambiguous-join report that FR-115 neither defines nor is amended to include | spec/functional/FR-120-trace-marks-in-test-result-output.md:80-90,107 |
| FND-006 | low | The interim annotator's removal condition is stated four different ways | spec/functional/FR-033-evidence-format-adapters.md:83-85,114 |

- **FND-005** (FR-120-AC-4): The fix makes `matrix.build` (FR-115) join run entries to declared symbols by path and qualified_name and report ambiguous joins. FR-115 is unchanged. It describes matrix.build as 'a join, not a new check' over coverage_matrix, the FR-030 binding graph and the FR-032 AuditReport, per criterion. It has no entry-to-symbol join by path and no ambiguity report. Today the run-entry to bound-symbol match lives in the FR-030/FR-032 binding evaluation. So the owner named in FR-120 has no requirement for the behaviour, and AC-4 tests another FR's component from inside FR-120. Fix: amend FR-115 (or FR-032, wherever the entry-symbol match actually lives) with the path-aware join and the ambiguity outcome, and move or cross-reference AC-4 there.

- **FND-006** (FR-033-AC-15): The FR-033 section says 'until the Rust emitter of PLAT-333 lands'. FR-033-AC-15 says 'when the FR-120 emitters land'. ADR-100 Status says 'until the emitters of PLAT-333 and its siblings land'. ADR-100 Consequences says 'once the emitters of FR-120 land'. The annotator only serves Rust/nextest, so the condition should be the Rust emitter (PLAT-333), stated once and cited.

## Dispositions

Round 1 (fix commit 90cc763e).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
