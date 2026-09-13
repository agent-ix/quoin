<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# quoin-validators

Deterministic repository QA-gate validators for quoin. Stage 2 of the Rust
burn-down ([quoin#377], parent [quoin#373]) — the port of `src/validators/`.

One capability today: `inspect_empty_gates`, which finds a declared, wired shell
gate that counts forbidden matches but never asserts the count ([quoin#224]).

**No TypeScript was deleted.** `src/validators/` and `tests/gate-validator.test.ts`
are retained and still passing; cutover and deletion are separate tickets per
FR-101/FR-018.

## Layout

| File                        | Owns                                                                     |
| --------------------------- | ------------------------------------------------------------------------ |
| `src/lib.rs`                | the public surface and the boundary contract                             |
| `src/error.rs`              | `ErrorCode` + `ValidatorError` — three variants, three stable codes      |
| `src/ids.rs`                | `ObligationId`, `RepoPath`, `LineNumber`                                 |
| `src/finding.rs`            | `EmptyGateFinding`, `GateReport`, `Verdict` — the emitted payload        |
| `src/gates.rs`              | the algorithm: claim, wiring, unasserted count                           |
| `src/repo.rs`               | the walk, and the `.sh` / wiring classification                          |
| `tests/golden/`             | the corpus, the expectations, and their provenance                       |
| `tests/golden_parity.rs`    | verdict parity against that corpus                                       |
| `tools/generate-oracle.mts` | the capture script that produced the expectations, and its vitest config |

## Boundary

The crate takes a repository root and returns a `GateReport`. It does not print,
does not exit, and does not know what a CLI flag is:

```rust
let report = GateReport::new(inspect_empty_gates(repo)?);
let lines = report.human_lines();          // what `quoin validate` prints
let code = report.verdict(strict).exit_code();
```

That split is what lets `quoin-core <domain>.<op>` wrap this without the
command's opinions leaking into the library. Serialising a `GateReport` produces
the exact bytes of `quoin validate --json`.

Every refusal is a `ValidatorError` with a stable code — `QV-E001` for an
unusable root, `QV-E002` for an unlistable directory, `QV-E003` for an unreadable
file. **Nothing degrades to an empty result**: an empty report means the
validator walked the whole tree and found nothing, which is the property the
TypeScript could not state because it let raw `fs` exceptions escape.

## The oracle

`tests/golden/expected.json` was captured **once** from the retained TypeScript at
quoin `4d27dcf` and is the only oracle these tests consult. No test here executes
Node. See [`tests/golden/PROVENANCE.md`](tests/golden/PROVENANCE.md) for the
revision, the source hashes, the corpus census, the two recorded
TypeScript↔Rust divergences, and how to re-derive the bytes.

`tools/generate-oracle.mts` is that capture. It is a **script, not a test**: it
is off the `*.test.ts` suffix, `vite.config.ts` excludes `rust/**`, and it
writes nothing unless `QUOIN_ORACLE_WRITE=1` is set. Run without that variable
it re-derives the verdicts and asserts they still match the committed bytes.

## Gates

This crate is a member of the `rust/` workspace (quoin#375): it inherits the
version fields, the lint policy and every shared dependency from
`rust/Cargo.toml`, so the gate it is measured by cannot drift from the
workspace's. `make rust-gate` from the repository root runs the whole lane.

Invoke cargo **from `rust/`** — rustup selects a toolchain from the working
directory, so a `--manifest-path` from the repository root ignores
`rust/rust-toolchain.toml`. Name the target directory explicitly; never rely on
an inherited `CARGO_TARGET_DIR`, because a shared target directory makes a clean
result a claim about whatever was built there last.

[quoin#224]: https://github.com/agent-ix/quoin/issues/224
[quoin#373]: https://github.com/agent-ix/quoin/issues/373
[quoin#377]: https://github.com/agent-ix/quoin/issues/377
