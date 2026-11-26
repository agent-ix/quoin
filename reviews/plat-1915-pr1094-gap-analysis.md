---
id: SR-967175026302197768900474181941935084888
title: "gap-analysis of quoin PR 1094"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1915; PR 1094; rust/crates/quoin-cli/tests/tc_1915_fr033_evidence_record.rs; rust/crates/quoin-evidence/tests/tc_adapters_behaviour.rs; FR-033"
review_set: subset
---

## Summary

Reviewed PR #1094 at 3acbd0a3bc5b533c652ce10cac2ee41f9a288f07. Four new tests correctly assert adapter registration and real CLI run-record preservation/selection for FR-033-AC-3, AC-11, AC-12 and AC-13.

## Verdict

FAIL for repository assurance — 498 Test criteria remain untagged and 11 have ignored-only bindings. The four FR-033 additions introduce no PR gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

Inspected AP-201 (`spec/assurance/AP-201-finding-quality.md`) and AP-202 (`spec/assurance/AP-202-jev-lens-evaluation.md`). This test-only diff changes neither finding measurement/report behavior nor live Jev evaluation. Unavailable selected context: none. Active exceptions: none.

## Coverage

Plan completion: not assessed

The computed 0.37.0 repository matrix has 2,215 Test criteria: 1,706 tagged, 498 untagged and 11 tagged only by ignored tests. All 17 FR-033 acceptance criteria are statically tagged; the four new bindings were checked against their actual assertions. Static tagging is not a claim that all 17 criteria were independently behaviorally audited or that run evidence is bound. The four rows move untagged to tagged against the supplied clean-main 0.37.0 matrix.

Reverse/stub procedure: inventoried 951 Rust source/test files and 250 requirement records; scanned the whole Rust tree for todo! and unimplemented! (zero matches). Inspected the seven-run-adapter registry, the finding/run selection boundary, select_adapter, the JUnit/mutation parser dispatch and CLI-to-core-to-store flow. Those behaviors are owned by FR-033/FR-030; the related finding-shaped OSV path is FR-149 and remains distinct. No new production behavior exists in the diff, no new unowned behavior was found in the related flow, and no changed test replaces production code, skips an assertion, or has an empty body. Full manual ownership adjudication of the remaining repository is incomplete; no repository-wide PASS is claimed. The optional semantic audit was not requested and did not run.

## Examined units and bindings

```yaml
scope:
- id: FR-033-AC-1
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: examined
  excerpt: An adapter is selected by `--adapter`, else by the suite's declared `--tool`,
    else the normalized `entries` shape.
- id: FR-033-AC-2
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: An unknown `--adapter` is an error naming the available adapters, never
    a silent fall back.
- id: FR-033-AC-3
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: examined
  excerpt: Adapters are registered as data, each carrying a name, a summary and a
    parse function.
- id: FR-033-AC-4
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: JUnit `classname` + `name` map to the qualified name the symbol extractor
    emits, without doubling a classname that already ends with the member.
- id: FR-033-AC-5
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: "Every JUnit outcome class is read \u2014 `error` outranks `failure` outranks\
    \ `skipped` \u2014 along with declared trace ids."
- id: FR-033-AC-6
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: The JUnit adapter names **no** evidence kind, because the format does not
    carry one.
- id: FR-033-AC-7
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: Input carrying no `<testcase>` is an error, not an empty run; a case with
    neither `classname` nor `name` is skipped.
- id: FR-033-AC-8
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: The cargo-mutants adapter produces a per-function `score`, with unviable
    mutants in neither side of the ratio.
- id: FR-033-AC-9
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: Outcome reflects the **tool's own** classification, never a threshold.
- id: FR-033-AC-10
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: Malformed, empty and unattributable mutation reports are rejected; a timed-out
    mutant counts as survived.
- id: FR-033-AC-11
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: examined
  excerpt: '`quoin evidence record --adapter junit --results <file>` records a JUnit
    file end to end, preserving symbols and trace ids.'
- id: FR-033-AC-12
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: examined
  excerpt: '`quoin evidence record --adapter cargo-mutants` records a mutation report
    end to end, preserving `score`.'
- id: FR-033-AC-13
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: examined
  excerpt: '`quoin evidence record` selects the adapter from `--tool` when none is
    named.'
- id: FR-033-AC-14
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: '`quoin evidence record` still accepts the normalized shape with no adapter
    at all.'
- id: FR-033-AC-15
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: 'Interim, removed when the PLAT-333 Rust emitter lands (ADR-100): a cargo-nextest
    JUnit report annotated by `quoin-suite-trace` carries, on each tagged test case,
    a `trace` property naming that test''s acceptance criteria, so `quoin evidence
    record --adapter junit` binds the run to them.'
- id: FR-033-AC-16
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: A case recorded with a `--tool` containing `pytest` (case-insensitive,
    for example `pytest 8.3.2`) is stored under the symbol the Python extractor emits.
    `classname="tests.test_bolus" name="test_basic"` is `test_basic`; `classname="tests.test_bolus.TestDose"
    name="test_a[1]"` is `TestDose.test_a`; `classname="tests.TestA.Inner" name="test_b"`
    is `TestA.Inner.test_b`; `classname="test_basic" name="test_basic"` is `test_basic`,
    not `test_basic.test_basic`; an empty `classname` gives the name alone. The same
    case recorded under another tool is stored by the `::` rule.
- id: FR-033-AC-17
  path: spec/functional/FR-033-evidence-format-adapters.md
  role: context_only
  excerpt: A pytest case that carries no `trace` property is recorded with its outcome
    and no `traceIds`. The run it belongs to is an error under FR-030-AC-33 when no
    case carries one, and a case whose id no obligation states is listed as unmatched
    under FR-030-AC-32.
bindings:
- test_id: tc_1915_each_run_adapter_is_registered_with_its_summary_and_parser
  ac_id: FR-033-AC-3
  trace: correct
- test_id: tc_1915_junit_record_keeps_the_extracted_symbol_and_trace_id
  ac_id: FR-033-AC-11
  trace: correct
- test_id: tc_1915_cargo_mutants_record_keeps_the_per_function_score
  ac_id: FR-033-AC-12
  trace: correct
- test_id: tc_1915_tool_name_selects_the_run_adapter_when_adapter_flag_is_absent
  ac_id: FR-033-AC-13
  trace: correct
```

## Checks

Independent cargo fmt --all --check and git diff --check passed. Independently executed the coder-built test artifacts through locked-build.sh with CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0 and CARGO_INCREMENTAL=0: adapter registration filter 1/1, all new CLI tests 3/3, and OSV storage-family regression filter 1/1; zero failures or ignored tests in those selections. This rerun reused compiled artifacts from target-plat1915-quoin-batch2 rather than rebuilding after generated node_modules had been removed. The coder reports the corresponding cargo test commands passed and npmjs-only make build-inputs succeeded. No reviewer build-input mutation was needed. Broad workspace test/lint/deny gates were not run under the owner's focused-only campaign rule; there is no production or dependency change.

Fresh matrix used the coder-identified npmjs-cached @agent-ix/quoin@0.37.0 executable at /tmp/npm-cache-codex-gaps/_npx/686d6a78c19c9673/node_modules/.bin/quoin (version verified), because PATH quoin resolves to 0.36.8. Quire is 0.52.0, engine 0.62.2. The 0.37.0 matrix contains 498 untagged Test criteria, while 0.36.8 reports 488; version-specific totals are not compared as progress. Comparison of the coder's clean-main 0.37.0 payload with the fresh 0.37.0 head payload shows exactly AC-3, AC-11, AC-12 and AC-13 changing untagged to tagged, with no new criteria or reverse transitions. FR-033 was first added on 2026-08-18, so these four rows qualify for the older cohort. The supplied older-cohort totals are 290 to 286 untagged, with two ignored-only unchanged; the full older-cohort denominator was not independently reconstructed.

Both quire trace file queries exit 0 and resolve the four new verifies claims. They emit existing cfg-platform symbol-id-collision warnings in measurement/campaign, selftest, completeness snapshot equivalence, config lock/service and store bounded-read; none is a changed path and none prevented the four claims resolving.

## Artifact retention

The eventual repository path is reviews/plat-1915-pr1094-gap-analysis.md. The PR-specific name preserves earlier PLAT-1915 batch review files rather than overwriting them. The reviewer writes only this scratchpad artifact; the lead or coder commits it.
