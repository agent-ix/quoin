---
id: FR-181
title: "quoin profile evaluate: evaluate every set of the effective profile against its context"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-180"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-029"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
---

# FR-181: quoin profile evaluate: evaluate every set of the effective profile against its context

## Description

`quoin profile evaluate` takes the effective profile ([FR-180](./FR-180-source-profile-selection.md)) and runs the
Quire engine's clause-set evaluation (`quire clauses evaluate`, `clause-binding-v1`) once per listed set, with the
effective context. For each set Quoin finds the one installed module that declares that exact key and passes it to the
engine; Quoin does not reimplement applicability.

The same context goes to every set. A dimension a set does not declare is ignored by that set. A dimension a clause
needs and the context lacks stays `unresolved`, exactly as the engine reports it, so a profile that omits a fact leaves
visible unresolved clauses rather than silently dropping them. When two sets use one dimension name, that name is one
fact in the context; a value one set does not admit is reported `unresolved` for that set by the engine, and Quoin does
not translate values between sets.

A listed set that no installed module declares is reported as `unavailable`, with its key. This is how a default
profile that lists a pack not yet installed degrades: the other sets are still evaluated and the gap is stated. An
unavailable set is not a failure and is not counted as binding, not-binding or unresolved.

The report is JSON with `schemaVersion: source-profile-evaluation-v1` and these fields, all required: `profile` (name),
`context` (the effective context), `sets` and `summary`. `sets` is a list in profile order of objects with `key`
(`authority`, `id`, `version`), `layer` (`profile`, `project` or `user`, per [FR-182](./FR-182-local-pack-joins-a-profile.md)),
`status` (`evaluated` or `unavailable`) and, when evaluated, `report` (the engine's `clause-binding-v1` document).
`summary` holds `setsEvaluated`, `setsUnavailable` and `clauses` (a count for each of `binding`, `not_binding`,
`unresolved`). The document has no other field. With `--out-dir` Quoin also writes each engine report unchanged to its
own file, so each can be given to `quoin discharge` ([FR-046](./FR-046-clause-discharge.md)) as a binding input.

## Inputs

- The effective profile.
- Installed module roots and the Quire engine.

## Outputs

- One `source-profile-evaluation-v1` document on stdout.
- With `--out-dir`, one `clause-binding-v1` file per evaluated set.

## Behavior

- Quoin SHALL validate each engine report against `clause-binding-v1` before including it ([FR-029](./FR-029-consume-the-quire-json-contract.md)).
- Quoin SHALL evaluate each set against only the module that declares its exact key.
- When a key is declared by more than one installed module, Quoin SHALL refuse the run, naming the modules.
- When no installed module declares a listed key, Quoin SHALL report that set as `unavailable` and continue with the other sets.
- Quoin SHALL include the summary counts and no score or percentage.
- When the engine fails to evaluate a set that a module declares, Quoin SHALL exit non-zero without reporting the set as unavailable.
- Quoin SHALL NOT modify any module, selection file or requirement; it writes to stdout and `--out-dir` only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-181-AC-1 | For a profile of two installed sets, the report has two entries in profile order, each holding a validated `clause-binding-v1` report whose `context` equals the effective context. | Test |
| FR-181-AC-2 | A context lacking a dimension a clause needs yields that clause as `unresolved` in the report; it is not dropped or promoted to binding. | Test |
| FR-181-AC-3 | A listed set no installed module declares appears as `unavailable` with its key, the other sets are still evaluated, and the exit status is 0. | Test |
| FR-181-AC-4 | A key declared by two installed modules exits non-zero and names both. | Test |
| FR-181-AC-5 | A declared set the engine fails to evaluate exits non-zero and is not shown as `unavailable`. | Test |
| FR-181-AC-6 | The summary states sets evaluated, sets unavailable and clause counts per outcome, and the document contains no score, percentage or pass verdict. | Test |
| FR-181-AC-7 | `--out-dir` writes one file per evaluated set, byte-equal to the engine report, and `quoin discharge` accepts each as a binding input. | Test |
| FR-181-AC-8 | A report that fails `clause-binding-v1` validation exits non-zero and is not included. | Test |
| FR-181-AC-9 | The output validates against the closed `source-profile-evaluation-v1` shape named above, and a document with an extra or missing top-level field fails it. | Test |
| FR-181-AC-10 | With a value in the context that one set admits and another does not, the first set reports the clause resolved and the second reports it `unresolved`; the value is not rewritten. | Test |
| FR-181-AC-11 | Hashes of every module root, every selection file and the `spec/` tree taken before and after a run are equal. | Test |

## Dependencies

- **Upstream**: [FR-180](./FR-180-source-profile-selection.md), [FR-029](./FR-029-consume-the-quire-json-contract.md); `agent-ix/quire-rs` clause-set evaluation
- **Downstream**: [FR-046](./FR-046-clause-discharge.md) consumes the per-set binding files
