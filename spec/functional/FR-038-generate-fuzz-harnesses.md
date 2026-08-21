---
id: FR-038
title: "Generate fuzz harnesses from Fuzz-kind obligations"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-028"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-031"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# FR-038: Generate fuzz harnesses from Fuzz-kind obligations

## Description

`quoin` SHALL supply a `spec-fuzz` skill that consumes the obligations whose verification
method carries `evidence_kind: Fuzz` and emits fuzz targets in the target repository's own
harness.

Robustness requirements — *"never panic on arbitrary input"*, *"reject malformed frontmatter"* —
are property-shaped over an input surface, and `spec-correctness` (FR-028) generates example and
property tests, not fuzz targets. A consumer that parses anything gets nothing from its spec toward a
fuzz harness: the method can be **advised** from the catalog and materializing it is entirely manual.

### The input surface is already declarable

`spec-artifacts-process` declares two Fuzz-kind methods — `fuzzing` (keyed on `untrusted-input`,
`parser`, `deserializer`) and `grammar-based-fuzzing` (`structured-input`, `grammar-declared`) — and
FR-031's advisor already recommends the first from an obligation's statement.

**So this skill introduces no new declaration.** A "fuzzable surface" manifest key would be a third
source of truth about the same fact, disagreeing with the catalog the first time either changed. The
selector is: *the obligation's method has `evidence_kind: Fuzz`*, whether the advisor recommended it
or an author wrote it into the `Verification` cell.

Two consequences worth stating rather than discovering. `grammar-based-fuzzing` is keyed on two
characteristics the advisor mints **neither** of, so today it is reachable only by being authored
(`agent-ix/quoin#128`). And `deserializer` is likewise unminted, so `fuzzing` is advised from
`untrusted-input` and `parser` alone.

### A harness names a symbol, or it is not written

A fuzz target must call something. The entry point is **grounded in the repository** — a symbol that
exists in the source — and recorded in the provenance line. Where no entry point can be grounded, the
skill emits a finding naming the obligation and writes nothing.

This is the FR-028 discipline applied to a harsher case: a property test that grounds nothing is a
weak test, and a fuzz target that calls nothing is a build error in somebody's CI.

### Absent tooling is a finding, never an install

Where the repository's fuzz tooling is absent — no `cargo-fuzz`, no `atheris`, no `fast-check` — the
skill SHALL emit one finding per obligation and write no files. It SHALL NOT install, add a
dependency, or scaffold a fuzz workspace.

Generated artifacts are consumer-owned (ADR-0011, L2). Adding a dev-dependency and a nightly
toolchain requirement to somebody's repository because their spec mentioned a parser is not a
generated artifact; it is a decision, and it is theirs.

### Writing a target discharges nothing

This is where fuzz differs from every other generator in the family. A property test that exists and
passes is evidence. **A fuzz target that exists has proved nothing** — fuzzing is a search, and an
unrun target is a search never started.

The obligation is discharged by a **recorded run** (FR-030), in its own suite with
`Evidence Kind: Fuzz`, so staleness is visible per suite. A generated target that has never run is
exactly the `vacuous-evidence` shape FR-034 exists to name.

### What it inherits from FR-028

The tag contract is unchanged, because `gap-analysis` reconciles by the same grep. Every emitted
target carries **two independent carriers** — a `Trace:` line in the doc comment and the id in the
target's own name — plus a provenance line the reconciliation ignores and re-runs read.

The placement rule is inherited verbatim and matters more here: a carrier binds to the symbol whose
source span encloses it. For a `libfuzzer` target that span is the `fuzz_target!` block, not the file
header.

## Inputs

- The obligations of `quoin advise --json`, carrying id, statement, authored method and
  recommendations (FR-031).
- The active `verification_catalog`, which names which methods are Fuzz-kind and what tooling each
  implies — module data, never a list in this skill.
- The target repository's source tree, for grounding the entry-point symbol.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-038-AC-1 | Obligations are selected by their method's `evidence_kind: Fuzz`, read from the catalog. | agent-behaviour-eval (TC-EV-054) |
| FR-038-AC-2 | An emitted target calls an entry point that exists in the repository's source. | agent-behaviour-eval (TC-EV-054) |
| FR-038-AC-3 | An obligation whose entry point cannot be grounded yields a finding and no file. | agent-behaviour-eval (TC-EV-055) |
| FR-038-AC-4 | Absent fuzz tooling yields one finding per obligation and no file, and nothing is installed. | agent-behaviour-eval (TC-EV-055) |
| FR-038-AC-5 | Every emitted target carries both trace carriers and a provenance line. | agent-behaviour-eval (TC-EV-054) |
| FR-038-AC-6 | A re-run with unchanged inputs rewrites nothing. | agent-behaviour-eval (TC-EV-056) |
| FR-038-AC-7 | The harness is chosen from the repository's manifest, never from the requirement's language. | agent-behaviour-eval (TC-EV-056) |
| FR-038-AC-8 | Generated targets are reported as undischarged until a run is recorded. | agent-behaviour-eval (TC-EV-057) |
| FR-038-AC-9 | The skill emits no verdict, grade or threshold, and never rewords a requirement. | agent-behaviour-eval (TC-EV-057) |
| FR-038-AC-10 | The eval assertions that prove refusal are falsifiable: a scenario glob the harness cannot express (brace expansion) is rejected when the scenario loads, rather than compiling to a pattern that matches nothing — under which an `absentFiles` check passes whether or not the forbidden file exists. | Test (TC-270) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-038-CON-1 | The skill SHALL NOT declare which methods are fuzz methods. That is the catalog's, read at run time. | Design | agent-behaviour-eval (TC-EV-054) |
| FR-038-CON-2 | The skill SHALL NOT install tooling, add a dependency, or scaffold a fuzz workspace. | Design | agent-behaviour-eval (TC-EV-055) |
| FR-038-CON-3 | The skill SHALL NOT treat an emitted target as evidence. Discharge requires a recorded run. | Design | agent-behaviour-eval (TC-EV-057) |
| FR-038-CON-4 | The skill SHALL NOT write a framework name into `spec/**` (inherited, FR-028-CON-2). | Design | agent-behaviour-eval (TC-EV-056) |

## Dependencies

- **Upstream**: [FR-031](./FR-031-catalog-driven-advisor.md) (supplies the obligations and their methods), [FR-028](./FR-028-generate-property-tests-from-criteria.md) (the generator pattern and the tag contract), [FR-030](./FR-030-evidence-store.md) (where a fuzz run is recorded)
- **Downstream**: `agent-ix/quoin#128` — `grammar-based-fuzzing` is unreachable by advice until its characteristics are mintable
