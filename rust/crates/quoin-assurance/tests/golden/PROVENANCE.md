<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Golden corpus provenance — quoin#447

`expected.json` is the **only** oracle the Rust suite consults. Nothing in this
crate executes TypeScript at test time; a Rust test that shells out to Node to
decide whether it passed is not a port (quoin#373, FR-101 AC-5).

This corpus exists because `src/assurance/` is being deleted. Until now it was
the difftest's oracle for roughly a hundred `assurance/*` cases, reached through
`src/core/reference.ts`. Those cases retire with the module; what replaces them
is this: hand-authored inputs, verdicts captured once from the retained
implementation, and a replay that reads bytes.

## What produced it

|                 |                                                                                                 |
| --------------- | ----------------------------------------------------------------------------------------------- |
| Produced by     | `rust/crates/quoin-assurance/tools/generate-oracle.mts`, deleted with its subject               |
| Captured on     | 2026-09-13                                                                                      |
| Oracle revision | quoin `c2d7f54` — the commit BEFORE the deletion                                                |
| Runtime         | node v22.15.0, vitest 4.1.10                                                                    |
| Oracle entry    | `src/assurance/index.ts` — `requirementOf`, `buildCase`, `renderCase`, `parseAssuranceArgument` |

`src/core/reference.ts` is deliberately **not** the oracle. Its assurance
handlers are a hand-written mirror of the Rust request schema, written so the
difftest could compare malformed input; capturing from them would be capturing
the port's own schema back from a second copy of itself. The capture imports the
retained library directly, and nothing from `dist/` or from `rust/`.

### Oracle bytes

```
c953693c586623f69f6e21502c1fa9ce21b164c4d2a6038e9bec3a40c643aa6b  src/assurance/argument.ts
9c7790f154cfae78ae2260d7d810db0aae1a9fc5847ea9113286d6830a331d6f  src/assurance/discharge.ts
691ae36214fb70bdb201ad571e0f1646147ae8b08fe76bfeb0f28ae48a849d2f  src/assurance/graph.ts
3da7ff57990c6a30051ccdd17f2ee2c17ee9201f2a26b8077187bd5c8693945e  src/assurance/index.ts
92084646d5640c8d8ae3fe7ad7e87921b30c4148ad2c25f7679e88234135d535  src/assurance/render.ts
8e021a9ecae8894913d61f537c3ed0f4d0390f0e4c8ab4cb6e62705154d7636a  src/measurement/date-time.ts
```

The sixth file is not under `src/assurance/` and is listed all the same:
`argument.ts` imports `parseRfc3339DateTime` from it at run time, so it is part
of what answered every `review_by` case below. The other two cross-directory
imports (`../evidence/index.js`, `../quire/index.js`) are `import type` and
contribute no behaviour.

If any of those hashes changes, the TypeScript moved and this corpus is stale.
Re-capture it deliberately, and say in the commit message what behaviour
changed — a golden that is refreshed as a matter of routine has stopped being a
gate.

### Corpus bytes

```
94f672d8c7ced7c05d567e3e091858115a38ce6d5438f89240b5003022b86c40  tests/golden/cases.json
96e7be27a29be0b9e56ae5f3c300c4296bc55e098d349c69930872e8e27e88db  tests/golden/expected.json
```

## Reproducing it

The capture cannot be re-run on this revision, and that is the point. The commit
that follows the capture deletes `src/assurance/` — the corpus exists precisely
because the oracle does not survive it — and the generator went with its subject
rather than staying behind as a script importing a module no longer in the tree.
FR-101 AC-5 forbids a non-Rust runtime oracle after cutover; a dormant one that
a future reader might try to run is the same thing with a longer fuse.

To re-derive the bytes, check out the revision where both still exist and run it
there:

```sh
# c2d7f54 is the commit before the deletion: subject and generator both present.
QUOIN_ORACLE_WRITE=1 pnpm vitest run \
  --config rust/crates/quoin-assurance/tools/vitest.oracle.config.mts
```

Verify — re-derive and compare to the committed bytes without writing anything —
by running the same command without `QUOIN_ORACLE_WRITE`. That run compares
**bytes**, not parsed JSON: the Rust suite asserts against these exact bytes, so
"equivalent JSON" is not the property under test.

The hashes above are what makes this checkable. They pin the six files that
answered every case; if a re-derivation at that revision disagrees with
`expected.json`, one of them moved, and this document says which.

## What is in it

128 cases in `cases.json`, each carrying a `name`, the requirement ids it
`covers`, an `op`, and its input. `expected.json` holds one entry per case, in
the same order, recording `ok: true` with the captured `value` or `ok: false`
with the thrown message.

| operation        | cases | captured a value | threw |
| ---------------- | ----: | ---------------: | ----: |
| `requirement_of` |    14 |               14 |     0 |
| `build_case`     |    33 |               32 |     1 |
| `render_case`    |    29 |               25 |     4 |
| `parse_argument` |    52 |               10 |    42 |
| **total**        |   128 |               81 |    47 |

A `parse_argument` case is `argument_base` plus one variation — `overrides` sets
top-level keys, `remove` deletes them, `review_by` supplies one assumption's
instant. `remove` exists separately from `overrides` because an explicit `null`
is itself under test: "absent" and "null" have to stay distinguishable. Both the
capture and the two Rust replays rebuild the request the same way; the
reconstruction is duplicated in three places on purpose, so a change to one
cannot silently move the corpus under the other two.

The captured `error` text is recorded **for a reader and never asserted**.
`quoin-difftest` states the rule this corpus inherits: the verdict is
contractual and the message prose is not, so a refusal is asserted by its exit
class and its diagnostic code and nothing else.

**This is not a fixture the Rust tests wrote for themselves.** `cases.json` is
hand-authored; every expected value in `expected.json` came out of the
TypeScript. The Rust assertions can fail, and two of them did on first run —
see the divergences below.

### What the corpus covers that the difftest did not

Beyond every input shape `quoin-difftest` already exercised: a supported
StR→FR→AC tree; a finding opening a leaf and propagating up two levels; a bare
claim with nothing under it; an unreachable requirement; a requirement traced
from two claims; a two-node and a three-node refinement cycle; caller-supplied
`claim_types`; case-insensitive claim-type matching in both directions; an empty
case; a statement carrying `(`, `;` and `"`; a label well over eighty
characters; a label carrying an astral-plane character; a satisfied
`evidence_independence` entry; and an invalidated `producer_trust` entry.

Three difftest shapes are **not** representable here and are not in the corpus:
the two transport-level inputs (`"{oops"` and a bare array reaching the
dispatcher, which are not valid corpus JSON values paired with an op) and the
16 MiB size-boundary cases, which would make the checked-in corpus larger than
the repository. `quoin-core`'s own `ops::assurance` unit tests keep those.

## Population, stated

Stated in numbers, because a comparison that runs over one half of a population
is green and says nothing.

**Verdicts.** 81 of the 128 cases captured a value and 47 captured a throw. The
throws are not incidental: `parse_argument` is a validator, and 42 of its 52
cases are the contract's refusals — nine impossible or misspelled instants, five
challenge defects, three empty-or-blank owners, five closed-vocabulary
violations, two unknown-key refusals, and eighteen further shape, uniqueness
and reachability rules.

**Findings.** Of the 32 `build_case` cases that captured a value, 5 supply a
non-empty `findings` list and 27 supply none; 13 produce a case containing at
least one `open` node and 7 produce a case that is wholly `supported`. A finding
is not the only way to open a node — an obligation nothing binds opens one too —
which is why those two counts differ.

**Empty against populated.** 12 of those 32 `build_case` cases produce an EMPTY
case (no claims at all, with the retained `reason` field set) and 20 produce a
populated one. On the render side, 10 of the 25 `render_case` cases that
captured a value render the empty-case prose and 15 render a populated
document; 8 of them contain the `◇` open mark.

**Refused at the request schema.** 13 cases are marked `"boundary": "refused"`:
inputs the Rust request schema turns away before the ported logic runs. The
retained TypeScript answered 8 of them anyway and threw on the other 5. See the
first divergence below.

## Known TypeScript↔Rust divergences

Three, all found by running this corpus against the port, all recorded rather
than papered over. None was hidden by adjusting a case, an assertion, or the
port.

1. **The request schema refuses input the retained library accepts (8 cases).**
   The retained functions are plain JavaScript: an unknown request field is
   ignored, a non-string `obligation_id` is coerced, a `CaseNode.kind` outside
   `goal | strategy | solution` is interpolated verbatim, a `Finding` missing
   `kind` renders `undefined`. The port types all four, so `serde` refuses the
   request with `CORE_BAD_REQUEST` and exit 3. This is the port declining to
   reproduce a coercion, not a behaviour difference in the logic, and it is the
   reason those cases carry `"boundary": "refused"` — their TypeScript verdict
   is still captured so the difference stays visible in the bytes.

2. **`build_case` carries assessments `render_case` will not read (2 cases).**
   `producer_trust` and `evidence_independence` are OPAQUE to `build_case` on
   both sides — the retained implementation copies them unread and the port
   types them as `serde_json::Value` — so both carry an assessment missing
   `useId`, or one whose `triggeredBy` holds objects rather than strings. The
   retained RENDERER accepts those too, interpolating `undefined` and
   `[object Object]`; the port's `TrustAssessment` and `IndependenceAssessment`
   are typed, so it refuses the request instead. 27 of the 29 built cases
   re-render at the boundary and 2 do not.
   `quoin-core`'s `tc_447_512` asserts both halves, and `quoin-evidence-types`'
   `TrustAssessment` docs already named `[object Object]` as the defect being
   declined.

3. **A non-object request is refused one layer earlier.** `parse_argument`'s
   request IS the argument, so a bare array is turned away by the DISPATCHER as
   `CORE_BAD_JSON` rather than by the handler as `CORE_BAD_REQUEST`. Same exit
   status, different diagnostic code, and the difference is contractual: a
   request that is not a JSON object never reaches an operation.

One difference is deliberate and is **not** a divergence, because it changes no
observable result: the capture serialises with `JSON.stringify` and the port
with `serde`, so the two files order object keys differently. The Rust replays
compare `serde_json::Value`s, which are order-insensitive; the vitest verify run
compares bytes, which is where key order is pinned.

### Where the port was already right

Two hazards this corpus was authored to catch, and did not:

- **`String.prototype.trim` against `char::is_whitespace`.** They disagree on
  U+FEFF and U+0085, in OPPOSITE directions. The five blank-owner cases pin all
  of it: U+FEFF and U+00A0 owners are refused, U+0085 and U+200B owners are
  accepted, and the port reproduces every one. `quoin-assurance`'s
  `js_trim_is_empty` transcribes the ECMA-262 set for exactly this reason.
- **Mermaid label truncation.** `MAX_LABEL_LENGTH` is 80 CODE POINTS, not
  UTF-16 units; the over-eighty-character label and the astral-plane label pin
  both the cut and the two-underscore node id an astral character produces.
