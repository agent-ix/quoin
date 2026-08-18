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

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-033-AC-1 | An adapter is selected by `--adapter`, else by the suite's declared `--tool`, else the normalized `entries` shape. | Test (TC-151) |
| FR-033-AC-2 | An unknown `--adapter` is an error naming the available adapters, never a silent fall back. | Test (TC-152) |
| FR-033-AC-3 | Adapters are registered as data, each carrying a name, a summary and a parse function. | Test (TC-153) |
| FR-033-AC-4 | JUnit `classname` + `name` map to the qualified name the symbol extractor emits, without doubling a classname that already ends with the member. | Test (TC-154, TC-160, TC-162) |
| FR-033-AC-5 | Every JUnit outcome class is read — `error` outranks `failure` outranks `skipped` — along with declared trace ids. | Test (TC-155, TC-159, TC-161) |
| FR-033-AC-6 | The JUnit adapter names **no** evidence kind, because the format does not carry one. | Test (TC-156) |
| FR-033-AC-7 | Input carrying no `<testcase>` is an error, not an empty run; a case with neither `classname` nor `name` is skipped. | Test (TC-157, TC-164) |
| FR-033-AC-8 | The cargo-mutants adapter produces a per-function `score`, with unviable mutants in neither side of the ratio. | Test (TC-158) |
| FR-033-AC-9 | Outcome reflects the **tool's own** classification, never a threshold. | Test (TC-159) |
| FR-033-AC-10 | Malformed, empty and unattributable mutation reports are rejected; a timed-out mutant counts as survived. | Test (TC-160, TC-163) |
| FR-033-AC-11 | `quoin evidence record --adapter junit --results <file>` records a JUnit file end to end, preserving symbols and trace ids. | Test (TC-161) |
| FR-033-AC-12 | `quoin evidence record --adapter cargo-mutants` records a mutation report end to end, preserving `score`. | Test (TC-162) |
| FR-033-AC-13 | `quoin evidence record` selects the adapter from `--tool` when none is named. | Test (TC-163) |
| FR-033-AC-14 | `quoin evidence record` still accepts the normalized shape with no adapter at all. | Test (TC-164) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-033-CON-1 | An adapter SHALL NOT read a file, spawn a process, or reach the network. It receives text and returns entries. | Design | Inspection |
| FR-033-CON-2 | An adapter SHALL NOT judge. No threshold, no policy, no verdict — only what the tool reported. | Design | Inspection |
| FR-033-CON-3 | The shipped adapters SHALL NOT name an evidence kind, so no fourth copy of that vocabulary is minted. | Design | Test (TC-156) |
| FR-033-CON-4 | The normalized `entries` shape SHALL remain available, so the registry is never a gate on recording evidence. | Design | Test (TC-164) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (the store these entries are written to)
- **Downstream**: `agent-ix/quoin#91b` (finding-shaped tools, and coverage), `agent-ix/quoin#91c` (inventories)
