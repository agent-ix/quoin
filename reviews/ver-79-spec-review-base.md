---
id: SR-20261113055212
title: "Base spec review of the quoin differential report v2 adapter (FR-150, NFR-032, TC-001)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #848 (git diff origin/main...HEAD at dfd3b6b7): spec/functional/FR-150, spec/non-functional/NFR-032, spec/test/TC-001, spec/functional/FR-069, docs/campaign-native-result-inventory.md, spec/spec.md, spec/functional/index.md, spec/non-functional/index.md"
---

# Base spec review of the quoin differential report v2 adapter (FR-150, NFR-032, TC-001)

## Summary

Ticket: VER-79. Reviewed against code, not prose: quoin `rust/crates/quoin-evidence/src/adapters/`
(differential_report.rs, registry.rs, contract_conformance.rs, types/run.rs, record.rs),
`rust/crates/quoin-cli/src/evidence.rs`, `rust/Cargo.toml`; quire-analyze `origin/main` (c188eaf)
`schemas/differential-report-v2.schema.json`, `src/report.rs`, `src/package_runner.rs`,
`src/producer_evidence.rs`, `src/solver.rs`, `Cargo.toml`, FR-005, FR-009; tl-mltl and quire-rs
`origin/main`.

Replace, not beside: confirmed. No source in quoin, quire-analyze, quire-rs or tl-mltl writes
`differential-summary/v1`; the only occurrence is the committed
`corpus/r2u2-v4.2/differential-report.json` in tl-mltl. Removing the v1 reader is correct and is
not a compatibility decision.

The v2 disposition table matches the producer: `compare_solver_records` / `disposition_from_values`
derive `unavailable` from any engine outcome `missing-executable` or `unsupported-platform`,
`agreement` from equal conclusive statuses (with a verified explanation for `sat`), `disagreement`
from opposite conclusive statuses, and `inconclusive` otherwise; `agreedStatus` is non-null only for
`agreement`. run-package handles one consistency request only and exits 0/1/3, collapsing
disagreement, inconclusive and unavailable to exit 3.

The adoption of `validate_report_document` as the single definition of a well-formed report is the
right design, but three statements contradict what that function does (FND-001, FND-002, FND-003),
and the spec does not say how quoin reaches it (FND-004). quoin already takes agent-ix crates as
Cargo git dependencies pinned by `rev` (quire-rs, quire-canonical, engineering-assurance), and every
quoin crate is `publish = false`, so a git dependency on quire-analyze is allowed under the
no-vendoring rule; copying or re-embedding the schema is not.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The adapter accepts only what quire-analyze's validation accepts, but `validate_report_document` (report.rs:291-508) decodes `queryHex`, `stdoutHex`, `stderrHex` and `modelHex`, re-parses each engine's stdout, and derives `queryDigest` from `requestDigest` and the query bytes (FR-005-AC-6: "Standalone v2 validation derives queryDigest"). FR-150 lines 111 and 113 forbid exactly that, so no implementation can satisfy all three statements. Delete lines 111 and 113, or reword them to "the adapter SHALL perform no digest derivation or hex decoding beyond what `validate_report_document` performs". | spec/functional/FR-150-differential-report-v2-adapter.md:87, :111, :113 |
| FND-002 | high | AC-11 says reordering the members of the JSON object changes nothing, and line 93 says the result does not depend on member order. The owner refuses any document that is not canonical compact, key-sorted JSON (report.rs: "report JSON is not in canonical compact form"; FR-005 canonical JSON), so a reordered or pretty-printed report is refused. As written, AC-11 cannot pass. Restate it: a reordered or whitespace-changed copy is refused, with the same error on every run, and determinism is stated over identical bytes only. | spec/functional/FR-150-differential-report-v2-adapter.md:93, FR-150-AC-11 |
| FND-003 | medium | The 16 MiB cap is quoin's own number and is below the producer's bound. quire-analyze `MAX_REPORT_BYTES` is 64 MiB (report.rs:18), and the default solver limits allow 16 MiB of stdout per engine (solver.rs:37), which is 32 MiB as hex for each engine. So a report run-package validly publishes can be refused by quoin. No neighbouring quoin run adapter has a per-adapter cap: the CLI reads `--results` whole (evidence.rs:351). Use the owner's `quire_analyze::MAX_REPORT_BYTES` through the dependency, checked before parsing, and move the AC-10 and NFR-032 boundary to that constant. | spec/functional/FR-150-differential-report-v2-adapter.md:95, FR-150-AC-10, NFR-032 |
| FND-004 | medium | The Description says a schema change "reaches quoin by dependency" but never names the dependency. VER-79 leaves this decision open. Recommended wording: "quoin SHALL depend on `agent-ix/quire-analyze` as a Cargo git dependency pinned by `rev` in `rust/Cargo.toml`, as it depends on quire-rs and engineering-assurance, and the adapter SHALL call `quire_analyze::validate_report_document` and `quire_analyze::MAX_REPORT_BYTES`; quoin SHALL NOT copy, re-embed or re-implement the schema." Two blockers must also be named as an Upstream enablement dependency. First, the quire-analyze lib does not build for quoin's `x86_64-pc-windows-msvc` release target, because package_runner.rs imports `std::os::unix` unconditionally; the validator must be exposed from a crate that builds on every quoin target (for example a `crates/` member, as `method-contract` is). Second, that crate brings in `jsonschema =0.17.1`, alongside quoin's `=0.56.0`. | spec/functional/FR-150-differential-report-v2-adapter.md:27-32 |
| FND-005 | medium | `disagreement` and `inconclusive` both become an `error` entry with the same symbol and no other field, so the store cannot tell a solver disagreement, which signals a soundness defect, from a timeout. `Outcome::Error` is defined as "The producer could not run this symbol" (types/run.rs:85). That fits an unavailable engine, not two engines that ran and concluded opposite results. The precedent runs the other way: the retired v1 reader mapped tool-error to `error` and gave `unrepresented` only to the state no outcome means. Recommend: `disagreement` becomes `unrepresented` with state `disagreement`; `unavailable` (could not run) becomes `error` like `inconclusive`. If the PR keeps its mapping, it must state how a disagreement stays distinguishable in the store. | spec/functional/FR-150-differential-report-v2-adapter.md:60-74, FR-150-AC-3, FR-150-AC-5, FR-069-AC-3 |
| FND-006 | medium | For `unavailable`, "no entry" leaves open whether `quoin evidence record` writes a RunRecord with zero entries: `record_run` writes the run unconditionally (record.rs:68-82). FR-069 also says an empty result set SHALL be refused, because a differential that compared nothing did not agree. The `unrepresented` result's `symbol` is not stated either. State whether a run is written for an unavailable report, and that the symbol is `<analysisKind>::<requestDigest>`. | FR-150-AC-5 |
| FND-007 | medium | AC-5 and TC-001 step 1 need an `unsupported-platform` report captured from `quire-analyze run-package`. On Linux the runner cannot produce one: `state_outcome` maps a containment failure to `cleanup-failed`, which is inconclusive, when the host is Linux (producer_evidence.rs:535-536). AC-14 already limits captures to the dispositions "the runner can produce". State where the unsupported-platform fixture comes from (a capture made on a non-Linux host), or drop that half of AC-5. | FR-150-AC-5, FR-150-AC-14, TC-001 |
| FND-008 | low | AC-14 allows "a refusal fixture" derived by "a single stated change", but AC-6 to AC-10 need about twenty distinct refusal inputs. The exactly-16-MiB valid input in AC-10 is neither a capture nor a single-change derivation, so it conflicts with FR-150-CON-1. Each derived fixture must also stay canonical, or it is refused for canonical form rather than for the member AC-7 names. Say "each refusal fixture is derived from a capture by one stated change and stays canonical", and allow a stated synthetic padding for the size boundary. | FR-150-AC-14, FR-150-CON-1 |
| FND-009 | medium | The inventory edit removes the differential-report **Sample** bullet, but FR-069-CON-4 and FR-069-AC-7 require every added adapter to name its sample. The edit also deletes the only row for tl-mltl's differential output, although tl-mltl has live producers: `tl-mltl.r2u2-past-differential/v1` and `tl-mltl.r2u2-future-differential/v1` (src/r2u2_live.rs:35-37) and `tl-mltl.campaign-run/v1` (src/campaign.rs:38). FR-069-AC-7 needs a real producer and a verdict for every scope item. Restore a Sample bullet naming the capture fixture, and keep a tl-mltl differential row with its own verdict. | docs/campaign-native-result-inventory.md:29, :53-57 |
| FND-010 | low | The no-producer argument says tl-mltl's own differential code writes `tl-mltl.differential/v1` and `v2`. It omits the r2u2 past and future differential reports and `campaign-run/v1`, which are tl-mltl's live differential outputs. The conclusion still holds, but the sentence understates what exists. | spec/functional/FR-150-differential-report-v2-adapter.md:36-42 |
| FND-011 | low | FR-069 still describes the case-list model the v2 adapter no longer has. Outputs say "one per ... compared case" (line 34). Behavior says "a differential that compared no case" (line 48). Error Conditions say "case id" (line 69). FR-069-AC-4 still requires an empty result set to be refused for both adapters, but a v2 report always holds exactly two engines and has no case list. Scope these to contract-conformance, or point to FR-150. | spec/functional/FR-069-campaign-native-adapters.md:34, :46-49, :68-71, FR-069-AC-4 |
| FND-012 | low | The inventory's "The state that does not fit" section (lines 59-79) still describes the v1 `unsupported` state and its printed line. "Three scope items get no adapter" (line 36) is now two: solver analysis reports moved to Covered. | docs/campaign-native-result-inventory.md:36, :59-79 |
| FND-013 | low | TC-001 opens a new `spec/test/` class. It has no index, and neither spec/index.md nor spec.md lists it, unlike `spec/integration/` (IT). The procedure is also unit-level adapter behaviour, which quoin otherwise traces through `quoin matrix` tags. Add the index entries, or drop TC-001 and rely on trace tags. | spec/test/TC-001-differential-report-v2-adapter.md, spec/index.md |

## Verdict

Not mergeable as written. FND-001 and FND-002 are contradictions between FR-150 and the owner
validation it adopts, so no implementation can satisfy the spec. FND-003 to FND-007 and FND-009
are real gaps.

What is sound: the replace-not-beside decision, the one-entry-per-report model, the symbol
`<analysisKind>::<requestDigest>`, the self-contradiction refusals of AC-9 (the owner refuses all
of them too), the tool claims (`--tool quire-analyze` does not collide with any existing claim
under the registry's substring match), no `traceIds`, and the no-execution and no-network
constraints. All fixes are spec-text changes inside this PR.

Examined (role examined): FR-150 statements, FR-150-CON-1, FR-150-CON-2, FR-150-AC-1 to AC-14,
NFR-032, TC-001, the FR-069 edits (Inputs, AC-2, AC-3), and the inventory and index edits.
Context only: quire-analyze FR-005 and FR-009, the v2 schema, quoin FR-033, the FR-069 remainder.
