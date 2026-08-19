---
id: FR-036
title: "Architecture conformance as a declared verification method"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-034"
    type: "extends"
  - target: "ix://agent-ix/quoin/NFR-007"
    type: "references"
---

# FR-036: Architecture conformance as a declared verification method

## Description

Specs and ADRs declare boundaries — *"the engine does not depend on the CLI"*, *"no network deps in
core"*, *"quoin transcribes, the consumer's CI executes"* — and prose rots silently. Nothing fails
when the boundary is crossed, so the document keeps asserting something that stopped being true.

Architecture conformance is the method that makes such a statement mechanical. The catalog already
carries the entry (`spec-artifacts-process`, `verification_catalog.architecture-conformance`,
`evidence_kind: Static`). This makes it **reachable, recordable, and applied to quoin itself**.

Three parts, each fixing a different break in the same chain:

1. the advisor can **recommend** the method, because both characteristics its rule is keyed on now exist;
2. an audit run's output is **evidence**, via an adapter in the FR-033 family;
3. quoin's own boundaries are **checked**, so the method has a worked example in this repository.

### The advisor could not recommend it

The catalog keys `architecture-conformance` on `[layering, module-boundary]`. `characteristicsOf`
minted `layering` and nothing minted `module-boundary`, so half the rule was inert.

The two are not synonyms and are not merged. **`layering` is directional** — who may depend on whom.
**`module-boundary` is about the surface** — what is exported, what is internal, what may cross. The
phrase `module boundary` moved out of `layering`'s pattern rather than appearing in both: one phrase
matching two characteristics is how two names stop meaning different things.

This is one instance of a larger gap. A census of the shipped catalog found **41 of its 60 declared
characteristics are minted by nothing**, leaving **7 methods** — `integration-testing` and
`mutation-testing` among them — unreachable outright. That is `agent-ix/quoin#128`, deliberately
**not** fixed here: several of the 41 are not lexical facts at all and need a fact source rather than
a regex, and widening `characteristicsOf` to lower a count is precisely the move this project
forbids.

### The criterion is in the failure line, which no other adapter can assume

FR-034 justified finding-shaped evidence by the join between a scanner's rule id and an obligation.
For SARIF that join is a mapping someone maintains. For an audit script it is **stated by the tool**:

```
check_no_schemars: FAIL — 'schemars' present in Cargo.lock (FR-003-AC-4).
```

The criterion the violation bears on is in the text. Nothing has to be kept in agreement with
anything, which is why this format is worth reading rather than replacing.

### A clean run is a result, not an absence

Seven checks reporting `OK` is the healthiest possible outcome, and it must not be indistinguishable
from a scan that never happened. `rulesEvaluated` therefore counts lines of **either** kind. Were only
failures counted, a fully-passing suite would report zero rules and read as vacuous under FR-034 —
exactly backwards.

Output carrying no recognised line is rejected outright. No audit ran, and recording it would
manufacture conformance evidence from a file that proves nothing.

### quoin's own boundaries are tests, not audit scripts

The method's prior art is `quire-rs/scripts/audits/` — bash, because a Rust repo has no import-graph
linter and its checks must survive a broken build. **Neither condition holds here**, and the
distinction matters more than the precedent.

Written as shell, the executor check reported five violations on its first run. Every one was
`LINE.exec(line)`: `RegExp.exec` read as `child_process.exec` by a regex that cannot tell them apart.
Bad rule, not bad corpus. The TypeScript compiler API can tell them apart, so it is used, and the
checks run in `make test` where every other assertion about this repository lives.

The adapter is unaffected by that choice — it reads `quire-rs`'s seven existing scripts, which remain
the right shape for that repository. Whether those seven should instead be `cargo deny` bans and
clippy lints is `agent-ix/quire-rs#178`.

### The two boundaries quoin declares

**Commands are leaves.** The library is what the plugin API, the skills and every consumer import;
oclif is a delivery surface on top. An import in the other direction makes the library unusable
without the CLI framework, and `tsc` accepts a cycle that only bites at runtime in whichever host
loads it first.

**quoin executes exactly `git`, `quire` and `ix-flow`.** This is ADR-0011 invariant 1 made mechanical.
A run record's claim is *"this ran in your CI"*, which holds only while quoin is not the thing running
it. NFR-007 names `quire` and `ix-flow`; `git` is the `rev-parse HEAD` the evidence store is keyed on.

The set is asserted **whole and sorted**, not as membership. A new binary fails by default rather than
passing until someone remembers to extend a list — the failure mode of every denylist, and the one
`quire-rs`'s `# extend as needed` comment demonstrates.

An executor whose binary is not a string literal fails the check rather than being skipped: it cannot
be read, and a check that silently ignores what it cannot read is asserting over a set it cannot see.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-036-AC-1 | A real all-passing audit run reads as rules that ran and found nothing, not as an absent scan. | Test (TC-197) |
| FR-036-AC-2 | A `FAIL` line becomes a finding carrying the acceptance criterion the script names. | Test (TC-198) |
| FR-036-AC-3 | `OK` lines count toward rules evaluated, so a clean run is not vacuous. | Test (TC-199) |
| FR-036-AC-4 | Output containing no recognised audit line is rejected. | Test (TC-200) |
| FR-036-AC-5 | The adapter is selected by `--adapter`, and by `--tool` for the tools emitting this shape. | Test (TC-201) |
| FR-036-AC-6 | An architectural statement is advised `architecture-conformance`, with both halves of the catalog's rule matching. | Test (TC-204, TC-205) |
| FR-036-AC-7 | No module outside `src/commands/` imports from `src/commands/`. | Test (TC-202) |
| FR-036-AC-8 | quoin executes exactly `git`, `quire` and `ix-flow`. | Test (TC-203) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-036-CON-1 | quoin SHALL NOT execute a conformance tool. The adapter reads captured output; the consumer's CI runs the check (ADR-0011 invariant 1). | Design | Test (TC-203) |
| FR-036-CON-2 | quoin SHALL NOT ship a conformance engine. The method rides per-language tooling and the audit-script pattern. | Design | Inspection |
| FR-036-CON-3 | `module-boundary` and `layering` SHALL NOT match the same phrase. Two characteristics sharing a trigger are one characteristic with two names. | Design | Test (TC-204) |
| FR-036-CON-4 | The author of a conformance check SHALL observe it fail against an injected violation before relying on it. A check never seen to fail is not a check. | Process | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (adapter contract and registry), [FR-034](./FR-034-finding-shaped-evidence.md) (finding-shaped records and vacuity), [NFR-007](../non-functional/NFR-007-external-tool-invocation.md) (the external-tool allowlist this enforces)
- **Downstream**: `agent-ix/quoin#128` (the 41 unmintable characteristics), `agent-ix/quire-rs#178` (migrating five of the seven audit scripts to `cargo deny` and clippy)
