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
| SUITE-001 | Unit and integration suite | `make test` | vitest | Unit |
| SUITE-002 | Spec validation gate | `make validate` | quire-cli | Static |
| SUITE-003 | Agent evals | `make evals` | cli-agent-evals | Eval |

## What the store discharges

The junit adapter reads test **names**, and quoin's trace ids live in source
comments (`// Trace: FR-002-AC-1`) where `quire coverage` reads them off symbols.
A junit report carries no symbol, so a SUITE-001 run binds no obligation.

Closing that gap needs one of two things, and neither is invented here:

- test names that carry their trace id, the way the Rust corpus writes
  `fn tc_744_…`; or
- an adapter that reads the symbol tags rather than the report, which is the
  same join `quire coverage` already performs.

No run record is kept in this repository; a run is transcribed by the consumer's
CI when it executes a suite.
