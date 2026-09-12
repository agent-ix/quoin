# `filament-ide-rs-coverage.json` — provenance

The volume fixture for quoin#379, and the successor to the payload
agent-ix/quoin#164 was filed about.

## What it is

`quire coverage --scope /home/peter/dev/filament-ide-rs --json`, captured once
on **2026-09-12** and checked in. It is a real corpus payload, not a generated
one: 139 documents contributing criteria, 1,215 obligations, 2,749 reference
rows of which 1,555 are backed.

| fact | value |
|---|---|
| bytes | 1,632,821 |
| producing CLI | `quire` 0.32.0 (cli `55a1c4e2`) |
| producing engine | `a874fb641cb70da83c8c8b23f9fea0a44255b88a` |
| `totals` | `backed 1555 / total 2749`, `criteria 1007`, `property_shaped 553`, `specific_shaped 79` |
| exit status | 0 |

## Why this file exists

`src/quire/exec.ts` raised Node's `maxBuffer` to 64 MiB because this same
corpus emitted **1,090,714 bytes** — 4% over Node's 1 MiB default — and killed
all six quoin commands that shelled out (#164). The corpus has since grown: the
payload is now 1,632,821 bytes, 50% larger than the one that caused the
incident, and still one payload for one repository.

The capture also reproduces the noise that made #164 hard to diagnose: the
producing run wrote `DuplicateArchetype: 'ADR' contributed by modules [...];
first-wins` to stderr, which is the text the old error message appended to an
`ENOBUFS` death and which was then investigated as the cause.

## The TypeScript oracle, run once

Captured with the retained TypeScript, at quoin `main` (`4d27dcf`), on the same
date:

```ts
import { parseCoverage } from "./src/quire/index.js";
parseCoverage(readFileSync(fixture, "utf8"));
// => { ok: true }, totals { backed: 1555, total: 2749, criteria: 1007,
//                           property_shaped: 553, specific_shaped: 79 }
```

`parseCoverage` compiles the vendored `coverage-v1.schema.json` with ajv and
validates the payload against it. It accepted this payload with zero errors.
That result is recorded **here**, once, as the note this file's tests cite.
**No TypeScript runs at test time**: `tests/volume.rs` asserts the same facts
against the same bytes through the Cargo edge, and nothing in the Rust suite
shells out to `node`, `quire`, or anything else.

## Refreshing it

Don't, unless a test needs a shape this payload cannot carry. It is a
*historical* artifact as much as a fixture: its value is that it is the real
output of a real corpus at a known size, and a regenerated copy from a
different day measures a different corpus.
