---
id: ADR-001
title: "Execution evidence is emitted by the run, not derived by matching"
type: ADR
relationships:
  - target: "ix://agent-ix/quoin/FR-033"
    type: "relates_to"
  - target: "ix://agent-ix/quoin/FR-120"
    type: "relates_to"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "relates_to"
---
# ADR-001: Execution evidence is emitted by the run, not derived by matching

## Status

Proposed. Supersedes nothing. Tracked by PLAT-332.

## Context

`quoin evidence record` transcribes a test run into the evidence store, and a
trace id in a result entry is what binds that run to a requirement criterion or
obligation. Today no producer names the id: the source markers (`#[trace(...)]`,
`@pytest.mark.trace(...)`, `trace(...)`) annotate source and the compiled or
interpreted run carries nothing out. No repository has a populated evidence
store, so a green Test Matrix row records that a test is tagged, not that it ran.

Two ways exist to close the gap.

**Option A, derive.** The runner reports `name + outcome`, and a joiner matches
each name against the static `symbol -> ids` relation that `quire coverage`
already produces. This needs no per-runtime work, and it is the cheaper option.

**Option B, emit.** The run itself writes the ids into its result output, as a
mark on each test case, and `quoin evidence record` reads them as given.

## Decision

Execution evidence SHALL be emitted by the run. A result file names the trace
ids of each test case itself; nothing downstream infers them from names.

The mark contract is [FR-120](../functional/FR-120-trace-marks-in-test-result-output.md).
The constraints on how an emitter produces it are
[FR-121](../functional/FR-121-emitter-mechanism-constraints.md),
[NFR-028](../non-functional/NFR-028-inert-and-low-cost-emission.md) and
[NFR-029](../non-functional/NFR-029-self-contained-result-evidence.md).

## Why emit wins

- **The adapter contract forbids the matcher's input.** [FR-033](../functional/FR-033-evidence-format-adapters.md)
  requires an adapter to be pure over text, with no filesystem beyond its input,
  and to report what the tool said and nothing else. A matcher needs a second
  input, the source tree at the right commit, and it asserts an id the runner
  never said. Quoin transcribes; the consumer's CI executes.
- **A matcher sits inside the evidence chain.** A rename, a parametrized case, or
  two files declaring the same qualified name can bind a result to the wrong
  requirement, with nothing downstream positioned to notice. Any component that
  can silently change what a result is bound to is a component whose correctness
  the evidence depends on, and so a candidate for qualification as a tool in a
  safety or certification regime. An emitted id removes the component from the
  chain. This ADR does not cite clause numbers from those standards; the
  references have not been checked against the standard text, and the argument
  does not rest on them.
- **Self-containment.** An emitted result file is interpretable on its own. A
  matched one means something only beside the source tree at one commit, so an
  archive of it must retain both and prove they correspond
  ([NFR-029](../non-functional/NFR-029-self-contained-result-evidence.md)).

## Consequences

- An emitter per runtime (pytest, vitest/jest, Rust), each held to one contract.
- Rust adopts `cargo-nextest` wherever JUnit output is needed; stock `cargo test`
  has no stable machine-readable output.
- The emitted symbol must equal what the static extractor reports, so the
  symbol-naming rules in FR-120 are a cost every emitter carries.
- Cases the old matcher would have absorbed (parametrization, nested suites,
  same name in two files) must be settled once in the contract, not per emitter.
- The existing nextest annotator in FR-033-AC-15 derives a `trace` property from
  the traceability engine after the run. It is a matcher in this ADR's sense and
  is not the emission contract. Its disposition once the Rust emitter lands is
  recorded in PLAT-333, not here.

## Alternatives Considered

- **Derive by matching (Option A).** Cheaper and needs no per-runtime work.
  Rejected for the reasons above: it needs an input FR-033 forbids, and it puts a
  name-matching step inside the evidence chain.
- **Per-runtime sidecar file merged after the run.** Reintroduces the inference
  step the decision removes, and adds file I/O and locking under parallel
  harnesses. Rejected.
