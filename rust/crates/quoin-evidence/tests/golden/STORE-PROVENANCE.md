<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Store golden corpus — provenance

## What this is

`store-expected.json` is the output of the **retained TypeScript** evidence
store — `store.ts`, `trust.ts`, `independence.ts`, `assurance-records.ts` and
`mock-inspection.ts` — over `store-cases.json`, captured once and checked in.
`tests/store_parity.rs` asserts the Rust store half reproduces it. No Node
process runs in the Rust test lane (FR-101-AC-5); that is the whole point of
capturing rather than shelling out.

The `canonical` cases are captured differently from the rest, deliberately: the
capture script calls the retained **writers** into a temporary store root and
records the store-relative path and the exact bytes each writer put on disk.
NFR-025 is a statement about the store's files, so the oracle for it has to be
the file, not a hand-built object passed through `canonicalJson`.

## Producer

| Field           | Value                                                                                                    |
| --------------- | -------------------------------------------------------------------------------------------------------- |
| Oracle          | `src/evidence/{store,trust,independence,assurance-records,mock-inspection}.ts` (the retained TypeScript) |
| Oracle revision | quoin `a2b5dfc`; those files last changed at `e31fbb9675dbd884bed9dd98df31be630f359df1`                  |
| Capture script  | `rust/crates/quoin-evidence/tools/generate-store-oracle.mts`                                             |
| Capture config  | `rust/crates/quoin-evidence/tools/vitest.store-oracle.config.mts`                                        |
| Capture date    | 2026-09-13                                                                                               |
| Runtime         | Node v22.15.0, pnpm 11.20.0, vitest 4.1.10                                                               |

## Oracle file digests (SHA-256)

Verify these before recapturing. A capture taken against different bytes is a
different oracle and must say so.

```
484681bc681321e0af163553644028b66ee8474fdb3031a33033980209224581  src/evidence/store.ts
12d1a67f39bb5e1b887131a6a87d4d40870ec5d59ae29fac48429a477e24c654  src/evidence/trust.ts
ee9e3103b5476b97e2d2880513b112ffe8dba361ce0cc637be4478268b9a9103  src/evidence/independence.ts
d21c518128fdc86c65eefac54ea24482adb2ab971539ff21057c704755ed32ca  src/evidence/assurance-records.ts
bdae6948014611bbd150cb303d5b6c1f112f2f0df21571deda4e660f80eab41a  src/evidence/mock-inspection.ts
b37efd00f5331abd66bf4991019032ae766f054c47dbfeec00d7653bc76ac837  src/store/paths.ts
```

Captured artefacts:

```
f93586424ae93d6a05b1a4b89556563211f50b18871b023ea9c0eb408e883c17  rust/crates/quoin-evidence/tests/golden/store-cases.json
361707a1ef295329e61801cb79dae635b51f4590cfea54772bb8035fc536f8ca  rust/crates/quoin-evidence/tests/golden/store-expected.json
bbfc4c79b8ae0b8eed50723727cef5ef328c82a9a28cded6d54fd9ae1e831843  rust/crates/quoin-evidence/tools/generate-store-oracle.mts
```

## Reproduction

From the repository root. Verify the committed golden still reproduces (writes
nothing):

```bash
pnpm vitest run --config rust/crates/quoin-evidence/tools/vitest.store-oracle.config.mts
```

Regenerate it, deliberately:

```bash
QUOIN_ORACLE_WRITE=1 pnpm vitest run \
  --config rust/crates/quoin-evidence/tools/vitest.store-oracle.config.mts
```

Assert the Rust side against it, from `rust/`:

```bash
cargo test -p quoin-evidence --target-dir target --test store_parity
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

82 cases. **Both halves are non-empty**, which the capture script asserts and
`store_parity.rs` re-asserts:

| Half                     | Count |
| ------------------------ | ----- |
| Inputs the store accepts | 59    |
| Inputs the store refuses | 23    |

Every ported behaviour family is exercised, and `tc_456_101` fails if a family
drops below three cases:

| Kind           | Count | Behaviour                                             |
| -------------- | ----- | ----------------------------------------------------- |
| `trust`        | 26    | `validateTrustDecision` + `assessTrust`               |
| `assurance`    | 15    | experiment and operational-evidence record validation |
| `canonical`    | 11    | the exact bytes each retained writer puts on disk     |
| `independence` | 10    | `assessIndependence` over a binding graph             |
| `mock`         | 7     | `inspectMockInjections` over a seeded working tree    |
| `bind`         | 6     | `bind` — created, suspect and unchanged outcomes      |
| `affirm`       | 4     | `affirm` — found, not found, suite-narrowed           |
| `vacuity`      | 3     | `scanIsVacuous`                                       |

## Known divergences

Seven, all recorded rather than silently reproduced or silently fixed:

1. **zod's own clause prose is not byte-copied.** Where a TypeScript refusal
   clause is zod's built-in text (`Too small`, `Invalid input`,
   `Invalid string`), `store_parity.rs` asserts only that the Rust message names
   the same **path** (`owner`, `schemaVersion`, `revalidateOn`, …). Quoin's own
   words are compared in full.
2. **The assurance validators accumulate.** The Rust reports every failing
   clause for a record; the retained TypeScript throws on the first. Every
   TypeScript clause still appears in the Rust message, so the parity comparison
   is containment per clause, not equality.
3. **Trust-decision id refusal moves earlier.** `TrustDecisionId` is a newtype
   with `#[serde(try_from = "String")]`, so `ETD-x` is refused at
   deserialization (`invalid trust decision id 'ETD-x'`) where the TypeScript
   refuses it inside `validateTrustDecision`.
4. **`is_instant` is narrower than `Date.parse`.** Rust accepts RFC 3339; V8
   accepts a wider, implementation-defined set. No case in the corpus sits in
   the gap, and none should: a store that accepts `"March 3"` as a timestamp is
   a defect in the oracle, not a behaviour to port.
5. **Typed deserialization refuses records the retained reader returned.** The
   TypeScript readers cast; the Rust readers deserialize into the record type,
   so a structurally malformed record is skipped (and named in `skipped`) rather
   than handed to the caller half-formed.
6. **`gc()` returns store-relative paths**, where the retained implementation
   returns absolute ones. The library half names no host capability
   (FR-100-CON-1); the caller joins the store root.
7. **Byte offsets, not UTF-16 offsets**, in the two mock-inspection context
   windows. Every fixture is ASCII, so no difference is observed; a non-ASCII
   line before a match would shift the window.

One representation difference that is **not** a divergence: `JSON.stringify`
writes an integral `f64` as `1` where `serde_json::Value` renders `1.0`.
`quoin-store` formats record numbers with the `ECMAScript` algorithm, so the
bytes on disk are unchanged (NFR-025); the parity test normalises both sides
and leaves number formatting to the crate that owns it (FR-100-CON-4).
