---
id: SR-001
type: SuiteRegistry
title: "quoin suite registry"
---

# SR-001: quoin suite registry

quoin ships an evidence store (FR-030) and, until `agent-ix/quoin#206`, did not
use it on itself — while quire-rs dogfoods aggressively (`spec_dogfood.rs`,
`trace_dogfood.rs`, `make validate`). A store nobody runs against its own
repository is a store whose failure modes nobody meets.

This registry is what makes a run recordable: `SUITE-N` is the id every join
keys on — run directories, bindings, freshness — and it is structured and
doc-scoped precisely so it can be renamed by nobody (spec-artifacts-process
FR-006).

## Suites

| ID | Name | Command | Tool | Evidence Kind |
|----|------|---------|------|---------------|
| SUITE-001 | Unit and integration suite | `make evidence` | cargo-nextest | Unit |
| SUITE-002 | Spec validation gate | `make validate` | quire-cli | Static |
| SUITE-003 | Agent evals | `make evals` | cli-agent-evals | Eval |

## What the store discharges

A JUnit report names tests, and quoin's trace ids live in source tags
(`/// Trace: FR-002-AC-1`, `#[trace("FR-002-AC-1")]`), so a plain report binds no
obligation. `make evidence` closes that gap in three steps:

1. `cargo nextest run --profile evidence` runs the Rust workspace and writes
   JUnit XML (profile in `rust/.config/nextest.toml`). It needs
   `cargo install cargo-nextest --locked`. Doc tests are not run by nextest.
2. `quoin-suite-trace` asks the quire engine, which defines the tag grammar, for
   every test symbol and the ids it verifies, joins each to the name nextest
   reported (binary id and module path, from `cargo metadata`), and inserts
   `<property name="trace" value="ID,ID"/>` into that test case. It parses no
   source comments itself and refuses a report in which no case matched.
3. `quoin evidence record --suite SUITE-001 --adapter junit --kind Unit` records
   the run and binds every obligation named by a *passing* case. A failing or
   skipped case is recorded in the run and binds nothing.

The target returns the tests' own exit status after recording, so a red suite is
recorded as red. Tagged tests absent from a run (filtered or ignored) are
counted in the tool's summary line and bind nothing.

No run record is kept in this repository; a run is transcribed by the consumer's
CI when it executes a suite.

## Open follow-ups

- **The generated evidence store is not committed, on purpose.** A run record and
  the bindings derived from it describe one commit's test run, so they go stale
  with the next commit and would add churn to every change without adding
  information a reader cannot regenerate with `make evidence`. The committed
  `bindings.json` is the empty skeleton. Freshness is judged by the consumer that
  ran the suite, not by a file in this repository.
- **A bound-and-accepted ratchet check is not implemented.** Nothing yet fails
  when an acceptance criterion is bound by a passing run but has no accepted
  verdict, so a criterion can be bound indefinitely without anyone accepting it.
  Binding records that a test exercised the criterion; it does not record that the
  criterion is satisfied.
