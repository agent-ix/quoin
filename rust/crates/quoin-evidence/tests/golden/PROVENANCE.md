<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Adapter golden corpus — provenance

## What this is

`expected.json` is the output of the **retained TypeScript** adapters over
`cases.json`, captured once and checked in. `tests/golden_parity.rs` asserts the
Rust adapters reproduce it. No Node process runs in the Rust test lane
(FR-101-AC-5); that is the whole point of capturing rather than shelling out.

## Producer

| Field           | Value                                                                                            |
| --------------- | ------------------------------------------------------------------------------------------------ |
| Oracle          | `src/evidence/adapters/` (the retained TypeScript)                                               |
| Oracle revision | quoin `a2b5dfc`; the oracle directory last changed at `7072d65db7e236e819d99be42e094ba548e9496a` |
| Capture script  | `rust/crates/quoin-evidence/tools/generate-adapter-oracle.mts`                                   |
| Capture config  | `rust/crates/quoin-evidence/tools/vitest.oracle.config.mts`                                      |
| Capture date    | 2026-09-13                                                                                       |
| Runtime         | Node v22.15.0, pnpm 11.20.0, vitest 4.1.10                                                       |

## Oracle file digests (SHA-256)

Verify these before recapturing. A capture taken against different bytes is a
different oracle and must say so.

```
3ba1aeee5a123d532c3232e00239a11bdeaa59a0717f4483463caa118c3ea6f7  src/evidence/adapters/agent-eval.ts
579b780fc7318f8b7f3486e0222edaabe254fe4289a08dcbb28632f15493f7c0  src/evidence/adapters/audit-script.ts
15214dba0031f49d4987bc0eb3dd128452c0728a49224f165bc9e9ef3e338553  src/evidence/adapters/cargo-mutants.ts
e98ea06584e94d0aea457fc19a79f4499c1169775c8b61f2f0f3c3d0b5cdbf4f  src/evidence/adapters/contract-conformance.ts
e6fdff1a3a9c00abf36007d54f0f01889190ac28878f9cbc3905566d999cdca8  src/evidence/adapters/differential-report.ts
40d34480cd70313f21f9378f5a516a29b9308fce511238683d5ce8bfb46969b0  src/evidence/adapters/junit.ts
fbb9771a204a2d26f5f676a013b7997bb57abfc3dce57cabb84c150e22e37eaa  src/evidence/adapters/registry.ts
c9934357effbde07cd72cfd0989a87fee213bfbd4971d3bcc3c60271b1930940  src/evidence/adapters/sarif.ts
c9365fb5cd869be309f566967ecf999b254a4fc0a3e7c9aeeb1e39d0098b1d6b  src/evidence/adapters/sbom.ts
07e97e9f98042d62fee36ce1d202b1d776288e43daa1552fbbe06448c28401ae  src/evidence/adapters/types.ts
```

Captured artefacts:

```
3a99fc4463ad01e4eb3f372a4f19e03ec46739d2b19a2353068a19a133f9cf83  rust/crates/quoin-evidence/tests/golden/cases.json
908f38f31c360f6e5ffd4dfd800ffb17cedccd51ec57d01db6e22b789be12983  rust/crates/quoin-evidence/tests/golden/expected.json
```

## Reproduction

From the repository root. Verify the committed golden still reproduces (writes
nothing):

```bash
pnpm vitest run --config rust/crates/quoin-evidence/tools/vitest.oracle.config.mts
```

Regenerate it, deliberately:

```bash
QUOIN_ORACLE_WRITE=1 pnpm vitest run \
  --config rust/crates/quoin-evidence/tools/vitest.oracle.config.mts
```

Assert the Rust side against it, from `rust/`:

```bash
cargo test -p quoin-evidence --target-dir target --test golden_parity
```

After quoin#458 retires `src/evidence/`, restore the oracle at the revision
named above before recapturing. Capturing against the port would ask the port
whether the port is right.

**The capture tooling is gone as of quoin#458.** FR-101-AC-5 forbids a non-Rust
test oracle after a cutover, so `tools/` was deleted in the same commit that
deleted `src/evidence/`. Nothing in the tree can re-derive these bytes any
more; they are a frozen record of what the retained TypeScript answered, and
the Rust replays assert against them directly. To recapture, restore both the
tools and their subject from the revision named above — `git show <rev>:<path>`
— never from the port.

## Population

76 cases. **Both halves are non-empty**, which the capture script asserts and
`golden_parity.rs` re-asserts per adapter group:

| Half                       | Count |
| -------------------------- | ----- |
| Inputs the adapter accepts | 39    |
| Inputs the adapter refuses | 37    |

Every registered adapter is exercised — the seven run-shaped
(`entries`, `junit`, `cargo-mutants`, `sbom`, `agent-eval`,
`contract-conformance`, `differential-report`) and the three finding-shaped
(`sarif`, `audit-script`, `cargo-audit`) — and `tc_456_006` fails if a new
adapter is registered without a case.

9 cases read **unedited real producer output** from
`tests/fixtures/evidence/`, whose own `README.md` carries that directory's
provenance. The rest are synthetic inputs lifted from
`tests/evidence-adapters.test.ts`, which exist to reach the edges real output
does not contain. **No SARIF fixture is checked in**, so every SARIF case here
is synthetic; that is a gap in the fixture directory, not a choice made here.

## Known divergences

Three, all recorded rather than silently reproduced or silently fixed:

1. **`entries`: unchecked cast.** The TypeScript does
   `parsed.entries as RunEntry[]` and never inspects the elements, so a
   malformed element reaches the record. The Rust deserialises into `RunEntry`
   and refuses under the same message. No golden case covers a malformed
   element, because the two implementations genuinely differ there.
2. **`sbom`: unrecognised-key listing order.** The TypeScript lists the first
   six top-level keys in document order; `serde_json::Map` is a `BTreeMap`, so
   the Rust lists them in byte order. Diagnostic text only — no record byte
   changes. The golden cases avoid a multi-key unrecognised document.
3. **Refusal text embedding a `JSON.parse` detail.** Messages of the form
   `… not JSON: Expected property name or '}' in JSON at position 1` carry V8's
   wording after the marker. `golden_parity.rs` compares the prefix up to and
   including `not JSON: ` — quoin's own words — and not the tail, which no Rust
   parser reproduces.

One representation difference that is **not** a divergence: `JSON.stringify`
writes an integral `f64` as `1` where `serde_json::Value` renders `1.0`.
`quoin-store` formats record numbers with the `ECMAScript` algorithm, so the
bytes on disk are unchanged (NFR-025); the parity test normalises both sides
and leaves number formatting to the crate that owns it (FR-100-CON-4).
