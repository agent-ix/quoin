<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Golden corpus provenance — quoin#377

`expected.json` is the **only** oracle the Rust suite consults. Nothing in this
crate executes TypeScript at test time; a Rust test that shells out to Node to
decide whether it passed is not a port (quoin#373, AC-5).

## What produced it

| | |
|---|---|
| Produced by | `generate-oracle.test.ts` in this directory |
| Captured on | 2026-09-12 |
| Oracle revision | quoin `4d27dcf1621d8c28da0961a5521a5be7b6d1cd28` (`main`) |
| Capture checkout | `888840185a917546b3ea1b65a5fb160495ceb58e` (`spec/373-rust-burn-down`), whose three oracle files are byte-identical to `4d27dcf` — verified by the hashes below |
| Runtime | node v22.15.0, vitest 4.1.10 |

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

A third difference is deliberate and not a divergence: `references_script` drops
the TypeScript's `source.includes("./" + repoPath)` arm, because
`source.includes(repoPath)` is true whenever that arm is, and the port keeps one
site rather than two that must agree.
