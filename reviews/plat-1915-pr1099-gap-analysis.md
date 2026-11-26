---
id: SR-967175026302197768900474181941935084897
title: "gap-analysis of quoin PR 1099"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1915; PR 1099; rust/crates/quoin-auditor/tests/tc_audit_ladder.rs; rust/crates/quoin-cli/tests/tc_fr034_sarif_record.rs; rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs; rust/crates/quoin-core/tests/tc_plat348_finding_and_sbom_record.rs; rust/crates/quoin-evidence/tests/tc_fr034_scan_store.rs; FR-034"
review_set: subset
---

## Summary

Reviewed PR #1099 at 3974321b630f8f765317de04425cc0ee7ae376eb. Seven new tests and one existing assertion provide nine precise FR-034 criterion bindings across auditing, recording and storage.

## Verdict

FAIL for repository assurance — 474 Test criteria remain untagged and 11 have ignored-only bindings. No new gap was found in the nine reviewed criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

Inspected AP-201 (spec/assurance/AP-201-finding-quality.md) and AP-202 (spec/assurance/AP-202-jev-lens-evaluation.md). AP-201's concern is concealed measurement gaps; the review preserves static binding versus run evidence and checks concrete assertions. No measurement/report production behavior or live Jev evaluation changes. Unavailable selected context: none. Active exceptions: none.

## Coverage

Plan completion: not assessed

The computed matrix and trace queries confirm exactly the nine targeted untagged-to-tagged transitions against the supplied base payload. Each binding was checked against its assertions and the relevant production route. All 27 FR-034 ACs were read as context; only the nine new bindings are claimed as examined here.

Reverse/stub procedure: inventoried 954 Rust files and 250 requirement records; the whole Rust tree has zero todo!/unimplemented! matches. Inspected auditor ladder pairing, clean/vacuous scan classification, SARIF and finding-adapter dispatch, evidence.record binding, CLI flag handoff, suite enumeration and timestamp-based GC/removal. These behaviors are owned by FR-034/FR-030/FR-033; no new production surface is introduced and no unowned behavior was found in the related flow. The test seams exercise real runtime or storage-port behavior; no hollow test or inflation was found. Full manual reverse ownership adjudication of the remaining repository is incomplete, and no repository-wide PASS is claimed. Optional semantic audit was not requested and did not run.

## Examined units and bindings

```yaml
scope:
- id: FR-034-AC-1
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A SARIF run with an empty `results` array is read as a scan that executed,
    reporting the tool and how many rules it evaluated.
- id: FR-034-AC-2
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: "A SARIF log whose `runs` array is empty is **rejected** \u2014 it proves\
    \ no scan executed."
- id: FR-034-AC-3
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: The SARIF adapter reads rule id, level, message and first location.
- id: FR-034-AC-4
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: The nested `rule.id` form is accepted; a result naming no rule at all is
    skipped.
- id: FR-034-AC-5
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: Malformed SARIF and a log with no `runs` array are rejected.
- id: FR-034-AC-6
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: The cargo-audit adapter parses `cargo audit --json` output, reading the
    advisory-database size as the rule count.
- id: FR-034-AC-7
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: Each cargo-audit warning kind is kept as its own severity rather than flattened.
- id: FR-034-AC-8
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: Malformed input and output that is not cargo-audit's are rejected; an advisory
    with no id is skipped.
- id: FR-034-AC-9
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: 'A clean scan discharges its binding: the obligation is neither `undischarged`
    nor `vacuous-evidence`.'
- id: FR-034-AC-10
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A scan that evaluated **no rules** is reported `vacuous-evidence` at `high`.
- id: FR-034-AC-11
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: When the tool reports no rule count the vacuity check stays silent.
- id: FR-034-AC-12
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: Every run-shaped check pairs a binding with **its own** suite's run when
    a scan is also bound to the same obligation.
- id: FR-034-AC-13
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: '`quoin evidence record --adapter sarif --results <file>` writes a `FindingRecord`
    under `scans/` and writes **nothing** under `runs/`.'
- id: FR-034-AC-14
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: 'A clean scan recorded through the command keeps `findings: []` and its
    rule count, so zero findings is still evidence.'
- id: FR-034-AC-15
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: The command selects a finding adapter from `--tool` when none is named.
- id: FR-034-AC-16
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: '`--discharges` binds the obligations a scan was run to check. A clean
    scan carries no finding to bind from, so they are stated rather than inferred.'
- id: FR-034-AC-17
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: A scan that evaluated **no rules** binds nothing, whatever `--discharges`
    names.
- id: FR-034-AC-18
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: A suite that recorded only scans is enumerated by `listRecordedSuites`.
- id: FR-034-AC-19
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: examined
  excerpt: '`gc` collects superseded scans, keeping the newest by timestamp.'
- id: FR-034-AC-20
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A tool reporting **zero** rules is distinguishable from a tool reporting
    **no** rule count.
- id: FR-034-AC-21
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: 'The cargo-deny adapter reads each `cargo deny --format json check` diagnostic
    as a finding: the advisory id (else the check''s `code`) as rule id, the tool''s
    own severity word, the message and the first crate of the dependency graph.'
- id: FR-034-AC-22
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A cargo-deny run that finished with a conforming summary and no diagnostic
    is a clean scan, reporting no rule count.
- id: FR-034-AC-23
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: cargo-deny output with no `summary` object, and a line that is not JSON,
    are rejected rather than read as a clean scan.
- id: FR-034-AC-24
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: '`quoin evidence record --tool cargo-deny` selects the cargo-deny adapter
    and writes a `FindingRecord` under `scans/`; `--tool cargo-audit` still selects
    the cargo-audit adapter.'
- id: FR-034-AC-25
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A cargo-deny summary counting more errors and warnings than the diagnostics
    read is refused, so a failed run printed under `--log-level off` is not a clean
    scan.
- id: FR-034-AC-26
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A cargo-deny summary must name at least one check, appear once and last,
    and the checks it names are recorded as the ruleset; a diagnostic with no id is
    kept as `unclassified`.
- id: FR-034-AC-27
  path: spec/functional/FR-034-finding-shaped-evidence.md
  role: context_only
  excerpt: A `FindingRecord` written with an `evaluated` list keeps it on read and
    gc, and one written without it reads back without the field and with no claim
    of coverage.
bindings:
- test_id: tc_fr034_a_clean_scan_is_neither_undischarged_nor_vacuous
  ac_id: FR-034-AC-9
  trace: correct
- test_id: tc_fr034_run_checks_stay_with_their_suite_beside_a_scan_binding
  ac_id: FR-034-AC-12
  trace: correct
- test_id: tc_fr034_clean_sarif_records_scan_with_rule_count_and_no_run
  ac_id: FR-034-AC-13
  trace: correct
- test_id: tc_fr034_clean_sarif_records_scan_with_rule_count_and_no_run
  ac_id: FR-034-AC-14
  trace: correct
- test_id: tc_2703_a_cargo_deny_scan_records_under_its_own_tool
  ac_id: FR-034-AC-15
  trace: correct
- test_id: tc_458_151_a_clean_scan_binds_its_stated_discharges
  ac_id: FR-034-AC-16
  trace: correct
- test_id: tc_458_152_a_vacuous_scan_does_not_bind_stated_discharges
  ac_id: FR-034-AC-17
  trace: correct
- test_id: tc_fr034_scans_only_suites_are_enumerated
  ac_id: FR-034-AC-18
  trace: correct
- test_id: tc_fr034_gc_keeps_the_newest_scan_by_timestamp
  ac_id: FR-034-AC-19
  trace: correct
```

## Checks

Independent cargo fmt --all --check and git diff --check passed. The coder's current-head compile/run log at /tmp/plat1915-quoin-fr034-final-focused.log reports 8 selected tests passing: auditor 2, CLI 1, core record 1+1, existing cargo-deny 1, and evidence-store 2. The reviewer attempted the same compiled-artifact selection through locked-build.sh. It remained queued while one-minute load was 21.30 (admission threshold under 18), and was cancelled before test launch (exit 130). No independent test pass or fresh rebuild is claimed; the completed current-head coder run is the test evidence. Full workspace gates were not run under the owner’s focused-only campaign rule.

Quoin 0.37.0 and Quire 0.52.0/engine 0.62.2 were used for review tooling. The fresh unique-ID exact-Test matrix is 1,721 tagged, 474 untagged and 11 ignored-only repo-wide. Comparing the supplied clean bff63d6d matrix with the fresh head matrix shows exactly AC-9/12/13/14/15/16/17/18/19 changing untagged to tagged, with no reverse transitions. All nine resolve to the expected test verifiers. The supplied older-file-date cohort is 351 to 342; its full denominator was not independently reconstructed. Six pre-existing cfg-twin trace collisions remain unrelated to the changed files. Static tags are distinct from bound execution evidence.

The branch codex/quoin-fr034-evidence-gaps and title test(quoin): cover FR-034 evidence gaps omit PLAT-1915; the body contains Refs PLAT-1915 and no closing directive. The lead explicitly assigned this campaign ticket, so it was not inferred from branch/title. There are no production changes, dependency pin moves, new interface types or workflow edits. Every new test has the criterion it actually asserts; the pre-existing cargo-deny assertion receives AC-15 alongside its existing AC-24 marker. Test helpers add no production behavior and need no implements marker.

## Artifact retention

Commit this scratchpad file as reviews/plat-1915-pr1099-gap-analysis.md after review, preserving prior batch artifacts. The reviewer makes no branch edits or pushes.
