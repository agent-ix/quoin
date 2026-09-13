# `quoin-measurement-graph` — declared divergences

<!--
Trace: FR-066-AC-1, FR-067-AC-1, FR-100-AC-4, FR-101-AC-5
Provenance: quoin#475, quoin#476, quoin#465
-->

## §1 — What is in this file, and the rule

This crate ports two retained modules:

| retained TypeScript | wave | ticket |
| --- | --- | --- |
| `src/measurement/graph-adapters.ts` (763 lines) | W8 | quoin#475 |
| `src/measurement/graph-portfolio.ts` (881 lines) | W9 | quoin#476 |

Both are gated on byte-identity against a TypeScript oracle captured once and
committed (`tests/corpus/` for W8, `tests/goldens/graph-portfolio-oracle.json`
for W9). No test spawns node; `tc_476_022_no_test_runs_the_retained_typescript`
asserts that as a census over the test directory.

**A divergence is a place where the two implementations would answer
differently.** It is declared here only when it can actually be reached. Where
a difference cannot be reached, it is *proved* unreachable (§7) rather than
declared, because a declaration that cannot fire is a licence rather than a
measurement. Where it can be reached, this file says what the fixture covers
**and what it cannot**, rather than widening the fixture until everything looks
covered.

W8 declared its two divergences in module documentation
(`src/base64.rs`, `src/attestation.rs`) and in `DECLARED_DIVERGENCES` in
`tests/tc_475_parity.rs`, and landed no `DIVERGENCE.md` — the only ported crate
without one. W9 consolidates them here; the module docs and the test table
stay, and this file is the index over them. **Reported against quoin#475
rather than absorbed silently.**

## §2 — `dimensions` members are read as strings, or the partition is `unknown`

`graph-portfolio.ts:770-778` builds a partition identity from
`row.dimensions?.measure ?? "unknown"` and the same for `dimension` and `key`.
`dimensions` is declared `Record<string, string>` and **nothing validates that
declaration**: a collection whose observation carries `"key": 7` puts the
number `7` into the identity, where it is compared with `<` against strings and
later rendered by `markdownCell`, which calls `.replace` on it and throws.

[`history::partition_identity`] reads through
[`quoin_store::JsonValue::as_str`] and falls back to `"unknown"` for a
non-string member, exactly as it does for an absent one. A number here would be
a *different measure name* in the partition key, not merely a different
rendering, so this deliberately does **not** route through
[`quoin_measurement::common::scalar::js_string`] — stringifying it would invent
a partition that neither implementation names.

So on such an input the retained module throws part-way through rendering and
this one reports a partition named `unknown`.

**What the fixture covers:** the **absent** case, which both implementations
agree on. `alpha`'s two governed collections each carry one observation that
states no `dimensions` at all, so the partition `unknown | unknown | unknown`
is inside the compared bytes — it is named, ordered against real partitions,
and rendered. `tc_476_001` asserts the capture names at least two such
partitions, so the fallback cannot quietly stop being exercised. Every other
partition in the tree carries string `measure`, `dimension` and `key`.

**What it cannot cover:** the non-string case is not in the fixture, because a
fixture carrying it would have no oracle — the TypeScript throws rather than
producing bytes to compare against. The census that bounds it instead:
`spec/evidence/measurements/` holds **581,130 dimension values across 161,421
observations in 1,522 collection files, and every one of them is a string**;
17 of those observations are `graph_quality`. That is a fact about the corpus
at this revision, not a guarantee. An observation that broke it would be
refused by [`quoin_measurement::validate`] before reaching either projection.

## §3 — base64 decoding refuses what `Buffer.from` accepted (quoin#465)

Declared by W8 in `src/base64.rs`; **widened by W9, not re-declared.**

Node's `Buffer.from(text, "base64")` silently discards every character outside
the alphabet, does not require padding, and stops at the first `=`, so
`Buffer.from("aG!!VsbG8=", "base64")` is `"hello"`. [`base64::decode`] refuses
all three forms, because the whole point of a `bytesBase64` attachment is that
its digest is checkable and a decoder that invents a decoding for corrupt input
makes the check pass over the wrong bytes.

W8 named the encode call site (`graph-adapters.ts:564`). **The decode call site
is in W9's module**: `graph-portfolio.ts:786`, inside the retained scorer
integrity check. `scorer.rs` routes it through `crate::base64::decode` — there
is no second decoder in this crate, and
`tc_476_021_there_is_no_second_codec_digest_or_bridge` asserts the RFC 4648 §4
alphabet appears in exactly one module.

The consequence at *this* call site: where the retained code would decode
corrupt base64 into plausible bytes and then report a **digest mismatch**, this
one reports `unreadable — retained scorer bytes are not base64: <reason>`. Both
verdicts are `unreadable` and both stop the reading; the sentence differs.

**What the fixture covers:** `echo/e-2026-02` carries well-formed base64 whose
bytes hash to the wrong digest, so the mismatch sentence is under the byte
comparison. The lenient-input refusal is covered by
`base64::tests::tc_475_012_the_lenient_forms_node_accepts_are_refused` and by
the `DECLARED_DIVERGENCES` entries in `tests/tc_475_parity.rs`, which fail if
they stop firing.

**What it cannot cover:** a golden case whose captured bytes are the decoding
Node invented, because the two implementations produce different bytes there by
construction — that is the divergence, and capturing it would be capturing
agreement that does not exist.

## §4 — instants are read by one grammar, and the fall-through is reproduced

`graph-portfolio.ts:844-846` is
`Date.parse(a) - Date.parse(b) || compare(a, b)`. `Date.parse` is ECMA-262's
implementation-defined heuristic: beyond the Date Time String Format it accepts
whatever V8 accepts (`2026/01/02 10:00`, `Jan 1 2026`, a bare
`2026-01-01T00:00:00` read as *local* time). [`order::compare_instants`] reads
with [`quoin_measurement::Rfc3339DateTime`], this workspace's single instant
grammar (unified by quoin#440; the Stage 6 plan §5 forbids a second).

The `NaN` branch is **reproduced, not diverged**: a timestamp the grammar
refuses makes the subtraction `NaN`, which is falsy, so the retained comparison
falls through to the text order. `compare_instants` falls through on exactly
the same condition, and `order::tests::an_unreadable_instant_falls_through_to_the_text_order`
holds it there.

The divergence is narrower than the grammar difference suggests: it is only a
timestamp **`Date.parse` accepts and RFC 3339 refuses**. There, the retained
code orders two collections by instant and this one orders them by text.
Governed history is sorted by this comparison (`build.rs:78`, `build.rs:135`),
so such a timestamp can reorder the history table and change which row is
`current`.

**What the fixture covers:** RFC 3339 instants in several spellings, including
two that name the same instant, so the text tie-break is under the byte
comparison. **What it cannot cover:** a V8-heuristic timestamp — no collection
under `spec/evidence/measurements/` carries one, and W8 already measures the
refusal directly in `DECLARED_DIVERGENCES`
(`quality/timestamp/date-only`, `…/v8-heuristic-slashes`,
`…/v8-heuristic-words`), each of which must fire.

## §5 — the golden compares after one path substitution

The governed portfolio's JSON and Markdown both carry absolute repository
roots, so captured bytes would otherwise record whoever's checkout produced
them. `oracle/capture-graph-portfolio-oracle.mjs` and
`tests/tc_476_graph_portfolio.rs` both replace the fixture tree's own absolute
root with the token `@@TREE@@` before comparing, and nothing else is edited.

Neither side canonicalises: node's `path.resolve` absolutises and normalises
lexically without following symlinks, and
[`quoin_measurement::portfolio::location::resolve_against`] — which this crate
calls rather than reimplementing — does the same. A checkout reached through a
symlink therefore produces the same string on both sides.

This is a property of the comparison, not of the port. Everything to the right
of the substituted prefix is compared as written. It is the same substitution
`quoin-measurement/DIVERGENCE.md` §13.1 declares, for the same reason.

## §6 — inherited divergences, declared where they live

The governed portfolio is a projection **over** the plain portfolio report:
`buildGovernedGraphPortfolioFrom` spreads a `PortfolioRepositoryReport` and
adds three members. Everything inherited keeps the divergences declared for it
in `quoin-measurement/DIVERGENCE.md`, and they are not restated here:

- §13.2 — a caught reader error renders this workspace's message, not node's
  `fs`/`JSON.parse` prose.
- §13.3 — staleness reads instants with the crate's one grammar (the same
  grammar as §4 above).
- §13.4 — observation member fidelity and the `Dimensions` newtype.
- §13.5 — `dimensions` are ordered, not in insertion order.

Restating them here would create a second place where they could drift.

## §7 — differences proved unreachable rather than declared

Two places where the implementations are written differently and **cannot**
answer differently. Neither is declared, because a declaration that cannot fire
is a licence by another route.

1. **The lost-sample throw.** `graph-portfolio.ts:300` builds two `Map`s, unions
   their keys, and throws `"graph partition index lost its sample"` when a key
   is in neither — which cannot happen, since the key came from one of them.
   [`compare`] uses one merged `Pair` index whose `before` and `after` are
   `Option`s, so the branch does not exist to be taken. The type makes the
   throw unrepresentable; there is nothing to test because there is no input.
2. **Observation identity member ordering** (W8). The identity sorts member
   names by UTF-16 code unit where the retained module sorts by code point.
   Every member name an admissible record may carry is schema-fixed ASCII,
   where the two orders coincide, and
   `tc_475_054_the_identity_ordering_difference_is_unreachable` measures that
   over the corpus.

## §8 — what the W9 golden is, and what it is not

`tests/goldens/graph-portfolio-oracle.json` (96 KB) was produced **once** by
`oracle/capture-graph-portfolio-oracle.mjs` from the retained TypeScript, and
records `produced_by`, `produced_by_digest`, `produced_from_revision`,
`produced_by_node`, `capture_script`, `fixture_tree` and `tree_token`
(FR-101-AC-11).

The capture deliberately does **not** call `graph-portfolio-load.ts`: that
loader reaches into `graph-analysis/` (FR-062), which is a different wave's
port — it landed as [`quoin_graph_analysis`] in quoin#385 after this golden was
captured, and the loader that binds it to this projection is still retained
TypeScript with no Rust counterpart. Calling it would have made this wave's
golden depend on a module nobody had ported. Instead both sides build the entry points' inputs from the same
committed tree using readers that are already ported, and the two things no
reader supplies — the opaque FR-062 structural graph, and the collection
refusals a loader would synthesise — are committed data in
`tests/fixtures/graph-portfolio-tree/graph-inputs.json`, read identically by
the oracle and by the test.

This matters for what the golden proves. The *inputs* are shared data; the
*outputs* come from the retained TypeScript, so the comparison is not a
self-fixture tautology. But it means **the loader itself is not covered by this
wave** — `loadGovernedGraphPortfolio`'s own behaviour (which repositories it
discovers, which refusals it synthesises) belongs to whichever wave ports
`graph-portfolio-load.ts`, not to this one.
