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

A test runner emitter SHALL write each test's trace ids into the result output as
a mark on that test's case, and `quoin evidence record` SHALL read the mark
without any input other than the result file. The contract below is shared by the
pytest, vitest/jest and Rust emitters, and the marker declarations that name the
source tags (`trace_tags`) belong to `spec-artifacts-process`, not to this
requirement. [ADR-001](../adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md)
records why the ids are emitted rather than matched afterward.

### The mark

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
| Outcome | The case's child element: `<error>` is `error`, `<failure>` is `fail`, `<skipped>` is `skip`, none is `pass`. The most severe child wins. The emitter never writes an outcome the runner did not report. |
| Normalized JSON | A runtime no adapter reads writes `{"entries":[{"symbol","outcome","traceIds"}]}`; the same symbol rules apply. |

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
| Rust | In-file `mod` nesting joined by `::`, then the function: `inner::name`. Neither the crate name nor the file stem appears, so `module_path!()` is not the symbol. A test in `tests/x.rs` is `name`; the path is `tests/x.rs`. |
| Python | Bare `name` at module level, `Class.method` under a class. The module is never a prefix. |
| vitest/jest | The registered title alone. An enclosing `describe` does not qualify its members; a class qualifies as `Class.method`. |

Awkward cases, settled once here:

- **Parametrized tests** (`test_foo[case-1]`): the symbol is the function, with the
  parameter suffix removed. Each case is its own `<testcase>` with that symbol,
  and a failing case is a failing entry for the symbol.
- **Same qualified name in two files:** the `file` attribute is what separates
  them. A result without `file` whose qualified name matches symbols in more than
  one file is reported as unjoinable and binds to none of them.
- **Nested suites:** the title alone is the symbol, per the table.

## Rationale

Without a shared contract each emitter would pick a different symbol spelling,
and the static and execution views would never join. The JUnit adapter already
reads the mark; the one thing it does not yet read is the file path, which is the
only quoin-side change this contract needs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-120-AC-1 | A `<testcase>` carrying `<property name="trace" value="FR-001-AC-1,TC-041"/>` records both ids on that case's entry; repeated properties accumulate and whitespace also separates ids. | Test |
| FR-120-AC-2 | A case whose `classname` equals its `name` records the name unchanged, including a name containing `.` or a space. | Test |
| FR-120-AC-3 | The JUnit adapter records a case's `file` attribute as the entry's path. | Test |
| FR-120-AC-4 | A case with no `file`, whose qualified name matches symbols in more than one file, is reported unjoinable and binds to none. | Test |
| FR-120-AC-5 | Each emitter's reported symbol for a tagged test equals the `qualified_name` and `path` of the same test in `quire coverage --json`, for a top-level test, a nested one, and a parametrized one. | Test |
| FR-120-AC-6 | A failing, erroring or skipped tagged test produces an entry with that outcome and its ids, and the obligation it names is not discharged. | Test |
| FR-120-AC-7 | Editing the requirement a recorded entry is bound to makes the binding stale in the auditor's report. | Test |
| FR-120-AC-8 | A worked end-to-end run in one repository goes from source marker, to a run, to a result file with marks, to `quoin evidence record`, to a bound obligation. | Demonstration |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-120-CON-1 | An emitter SHALL NOT write an id the source marker did not declare. | Design | Inspection |
| FR-120-CON-3 | An emitter SHALL NOT infer an id from a test name. | Design | Inspection |
| FR-120-CON-2 | The adapter SHALL NOT read the source tree to complete a symbol; it uses only the result file (FR-033-CON-1). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the adapter this extends) and [FR-030](./FR-030-evidence-store.md).
- **Downstream**: [FR-121](./FR-121-emitter-mechanism-constraints.md); the pytest, vitest/jest and Rust emitters (PLAT-333 and its siblings).
