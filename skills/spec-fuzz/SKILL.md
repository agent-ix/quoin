---
name: spec-fuzz
description: Turn Fuzz-kind obligations into runnable fuzz targets. Consumes `quoin advise
  --json`, selects the obligations whose verification method carries `evidence_kind: Fuzz`,
  grounds each one's entry-point symbol in the repository's source, and emits targets in the
  repo's own harness (cargo-fuzz / atheris / fast-check) keyed on the obligation id. Writes
  nothing where the tooling is absent or the entry point cannot be grounded — a finding
  instead. Emits a validated SpecReview recording every obligation it could not serve.
---

# Spec Fuzz

Use this skill to **generate fuzz targets from robustness requirements you already wrote**.

*"Never panic on arbitrary input"* and *"reject malformed frontmatter"* are property-shaped
over an input surface. `spec-correctness` generates example and property tests, not fuzz
targets, so a consumer that parses anything gets nothing from its spec toward a harness —
the method can be advised from the catalog and materializing it is entirely manual.

Specified by [FR-038](../../spec/functional/FR-038-generate-fuzz-harnesses.md).

## The rule that is not negotiable

**Writing a target discharges nothing.**

This is where fuzz differs from every other generator in this family. A property test that
exists and passes is evidence. A fuzz target that exists has proved nothing — fuzzing is a
search, and an unrun target is a search never started.

The obligation is discharged by a **recorded run** (FR-030), in its own suite with
`Evidence Kind: Fuzz`. A generated target that has never run is precisely the
`vacuous-evidence` shape FR-034 exists to name. Report generated targets as **undischarged**,
always, and never imply otherwise in the handoff.

Three more:

- **Never install anything.** No `cargo install`, no dependency added, no fuzz workspace
  scaffolded. Absent tooling is a finding. (CON-2.)
- **Never decide which methods are fuzz methods.** That is the catalog's, read at run time.
  (CON-1.)
- **Never write a framework name into `spec/**`.** Inherited from FR-028-CON-2.

## When to use

- After `quoin advise` recommends `fuzzing` for an obligation and nothing implements it.
- On a repository that parses untrusted or structured input and has no fuzz suite.
- To extend an existing fuzz suite to surfaces the spec names and the suite misses.

Not for: authoring the requirement (`specify`), choosing the method (`quoin advise`), or
running the fuzzer. Running is the consumer's CI — quoin transcribes, it does not execute.

## When fuzzing stops being the answer

A fuzz campaign has a shape: coverage climbs, then flattens. **The plateau is
the documented signal to escalate**, not to fuzz harder. Random input cannot
satisfy a checksum, a CRC or a magic number — `if x * 3 == 51` is roughly a
1-in-4-billion coin flip for a fuzzer and one line of algebra for a solver.

The industrial pattern is **hybrid fuzzing**: fuzz until the curve flattens,
hand the stuck branches to a solver, feed the solved inputs back to the fuzzer
as fresh seeds. Driller (2016) did it with AFL + angr; QSYM, SymCC and Fuzzolic
followed.

The concrete hop in Rust:

```
cargo-fuzz  ──plateau──▶  Kani        bounded model checking over MIR (AWS)
                          haybale     symbolic execution of LLVM IR, in Rust
```

`quoin advise` will point there on its own once the evidence says so — the
catalog keys `concolic-execution` on `fault-detection-unmeasured` (exercised,
and nothing measures whether the tests discriminate) and
`fault-detection-failed` (measured, and a seeded fault survived). Record the
fuzz and mutation runs and the recommendation follows.

**Do not escalate before that.** Concolic execution path-explodes and is slow;
it earns its cost only where the cheap search has demonstrably stalled. The
catalog cannot yet express that ordering (`agent-ix/quire-rs#190`), so it is
written here.

## Inputs

- The target repository.
- `quoin advise --repo <repo> --json` — obligations with their authored method and
  recommendations.
- The active `verification_catalog` — which methods are Fuzz-kind, and the `tooling` each
  names. **Read it; do not carry a list here.**

## Steps

0. **[Scope and harness](references/step-0-scope-and-harness.md)** — detect the repository's
   language and fuzz tooling from its manifest. Absent tooling stops the run here, with
   findings.
1. **[Select the obligations](references/step-1-select-obligations.md)** — those whose
   method has `evidence_kind: Fuzz`, from the catalog rather than by name.
2. **[Ground the entry point](references/step-2-ground-entry-point.md)** — find the symbol
   the target will call, in the source. Ungrounded means a finding, not a guess.
3. **[Emit the targets](references/step-3-emit-targets.md)** — harness-native, two trace
   carriers, one provenance line.
4. **[Report](references/step-4-report.md)** — a validated `SpecReview` naming every
   obligation not served and why, and the handoff that says the targets are undischarged.

## What "could not be served" looks like

Three distinct outcomes, and they are not interchangeable:

| Outcome | Meaning | What to write |
|---|---|---|
| **no tooling** | the repo has no fuzz harness | finding per obligation; **no files** |
| **ungrounded** | no entry-point symbol found in the source | finding naming the obligation; **no file for it** |
| **already covered** | an existing target already claims the id | nothing; record as covered |

Never collapse these into "skipped". The first is a decision for the repository owner, the
second is a gap in the spec or the code, and the third is success.

## Why there is no `quoin fuzz` command

The selection is mechanical and the emission is not. Choosing the entry point for
*"the parser SHALL NOT panic on arbitrary input"* means reading the parser, and a command
that guessed would produce targets that do not compile.

`quoin advise` supplies the mechanical half as JSON. This skill is the judgement half, and
it records which is which in the provenance line so a reviewer can tell them apart.
