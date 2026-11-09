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

Every pytest, vitest/jest and Rust emitter writes the mark below. The criteria
that bind emitters are allocated to the emitter repositories (see Acceptance
Criteria); this section is the shared contract they implement.

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
| Outcome | The case's final child element: `<error>` is `error`, `<failure>` is `fail`, `<skipped>` is `skip`, none is `pass`; the most severe wins. A `<flakyFailure>` or `<rerunFailure>` child records an earlier retry attempt and does not change the outcome, so a test that passed on retry is `pass` and one that failed its last attempt is `fail`. |
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
symbol set. The component that already holds that set, the matrix builder
(`matrix.build`, [FR-115](./FR-115-evidence-backed-test-matrix.md)), joins an
entry to a declared symbol by `path` and `qualified_name`. When an entry has no
`path` and its qualified name matches symbols in more than one file, the builder
reports the join as ambiguous and attaches the entry to none of those symbols.
The trace ids on the entry still bind the criteria they name, because the mark
is what binds; the symbol join only places the test in the source.

## Rationale

Without a shared contract each emitter would pick a different symbol spelling, and
the static and execution views would never join. The JUnit adapter already reads
the mark. Two quoin-side changes are needed: the adapter and `RunEntry` carry the
file path, and the matrix builder reports an ambiguous join.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-120-AC-1 | A `<testcase>` carrying `<property name="trace" value="FR-001-AC-1,TC-041"/>` records both ids on that case's entry; repeated properties accumulate and whitespace also separates ids. | Test |
| FR-120-AC-2 | A case whose `classname` equals its `name` records the name unchanged, including a name containing `.` or a space. | Test |
| FR-120-AC-3 | The JUnit adapter records a case's `file` attribute, and the normalized shape its `path` field, as the entry's path. | Test |
| FR-120-AC-4 | An entry with no `path`, whose qualified name matches symbols in more than one file, is reported ambiguous by the matrix builder and attached to none of them, while its trace ids still bind. | Test |
| FR-120-AC-5 | A failing, erroring or skipped tagged test produces an entry with that outcome and its ids, and that entry discharges nothing. | Test |
| FR-120-AC-6 | A case with a `<flakyFailure>` or `<rerunFailure>` child records the outcome of its final attempt. | Test |
| FR-120-AC-7 | Each emitter's reported `qualified_name` and `path` for a tagged test equal those of the same test in `quire coverage --json`, for a top-level, a nested and a parametrized test, including the rstest and `it.each` cases above. | Test (cross-repo: owned by each emitter repository's spec; PLAT-333 for Rust) |
| FR-120-AC-8 | A worked end-to-end run in one repository goes from source marker, to a run, to a result file with marks, to `quoin evidence record`, to a bound obligation, and a failing variant binds nothing. | Demonstration (cross-repo: the repository chosen for the example) |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-120-CON-1 | An emitter SHALL NOT write an id the source marker did not declare. | Design | Inspection |
| FR-120-CON-2 | An emitter SHALL NOT infer an id from a test name. | Design | Inspection |
| FR-120-CON-3 | The adapter SHALL NOT read the source tree to complete a symbol; it uses only the result file (FR-033-CON-1). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the adapter this extends), [FR-030](./FR-030-evidence-store.md) and [FR-115](./FR-115-evidence-backed-test-matrix.md).
- **Downstream**: [FR-121](./FR-121-emitter-mechanism-constraints.md); the pytest, vitest/jest and Rust emitters (PLAT-333 and its siblings).
