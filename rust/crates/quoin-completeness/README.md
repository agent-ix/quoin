# quoin-completeness

Declared-vocabulary completeness and its verdict policy (quoin FR-037, ADR-0011)
in Rust. Port of `src/completeness/` — EPIC
[#373](https://github.com/agent-ix/quoin/issues/373) Stage 3, issue
[#378](https://github.com/agent-ix/quoin/issues/378).

## The split ADR-0011 names

**quire-rs computes coverage over the spec corpus. quoin decides what a gap is
worth and whether an excuse was earned.** This crate is the second half.

The engine accepts a bare `quality_attributes_not_applicable: [safety]` and stops
reporting the value. Measured on this repository: 7 findings before, 5 after
adding that one line, with no reason written anywhere. So this crate requires a
written reason — a table row naming the value, carrying at least three words that
are not `n/a`, `-` or `TBD` — and grades an unjustified exclusion **higher** than
an admitted gap.

`UNCHECKED` is a verdict. A bundle whose module set declares no vocabulary has
not been assessed, and `PASS` over it is the green-matrix-over-dead-links result
the whole area exists to stop.

## What it owns

| Module         | Replaces                                                                            |
| -------------- | ----------------------------------------------------------------------------------- |
| `declarations` | `declarations.ts` — reads `traceability.vocabulary_coverage` and the enum behind it |
| `bundle`       | `bundle.ts` — one frontmatter pass over the bundle                                  |
| `assess`       | `assess.ts` — the policy                                                            |
| `run`          | `run.ts` — one bundle, every declaration, one walk                                  |

`defaultModuleRoots()` is deliberately **not** ported: resolving where modules
live belongs to the catalog (Stage 7), and a second answer here is exactly the
duplication FR-037 exists to avoid. The caller supplies the roots.

## Parity

Captured once from the TypeScript by `scripts/capture-semantic-goldens.mjs`;
goldens live in `tests/goldens/` and name the revision they came from. Verdicts,
finding kinds, severities and rollup counts are compared exactly. Message text is
not.

This crate compiles no JSON Schema, so the ajv/`jsonschema` divergence does not
reach it. The one that does is YAML — see
[`../quoin-semantic/DIVERGENCE.md`](../quoin-semantic/DIVERGENCE.md) §6.

## Gates

```bash
cargo +1.98.1 fmt --check
cargo +1.98.1 clippy --all-targets --all-features --target-dir <dir> -- -D warnings
cargo +1.98.1 test --target-dir <dir>
```
