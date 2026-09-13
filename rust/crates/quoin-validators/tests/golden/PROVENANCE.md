<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Golden corpus provenance — quoin#377

`expected.json` is the **only** oracle the Rust suite consults. Nothing in this
crate executes TypeScript at test time; a Rust test that shells out to Node to
decide whether it passed is not a port (quoin#373, AC-5).

## What produced it

|                  |                                                                                                                                                                 |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Produced by      | `../../tools/generate-oracle.mts`                                                                                                                               |
| Captured on      | 2026-09-12                                                                                                                                                      |
| Oracle revision  | quoin `4d27dcf1621d8c28da0961a5521a5be7b6d1cd28` (`main`)                                                                                                       |
| Capture checkout | `888840185a917546b3ea1b65a5fb160495ceb58e` (`spec/373-rust-burn-down`), whose three oracle files are byte-identical to `4d27dcf` — verified by the hashes below |
| Runtime          | node v22.15.0, vitest 4.1.10                                                                                                                                    |

### Oracle bytes

```
6800c434299a35b915d177727453f371b5563c97a4324220d9890ba0999aac79  src/validators/gates.ts
ad0c05c35cc68e1f861fd12ae71f40c101920d33301855548fb5d9100b2f0b2a  src/validators/index.ts
df1d713391c702b8e9bd15c70e94d94fafe24d609ee3cc5aa57549eba7e522a0  src/commands/validate.ts
```

If any of those three hashes changes, the TypeScript moved and this corpus is
stale. Re-capture it deliberately, and say in the commit message what behaviour
changed — a golden that is refreshed as a matter of routine has stopped being a
gate.

## Reproducing it

The capture is a script, not a test. It is excluded from `pnpm test` by the
`rust/**` entry in `vite.config.ts` **and** by its name, and it writes nothing
unless `QUOIN_ORACLE_WRITE=1` is set. Run from the repository root:

```sh
# Verify: re-derive the verdicts and compare them to the committed bytes.
pnpm vitest run --config rust/crates/quoin-validators/tools/vitest.oracle.config.mts

# Regenerate: overwrite expected.json. A decision, not a convenience.
QUOIN_ORACLE_WRITE=1 pnpm vitest run \
  --config rust/crates/quoin-validators/tools/vitest.oracle.config.mts
```

The verify run compares **bytes**, not parsed JSON: the Rust suite asserts
against these exact bytes, so "equivalent JSON" is not the property under test.

## What is in it

46 cases, 28 findings, materialised from `cases.json`. Each case records four
things captured from the retained implementation:

- `findings` — the return of `inspectEmptyGates(root)`
- `json` — the exact stdout of `quoin validate --repo <root> --json`
- `human` — the exact stdout lines of `quoin validate --repo <root>`
- `strictExit` — the process exit code of `quoin validate --repo <root> --strict`

The capture asserts `JSON.parse(json)` equals `{ findings }` before writing, so
the library result and the shipped command cannot be recorded as two different
oracles under one name.

**This is not a fixture the Rust tests wrote for themselves.** `cases.json` is
hand-authored input; every expected value in `expected.json` came out of the
TypeScript. The Rust assertion can fail, and does fail if the port is wrong — the
port was developed against 15 unit tests first and then run against this corpus
cold.

## Population, stated

23 of the 46 cases expect findings and 23 expect none. A check that only ever
ran over the empty half would pass vacuously, so both halves are asserted, and
`tc_377_020` asserts the corpus itself still contains both — a corpus silently
trimmed to only-negative cases is a green suite that measures nothing.

## Known TypeScript↔Rust divergences

Two, both unreachable from this corpus and from any realistic repository, both
recorded rather than papered over:

1. **Sort collation.** JavaScript's `<` on strings compares UTF-16 code units;
   the Rust port compares UTF-8 bytes. These disagree only for paths containing
   characters at or above U+E000 mixed with astral-plane characters. No
   repository path in the corpus is affected.
2. **Unicode case folding in the claim regex.** `[A-Z]` under `/i` in JavaScript
   folds via `toUpperCase`, while the `regex` crate folds via Unicode simple case
   folding. They differ on exactly one character, U+212A KELVIN SIGN, which Rust
   accepts as `K` in an obligation id and JavaScript does not.

Two further differences are deliberate and are **not** divergences, because
neither changes any observable result:

- `references_script` drops the TypeScript's `source.includes("./" + repoPath)`
  arm, because `source.includes(repoPath)` is true whenever that arm is, and
  the port keeps one site rather than two that must agree.
- Wiring bodies are **cached** after their first read. The TypeScript re-reads
  the same body once per candidate script. Caching only ever removes a re-read
  of a file already opened, so the set of files opened is unchanged.

Reading the wiring _eagerly_ would not have been in that class, and an earlier
revision of this crate did exactly that. `inspectEmptyGates` calls
`readFileSync` **inside** `wiring.find(...)`, so a repository whose scripts
declare no negative gate claim opens no wiring file at all; an unreadable
`Makefile` that the oracle never touches would have become a `QV-E003` refusal
where the oracle returns a clean verdict. The port now reads on demand and
`tc_377_023` pins both halves of that behaviour.
