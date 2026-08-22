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

## What the store discharges today, and what it does not

**Nothing, yet — and that is recorded rather than dressed up.** A run of
SUITE-001 transcribes 541 vitest results into `runs/SUITE-001/`, and binds
**zero** obligations:

```
{ "runPath": "spec/evidence/runs/SUITE-001/…json", "bound": [], "suspect": [], "unmatched": [] }
```

The reason is structural, not a misconfiguration. The junit adapter reads test
**names**, and quoin's trace ids live in source comments (`// Trace: FR-002-AC-1`)
where `quire coverage` reads them off symbols. A junit report carries no symbol,
so there is nothing for an obligation id to match.

So the audit reports **341 undischarged** obligations and **0 healthy**, and
`baseline.json` accepts all 341 as the ratchet floor. `make evidence-audit`
compares against that floor: the backlog cannot grow, and every obligation that
becomes discharged is a permanent gain.

Closing the binding gap needs one of two things, and neither is invented here:

- test names that carry their trace id, the way the Rust corpus writes
  `fn tc_744_…`; or
- an adapter that reads the symbol tags rather than the report, which is the
  same join `quire coverage` already performs.

Recording a store that discharges nothing is still worth doing: the run
directory, the suite ids and the ratchet floor all exist now, and the next
person meets a store with real data in it rather than an absent directory.
