---
id: FR-033
title: "Evidence format adapters"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---
# FR-033: Evidence format adapters

## Description

`quoin evidence record --results` parsed exactly one hardcoded shape:
`{entries: [{symbol, outcome, traceIds}]}`. There was no dispatch, no registry, and no plugin
point, so every consumer had to transform their tool's output by hand before quoin would read it.

quoin SHALL accept an **adapter**: a pure function from raw tool output to run entries.

### The contract is narrow on purpose

| Rule | Why |
|---|---|
| **Pure over text.** No filesystem beyond the input, no subprocess, no network. | An adapter that could run the tool would make `quoin evidence record` a test runner, and a transcript nobody can trust. ADR-0011 invariant 1: quoin transcribes; the consumer's CI executes. |
| **No verdict.** An adapter reports what the tool said. | Whether a result is *acceptable* is the auditor's and the consumer gate's decision. An adapter that synthesized pass/fail from a threshold would leak verdict policy into the intake layer, where nobody would look for it. |

### The registry is data, and an unknown name is an error

Adapters are registered in a list keyed by the tool identifiers they claim, matched against the
suite's declared `tool`. `--adapter` overrides. Nothing is hardcoded to `agent-ix` repositories,
and the normalized shape stays available as `entries` so a consumer whose tool no adapter reads can
still write evidence by hand — the registry is a convenience, never a gate on recording.

An **unknown** `--adapter` SHALL be an error naming the available adapters, never a silent fall back
to the default. Falling back would parse a JUnit file as normalized JSON and fail with a complaint
about JSON shape, sending the reader to look at their XML instead of at their typo.

### An adapter does not name the evidence kind

`evidenceKind` exists on the result but the shipped adapters leave it unset, and that is a decision
rather than an omission.

**A JUnit XML file is emitted by unit, integration and end-to-end suites alike.** An adapter
answering "Unit" would assert something the format does not contain. The kind is declared by the
suite registry's `Evidence Kind` column and passed with `--kind`; that is where the vocabulary lives
and where it stays. Minting a mapping here would create a **fourth** copy of a vocabulary that
already exists in the catalog, the Test Matrix `Type` column, and the suite registry — in a
different repository, where the two tests holding the first three honest cannot see it.

The field remains on the contract because an external adapter for a genuinely single-purpose tool
may legitimately know.

### Three formats, three corners of the contract

- **JUnit XML** — the common case. The real work is `classname` + `name` → the qualified name the
  symbol extractor emits (`tests::corpus::tc001`). A tool's own test name is **not** a symbol
  identity; an adapter that skipped this would write a store whose entries match no declared symbol,
  and every obligation would read as unmatched while looking recorded.
- **cargo-mutants** — the only format with a native `score`, and therefore the only proof that the
  contract carries more than pass/fail. Designing it without exercising `score` would design it
  blind.
- **lcov / llvm-cov — deliberately NOT in this FR.** See below.

### Why coverage is not a `RunEntry`

The ticket asked this to be resolved rather than assumed, and the answer is that it does not fit.

`RunEntry.symbol` is *a test symbol*: the join between what a tool reports and what the matrix
declares. **Coverage is a measurement about production code**, not an outcome of a test symbol, and
lcov has no `outcome` at all. The two ways to force it are both wrong: synthesizing pass/fail from a
threshold leaks verdict policy into the adapter, and inventing a neutral outcome asserts a result
the tool never produced.

Coverage belongs with the finding-shaped tools in `agent-ix/quoin#91b`. Recorded here so the next
reader finds the reasoning rather than the gap.

`cargo-mutants` sits on the same boundary and is included anyway, with its meaning stated: its
entries name **production** functions (`src/path.rs::function`), so they record a measured fact and
bind to no obligation until a requirement→production-code relation exists
(`agent-ix/quire-rs#171`). That is the honest state, not a convenient one.

### A pytest case maps to the Python extractor's symbol

The Python extractor names a test `Class.method` under a class and the bare function
name at module level; the module is never a prefix and only a class is a scope
(FR-120). The pytest mapping applies when the run is recorded with the suite's
declared `--tool`, matched the way adapter selection matches any tool claim
(FR-033-AC-1): the lower-cased tool contains `pytest`, so a versioned `pytest 8.3.2`
selects it. The JUnit parser receives the declared tool for this mapping. The mapping is never chosen by inspecting the report's attributes: pytest's default JUnit
output carries no `file` attribute. Any other tool uses the `::` rule above. For a
pytest case the symbol is built in this order:

1. When `classname` equals `name`, the name is taken as written, with no further
   rewriting. This is the form a FR-120 emitter writes.
2. Otherwise `name` loses a trailing `[...]`, and `classname` is split on `.`. The
   first segment beginning with `Test` and every segment after it are the enclosing
   classes, joined to the name with `.`. The segments before it are the module path
   and never appear in the symbol.
3. When no segment begins with `Test`, or `classname` is empty, the symbol is the
   name alone: a module-level function.

A project that sets non-default `python_classes`, and any class collected through
`unittest.TestCase` whatever its name, are out of scope: such classes are not
recognised, and their cases are recorded as module-level functions.

A pytest case obtains trace ids from the `trace` property that the FR-120 emitter
writes from the `trace` marker on the test. A case without the property has none.
The store therefore detects an unbound run from the entries alone: a case whose
ids no obligation states is unmatched, and a run in which no case has an id binds
nothing and is an error (FR-030-AC-32, FR-030-AC-33).

### A nextest report names tests, not criteria (interim)

**Interim.** [ADR-100](../adrs/ADR-100-execution-evidence-is-emitted-by-the-run.md) supersedes this annotator: ids are emitted by the run ([FR-120](./FR-120-trace-marks-in-test-result-output.md)), not derived afterward. It is removed when the PLAT-333 Rust emitter lands.

cargo-nextest writes a JUnit report that identifies each test by binary and name and says nothing
about the criteria the test exercises. The tags that say so live in source (`#[trace("FR-002-AC-1", ...)]`),
and only the traceability engine knows their grammar. `quoin-suite-trace` therefore copies the
engine's per-test relation into the report as a `trace` property, and the JUnit adapter reads that
property like any other producer's declared trace ids. A case with no tags is left unchanged, and the
ids come from the engine's relation rather than a second reading of the source. Parsing the tags a second time inside the
adapter would give the tag grammar two owners that can drift apart; an unannotated report would
record every run as bound to nothing.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-033-AC-1 | An adapter is selected by `--adapter`, else by the suite's declared `--tool`, else the normalized `entries` shape. | Test |
| FR-033-AC-2 | An unknown `--adapter` is an error naming the available adapters, never a silent fall back. | Test |
| FR-033-AC-3 | Adapters are registered as data, each carrying a name, a summary and a parse function. | Test |
| FR-033-AC-4 | JUnit `classname` + `name` map to the qualified name the symbol extractor emits, without doubling a classname that already ends with the member. | Test |
| FR-033-AC-5 | Every JUnit outcome class is read — `error` outranks `failure` outranks `skipped` — along with declared trace ids. | Test |
| FR-033-AC-6 | The JUnit adapter names **no** evidence kind, because the format does not carry one. | Test |
| FR-033-AC-7 | Input carrying no `<testcase>` is an error, not an empty run; a case with neither `classname` nor `name` is skipped. | Test |
| FR-033-AC-8 | The cargo-mutants adapter produces a per-function `score`, with unviable mutants in neither side of the ratio. | Test |
| FR-033-AC-9 | Outcome reflects the **tool's own** classification, never a threshold. | Test |
| FR-033-AC-10 | Malformed, empty and unattributable mutation reports are rejected; a timed-out mutant counts as survived. | Test |
| FR-033-AC-11 | `quoin evidence record --adapter junit --results <file>` records a JUnit file end to end, preserving symbols and trace ids. | Test |
| FR-033-AC-12 | `quoin evidence record --adapter cargo-mutants` records a mutation report end to end, preserving `score`. | Test |
| FR-033-AC-13 | `quoin evidence record` selects the adapter from `--tool` when none is named. | Test |
| FR-033-AC-14 | `quoin evidence record` still accepts the normalized shape with no adapter at all. | Test |
| FR-033-AC-15 | Interim, removed when the PLAT-333 Rust emitter lands (ADR-100): a cargo-nextest JUnit report annotated by `quoin-suite-trace` carries, on each tagged test case, a `trace` property naming that test's acceptance criteria, so `quoin evidence record --adapter junit` binds the run to them. | Test |
| FR-033-AC-16 | A case recorded with a `--tool` containing `pytest` (case-insensitive, for example `pytest 8.3.2`) is stored under the symbol the Python extractor emits. `classname="tests.test_bolus" name="test_basic"` is `test_basic`; `classname="tests.test_bolus.TestDose" name="test_a[1]"` is `TestDose.test_a`; `classname="tests.TestA.Inner" name="test_b"` is `TestA.Inner.test_b`; `classname="test_basic" name="test_basic"` is `test_basic`, not `test_basic.test_basic`; an empty `classname` gives the name alone. The same case recorded under another tool is stored by the `::` rule. | Test |
| FR-033-AC-17 | A pytest case that carries no `trace` property is recorded with its outcome and no `traceIds`. The run it belongs to is an error under FR-030-AC-33 when no case carries one, and a case whose id no obligation states is listed as unmatched under FR-030-AC-32. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-033-CON-1 | An adapter SHALL NOT read a file, spawn a process, or reach the network. It receives text and returns entries. | Design | Inspection |
| FR-033-CON-2 | An adapter SHALL NOT judge. No threshold, no policy, no verdict — only what the tool reported. | Design | Inspection |
| FR-033-CON-3 | The shipped adapters SHALL NOT name an evidence kind, so no fourth copy of that vocabulary is minted. | Design | Test |
| FR-033-CON-4 | The normalized `entries` shape SHALL remain available, so the registry is never a gate on recording evidence. | Design | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (the store these entries are written to)
- **Downstream**: `agent-ix/quoin#91b` (finding-shaped tools, and coverage), `agent-ix/quoin#91c` (inventories)
