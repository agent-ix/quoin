# quoin-semantic

The semantic-module contract (quoin FR-070, FR-073, FR-074, FR-075) in Rust.
Port of `src/semantic/` — EPIC [#373](https://github.com/agent-ix/quoin/issues/373)
Stage 3, issue [#378](https://github.com/agent-ix/quoin/issues/378).

## What it owns

| Module             | Replaces              | Owns                                                                      |
| ------------------ | --------------------- | ------------------------------------------------------------------------- |
| `contract`         | `contract.ts`         | The vendored schema bundle's provenance and digests                       |
| `manifest`         | `manifest.ts`         | Reading and refusing a manifest's `semantic` block                        |
| `data_schema`      | `data-schema.ts`      | Resolving `{ schema, digest }` references and their `$ref` closure        |
| `package_manifest` | `package-manifest.ts` | The derived filament-core-data manifest, registry pins, import resolution |
| `sweep`            | `sweep.ts`            | The legacy Properties-form classifier and corpus sweep                    |
| `schema`           | ajv, inline           | The ajv-shaped adapter over the Rust `jsonschema` crate                   |

The TypeScript is **not deleted**: cutover and deletion are separate tickets per
FR-101. Both implementations read the same vendored schema tree at
`src/semantic/schemas/`, which is why this crate does not copy it.

## Read this first

**[`DIVERGENCE.md`](./DIVERGENCE.md)** — where ajv and the Rust `jsonschema`
crate disagree, what each disagreement costs a user, and why the pin is
`=0.56.0` when the rest of the ecosystem is on 0.17/0.18. It is a deliverable of
#378, not an appendix, and `src/schema.rs` is unreviewable without it.

## Parity

The TypeScript was the oracle **exactly once**.
`scripts/capture-semantic-goldens.mjs` ran it and wrote every verdict and every
diagnostic to `tests/goldens/`, each file naming the quoin revision and the ajv
version it was captured at. The tests read those files; nothing in the test lane
invokes Node.

- **Verdict parity is exact and contractual** over 171 golden documents.
- **Diagnostic parity is normalized** on `(instance location, keyword)` as a
  multiset.
- **Error text and error order are not contractual.** See `DIVERGENCE.md` §5.

## Gates

```bash
cargo +1.98.1 fmt --check
cargo +1.98.1 clippy --all-targets --all-features --target-dir <dir> -- -D warnings
cargo +1.98.1 test --target-dir <dir>
```

Always pass `--target-dir` explicitly; a shared default target directory across
worktrees produces results that belong to another branch.
