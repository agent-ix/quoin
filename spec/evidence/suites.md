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
| SUITE-004 | Real-tool integration checks | `make real-tool-test` | real-tool-test lane | Integration |
| SUITE-005 | Behavioral mutation checks | `make mutation-test` | cargo-mutants | Integration |

For the Quoin tag-attestation release-verification result, `SUITE-001` is the
declared suite. Its complete required release binding set is
`FR-186-AC-1`, `FR-186-AC-2`, `FR-186-AC-3`,
`FR-186-AC-4`, `FR-186-AC-5`, `FR-186-AC-6`, `FR-186-AC-7`, and
`FR-186-AC-8`; a healthy fresh run must have at least one passing tagged case
for each criterion. A failed or missing run is retained with an explicit
incomplete status in the release-evidence artifact. The maintained suite
declaration and binding registry are committed. `make evidence` generates
run-specific bindings and a run record for the tag; the workflow uploads those
generated facts with the maintained declarations and tag/run provenance as the
`release-evidence` artifact. It does not commit run-specific bindings or
tag-run records. `SUITE-002` and `SUITE-003` remain separate evidence suites
and are not claimed by the release Test Result.

`SUITE-004` runs the repository's applicable ignored tests against the real
tools that compose its verification path. The local target records every
attempted tool check, including a failed or unavailable tool, in a retained run
record. Each entry identifies the tool version and any result-affecting input
in its `config`; an OSV-Scanner entry includes the exact caller-supplied
`osv_mirror_date`. An unavailable tool has `outcome: error` and
`config.execution_state: not-executed`; it never has a passing outcome. This
local suite is independent of release CI.

`SUITE-005` runs behavioral mutations over the approval-decision, discharge,
evidence-binding, risk-completeness and per-class-floor kernels. The raw
cargo-mutants output is retained under the worktree's target directory; the
Quoin run record retains the survivor projection. Every surviving mutation
requires a human review disposition in the adjacent retained disposition file.
The lane records results and does not assign acceptance, risk, severity or
qualification.

## What the store discharges

A JUnit report names tests, and quoin's trace ids live in source tags
(`#[trace("FR-002-AC-1", "TC-2")]`), so a plain report binds no
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

Local runs of `SUITE-004` and `SUITE-005` are transcribed by their repository
targets and retained in the generated evidence store. Those run-specific files
are not committed: they describe one exact tool and input set and go stale with
the next commit. Consumer CI may also transcribe its own run when it executes a
suite.

## Open follow-ups

- **The generated evidence store is not committed, on purpose.** A run record and
  the bindings derived from it describe one commit's test run, so they go stale
  with the next commit and would add churn to every change without adding
  information a reader cannot regenerate with the suite targets. The committed
  `bindings.json` is the empty skeleton. Freshness is judged by the consumer that
  ran the suite, not by a file in this repository.
- **A bound-and-accepted ratchet check is not implemented.** Nothing yet fails
  when an acceptance criterion is bound by a passing run but has no accepted
  verdict, so a criterion can be bound indefinitely without anyone accepting it.
  Binding records that a test exercised the criterion; it does not record that the
  criterion is satisfied.
