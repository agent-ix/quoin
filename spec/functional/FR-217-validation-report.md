---
id: FR-217
title: "Generate a per-release test report from retained evidence"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-236"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-185"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-219"
    type: "depends_on"
---

# FR-217: Generate a per-release test report from retained evidence

## Description

The quoin CLI SHALL generate a per-release test report from retained normal
suite run records and requirement evidence, so a reader can see the exact tool
version tested, test outcomes, requirement coverage, uncovered requirements,
and known defects. The report SHALL distinguish test results, requirement
coverage, evidence completeness, and authored risk ratings.

## Inputs

- One or more retained normal-suite run records for the release
- The declared suite scope and the required run records for that scope
- The Quoin evidence-backed matrix for the same repository and recorded runs
- The applicable intended-use and feature-risk-rating records
- The external-tools configuration index reference, when applicable
- The Engineering Assurance `producer-execute` result and retained artifacts,
  when that command executed a suite process
- The required sibling checkouts `agent-ix/spec-objects-safety` and
  `agent-ix/spec-objects-security` used by the suite
- The writable temporary directory used by the suite

## Outputs

- `validation-report.md`
- `validation-report.json`
- Test results, the exact tool version observed during each run, requirement
  coverage, uncovered requirements, and known defects for the reported release

## Behavior

- The command `quoin validation report --results <file>... --out <dir>` SHALL
  write both report formats to the requested directory.
- A report for a released build SHALL identify the normal suite run or runs
  that produced the report. It SHALL report the full normal suite for that
  release, including library-linked tests as internal release evidence.
  Customer self-test runs SHALL use only the version-and-target-matched
  standalone runner assets that invoke the installed command, as described by
  FR-235; they do not replace the normal-suite results in the release report.
- A self-test asset SHALL be a per-version, per-target GitHub Release file.
  The self-test runner selects it from the installed command's exact
  `--version` output and host target; when necessary it fetches the matching
  file. The report SHALL show that exact version. It does not require a signed
  asset manifest or asset hashes.
- The report SHALL contain the exact tool version output observed for each
  run, the operating system and architecture, suite identity, run time, and
  each test result and status.
- For each suite run, the report environment SHALL identify the required
  sibling checkouts `agent-ix/spec-objects-safety` and
  `agent-ix/spec-objects-security`. For each checkout, it SHALL record the
  canonical repository identity, observed commit revision, and resolved
  absolute checkout path.
- The report environment SHALL record the resolved path of the temporary
  directory used by the suite and whether that directory was writable when the
  suite began, as captured by suite execution. The report generator SHALL NOT
  infer that value later.
- For Rust suites that emit JUnit, the normal-suite workflow SHALL use the
  existing evidence path: `cargo nextest` writes JUnit,
  `quoin-suite-trace` adds the source test trace tags, and
  `quoin evidence record --adapter junit` retains the result.
- The report SHALL consume the retained Quoin run evidence.
- The report MAY include the in-toto Test Result produced by
  `quoin evidence export-test-result`.
- The report SHALL derive requirement coverage from the Quoin matrix and the
  retained results for the reported suite. It SHALL identify requirements and
  acceptance criteria with no passing, trace-linked result as uncovered, and
  show failed, errored, skipped, and not-executed results separately. A result
  status SHALL NOT be inferred from a risk rating.
- The report SHALL include applicable `FeatureRiskRating` values and their
  authored status as descriptive metadata. A rating SHALL NOT select tests,
  require a dedicated self-test case, or determine requirement coverage or
  evidence completeness.
- When Engineering Assurance `producer-execute` ran the suite process, the
  report SHALL preserve the producer result and artifact references from the
  retained run record details. That result records process execution and
  artifacts; it does not map individual tests to requirements, so requirement
  coverage SHALL come from the trace-linked test results and Quoin evidence
  matrix.
- The report SHALL name known defects from the version-matched anomaly records
  retained with the run. It SHALL NOT substitute anomaly records from the
  report generator's current build.
- If no matching anomaly set was supplied, the report SHALL say
  `not provided`.
- If the run supplied a matching empty anomaly set, the report SHALL say
  `none known`.
- The report SHALL identify the retained run records and producer results it
  consumes.
- For each FR, the coverage row SHALL preserve the supplier-rating status and
  all supplier rating record IDs returned by feature-rating assessment. When
  more than one supplier record rates an FR, the report SHALL show
  `rated_more_than_once` and SHALL NOT collapse the records by their rating
  values.
- Each retained run record SHALL preserve the anomaly records embedded in the
  binary that produced that run, together with whether the set was provided
  and the producer version. The producer version SHALL equal the run record's
  `toolVersion`.
- The report SHALL resolve the `Known defects` section from the anomaly sets
  retained with its run records and SHALL NOT substitute anomaly records
  embedded in the report generator's current build.
- When an anomaly set's producer version does not match its run record's
  `toolVersion`, the report SHALL treat that set as `not provided`.
- Quoin SHALL embed only its own `Anomaly` records from `spec/anomalies/` in
  its binary at build time.
- The quoin validation report generator SHALL render the `Known defects`
  section from the selection returned by `quoin_anomaly::build_report`,
  preserving FR-162's carried-anomaly selection. Each row SHALL state `Tool`,
  `Anomaly`, `Title`, `Severity`,
  `Safety impact`, `Affected FRs`, and `Status`, using authored values and
  `not stated` where a value is absent. A `safety_impact` of `unassessed` SHALL
  be shown as `unassessed`.
- A row whose assessment targets an FR rated `high_process_risk` SHALL be
  marked `affects high-process-risk feature` and SHALL appear before rows that
  do not target such an FR. Rows within each group SHALL be ordered by anomaly
  id. The section SHALL not report a score.
- When a run record carries an anomaly set and the selection is empty, the
  section SHALL state `none known`.
- When a run record does not carry an anomaly set or its producer version does
  not match, the section SHALL state `not provided`.
- Each tool binary SHALL embed its own anomaly records.
- A CLI that links an engine MAY also embed anomaly records exposed by that
  engine.
- A CLI SHALL NOT embed anomaly records from unrelated tools.
- The report SHALL preserve each case's `pass`, `fail`, `error`, or
  `not-executed` status.
- If a required run record is absent, the report SHALL state that it was not
  provided. [FR-220](./FR-220-external-tool-configuration-index.md) defines
  how the report resolves external-tool index evidence.
- The report SHALL list uncovered requirements independently of rating.
- The report SHALL NOT present missing test evidence as a passing result.
- The report SHALL state the exact version under test. It SHALL NOT require a
  signed test-asset manifest or store test-asset hashes.
- When the report generator receives the same retained run records and rating
  inputs again, it SHALL produce byte-identical Markdown and JSON files.
- The report SHALL contain a `Sign-off` section for a named person's
  statement.
- The report generator SHALL NOT make that statement for the person.
- The report SHALL NOT state that the tool is compliant or accepted for a
  customer's intended use.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-217-AC-1 | Given retained normal-suite run records for two tools and their Quoin matrix results, the command writes valid Markdown and JSON containing each exact tool version, environment, individual test results, requirement coverage, uncovered requirements, and the declared risk-rating metadata. | Test |
| FR-217-AC-2 | Given a missing required run record, the report says `not provided` for that evidence and does not mark it as passing. | Test |
| FR-217-AC-3 | Given unchanged retained run records and rating inputs, two report invocations produce byte-identical Markdown and JSON. | Test |
| FR-217-AC-4 | The generated report contains a `Sign-off` section and does not contain a generated acceptance or compliance decision. | Test |
| FR-217-AC-5 | Given a version-matched retained anomaly set with an open anomaly assessed against a high-rated FR, an open anomaly assessed against a not-high-rated FR, a closed `fixed` anomaly, and a closed `wont-fix` anomaly, the `Known defects` section renders both open anomalies and the closed `wont-fix` anomaly, highlights the high-rated row first, and omits the closed `fixed` anomaly. | Test |
| FR-217-AC-6 | Given a retained empty anomaly set whose producer version matches its run record, the `Known defects` section states `none known`. | Test |
| FR-217-AC-7 | Given a carried anomaly with `safety_impact: unassessed`, the `Known defects` section renders `unassessed` and does not supply a default value. | Test |
| FR-217-AC-8 | Given a version-matched CLI anomaly set containing CLI and linked-engine records, the section lists those retained records and does not substitute records from the report generator's current build or an unrelated tool. | Test |
| FR-217-AC-9 | Given a run record with no retained anomaly set or a set whose producer version differs from `toolVersion`, the `Known defects` section states `not provided` and does not use the report generator's build-time set. | Test |
| FR-217-AC-10 | Given two supplier rating records for one FR with equal rating values, the coverage row preserves both rating IDs and reports `rated_more_than_once`. | Test |
| FR-217-AC-11 | Given equivalent run records and requirement mappings for two requirements with different risk ratings, the report selects the same tests and computes the same coverage states for both; it displays each rating only as authored metadata. | Test |
| FR-217-AC-12 | Given a retained test result whose detail records the bounded Engineering Assurance `producer-execute` result and its artifact references, the report preserves that detail, while deriving test-to-requirement coverage only from the test result's trace-linked criteria; artifact details alone do not establish coverage. | Test |
| FR-217-AC-13 | Given a run environment containing both required sibling checkouts, the report records each canonical repository identity, commit revision, and resolved absolute path, plus the resolved temporary-directory path and its captured writable status. | Test |
| FR-217-AC-14 | Given either required checkout or the temporary-directory write check is unavailable, the report records the unavailable checkout identity or `writable: false`; it preserves test outcomes and requirement coverage as separate facts. | Test |

## Dependencies

- **Upstream**: [FR-236](./FR-236-selftest-run-record.md) defines retained results; [FR-030](./FR-030-evidence-store.md) defines the evidence store.
- **Execution evidence**: [FR-219](./FR-219-validation-report-tool-evidence.md) defines retained producer execution evidence.
- **Anomaly selection**: [FR-162](./FR-162-anomaly-report.md) defines which anomaly records a release carries; records are defined by `spec-artifacts-process` FR-014, FR-015 and FR-016.
- **Downstream**: [FR-218](./FR-218-validation-report-completeness-and-signoff.md) defines report completeness and signature verification.
