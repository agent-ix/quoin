---
id: FR-120
title: "Trace ids are marks in test result output"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-023"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
---

# FR-120: Trace ids are marks in test result output

## Description

When `quoin evidence record` reads a test result file, it SHALL take each case's
trace ids from the mark on that case, using only the file.

This requirement extends the adapter contract of
[FR-033](./FR-033-evidence-format-adapters.md) with the mark and with the file
path, and it supersedes the post-run `quoin-suite-trace` annotator of
FR-033-AC-15, which stays only as an interim until the emitters below land
([ADR-100](../adrs/ADR-100-execution-evidence-is-emitted-by-the-run.md)). The
marker declarations that name the source tags (`trace_tags`) belong to
`spec-artifacts-process`, not to this requirement.

### What an emitter writes

Every pytest, vitest/jest and Rust emitter writes the mark below, under the
emitter contract at the end of this requirement.

In JUnit XML the mark is a `<property>` child of `<testcase>`:

```xml
<testcase classname="tests::coverage" name="tests::coverage" file="tests/coverage.rs">
  <property name="trace" value="FR-001-AC-1,TC-041"/>
</testcase>
```

| Element | Rule |
| --- | --- |
| Property name | `trace`. The JUnit adapter also reads `traceIds` and `trace_ids`; an emitter writes `trace`. |
| Value | One or more ids separated by commas or whitespace. |
| Repetition | Several `<property>` elements on one case accumulate. |
| Outcome | The runner's own result for the case, read from its final child element: `<error>` is `error`, `<failure>` is `fail`, `<skipped>` is `skip`, none is `pass`; the most severe wins. A `<flakyFailure>` or `<rerunFailure>` child records an earlier retry attempt and does not change the outcome, so a test that passed on retry is `pass` and one that failed its last attempt is `fail`. |
| Normalized JSON | A runtime no adapter reads writes `{"entries":[{"symbol","path","outcome","traceIds"}]}`. `path` is the same repo-relative path the JUnit `file` attribute carries, and the `RunEntry` type gains an optional `path` field to hold it. |

### The symbol must equal the extractor's

`quire coverage --json` identifies a test as `{language, path, qualified_name}`.
The emitter reports the extractor's `qualified_name` as `name`, repeats it as
`classname`, and reports `path` as the case's `file` attribute (repo-relative,
`/`-separated). With `classname` equal to `name`, the adapter's qualified-name
rule returns the name unchanged, so no dot rewriting reaches a name the extractor
spelled with dots or spaces. The runner's own default naming is never assumed to
agree.

| Runtime | `qualified_name` the emitter reports |
| --- | --- |
| Rust | The extractor's container chain joined by `::`: each enclosing `mod`, `struct`, `enum` or `trait`, the `impl` target type (never the trait), and an enclosing `fn`, then the function. A test in `mod tests` is `tests::name`. Neither the crate name nor the file stem appears, so `module_path!()` is not the symbol. A test in `tests/x.rs` is `name` and its path is `tests/x.rs`. |
| Python | Bare `name` at module level, `Class.method` under a class. The module is never a prefix. |
| vitest/jest | The registered title exactly as written in source. An enclosing `describe` does not qualify its members; a class qualifies as `Class.method`. |

Awkward cases, settled once here:

- **Parametrized tests:** the symbol is the source function (or, for vitest/jest,
  the literal title in source), never the runner's per-case spelling. A pytest
  `test_foo[case-1]` reports `test_foo`; an rstest `name::case_1` reports `name`;
  a vitest/jest `it.each` or `test.each` reports its template title as written
  (for example `adds %s`), not the interpolated one. Each case is its own
  `<testcase>` carrying that symbol, and a failing case is a failing entry for
  the symbol.
- **Same qualified name in two files:** the `file` attribute separates them.
  A result without `file` is still recorded with its ids.
- **Nested suites:** the title alone is the symbol, per the table.

### Who joins symbols

The JUnit adapter stays pure over the result text and never learns the static
symbol set. The evidence store owns the join of a run entry to a declared symbol
by `path` and `qualified_name`, and the ambiguity report for an entry that has no
`path`: [FR-030](./FR-030-evidence-store.md), criterion FR-030-AC-18. The trace
ids on an entry bind the criteria they name regardless of the symbol join.

## Rationale

Without a shared contract each emitter would pick a different symbol spelling, and
the static and execution views would never join. The JUnit adapter already reads
the mark. Two quoin-side changes are needed: the adapter and `RunEntry` carry the
file path, and the evidence store reports an ambiguous join (FR-030-AC-18).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-120-AC-1 | A `<testcase>` carrying `<property name="trace" value="FR-001-AC-1,TC-041"/>` records both ids on that case's entry; repeated properties accumulate and whitespace also separates ids. | Test |
| FR-120-AC-2 | A case whose `classname` equals its `name` records the name unchanged, including a name containing `.` or a space. | Test |
| FR-120-AC-3 | The JUnit adapter records a case's `file` attribute, and the normalized shape its `path` field, as the entry's path. | Test |
| FR-120-AC-4 | The join of an entry to a symbol, and the ambiguity report for an entry with no path, are those of FR-030-AC-18; the adapter performs neither and reads no source file. | Inspection |
| FR-120-AC-5 | A failing, erroring or skipped tagged test produces an entry with that outcome and its ids, and that entry discharges nothing. | Test |
| FR-120-AC-6 | A case with a `<flakyFailure>` or `<rerunFailure>` child records the outcome of its final attempt. | Test |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-120-CON-1 | The adapter SHALL NOT read the source tree to complete a symbol; it uses only the result file (FR-033-CON-1). | Design | Inspection |

## Emitter contract

This section binds the emitters. It is normative for the repositories that own
them, and each of those repositories' own specs must carry criteria satisfying
it: `quire-rs` and `ix-trace-rs` for the Rust emitter (PLAT-333), the vitest/jest
helper (PLAT-334), and the pytest plugin (PLAT-335). Quoin tests none of it.

| Rule | Requirement |
| --- | --- |
| E-1 Marks | An emitter SHALL write each tagged test's mark in the form above and in no other carrier; a different carrier needs an amendment to this requirement first. |
| E-2 Symbols | An emitter SHALL report, for each tagged test, the `qualified_name` and `path` that `quire coverage --json` reports for the same test, including a top-level, a nested and a parametrized test (rstest cases and `it.each` titles as above). |
| E-3 Declared ids | An emitter SHALL write only ids the source marker declared, and SHALL NOT infer an id from a test name. |
| E-4 Outcome | An emitter SHALL NOT write an outcome the runner did not report. A test that panics or throws is recorded `fail` with its ids, never absent. |
| E-5 Once | Under the runner's default parallel execution, an emitter SHALL record each tagged test once, with its ids unmixed with another test's. |
| E-6 Inert | While emission is not enabled, an emitter SHALL leave compiled output byte-identical to a build without the emitter and SHALL write no `trace` property. Enabling is an explicit switch (a cargo feature, an environment variable, or the runner's equivalent), never the mere presence of the marker, and the emitter adds no runtime dependency to a build that does not enable it. |
| E-7 Cost | While emission is enabled, an emitter SHOULD add under 2% and SHALL add under 5% to the median wall-clock time of a suite. |
| E-8 No rewrite | The run's own harness SHALL write the final result file. No sidecar file is merged afterward and no post-run step rewrites names or ids. |
| E-9 Rust shape | The Rust run SHALL write JUnit in which each case carries `name` and `classname` equal to the extractor's `qualified_name`, a `file` attribute and the `trace` property. The runner or harness that meets this shape, whether a custom harness or a nextest feature confirmed to provide it, is chosen in PLAT-333. |
| E-10 Worked example | One repository SHALL demonstrate source marker, run, result file with marks, `quoin evidence record`, and a bound obligation, plus a failing variant that binds nothing. |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the adapter this extends), [FR-030](./FR-030-evidence-store.md) and - **Downstream**: the pytest plugin (PLAT-335), the vitest/jest helper (PLAT-334) and the Rust emitter (PLAT-333), each in its own repository.
