# `quoin-measurement-graph` — declared divergences

<!--
Trace: FR-066-AC-1, FR-067-AC-1, FR-100-AC-4, FR-101-AC-5
Provenance: quoin#475, quoin#476, quoin#465
-->

## §1 — What is in this file, and the rule

This crate ports two retained modules:

| retained TypeScript                              | wave | ticket    |
| ------------------------------------------------ | ---- | --------- |
| `src/measurement/graph-adapters.ts` (763 lines)  | W8   | quoin#475 |
| `src/measurement/graph-portfolio.ts` (881 lines) | W9   | quoin#476 |

Both are gated on byte-identity against a TypeScript oracle captured once and
committed (`tests/corpus/` for W8, `tests/goldens/graph-portfolio-oracle.json`
for W9). No test spawns node; `tc_476_022_no_test_runs_the_retained_typescript`
asserts that as a census over the test directory.

**A divergence is a place where the two implementations would answer
differently.** It is declared here only when it can actually be reached. Where
a difference cannot be reached, it is _proved_ unreachable (§7) rather than
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
a _different measure name_ in the partition key, not merely a different
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
producing bytes to compare against.

**The census that bounds it instead** is
`tc_476_024_no_retained_dimension_value_is_a_non_string`, which walks
`spec/evidence/measurements/` — that directory only, since the repository holds
several checkouts of itself under `.worktrees/` and a walk that reached them
would count the same observation many times. At the revision that wrote this
paragraph it read:

| counted                                             | at this revision |
| --------------------------------------------------- | ---------------- |
| `.json` collection files directly in that directory | 48               |
| members of their `observations` arrays              | 14,644           |
| observations stating a `dimensions` object          | 14,339           |
| name/value pairs inside those objects               | 50,882           |
| **of those values, non-strings**                    | **0**            |

The test asserts the zero, not the totals: the totals move as the corpus grows,
so it carries floors an order of magnitude below them and fails if the
population it reads collapses. It also asserts that some observations state no
`dimensions` at all, which is the case the fixture _does_ cover, above.

Two things this census is **not**. It is not a guarantee: it is a fact about
the retained corpus at one revision, and an observation that broke it would be
refused by [`quoin_measurement::validate`] before reaching either projection.
And it says nothing about `graph_quality` in particular — the retained corpus
contains **no** `graph_quality` observation at all. The only ones in this
repository are this wave's own fixtures, and a bound quoting those would be a
self-fixture claim rather than a measurement.

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

The consequence at _this_ call site: where the retained code would decode
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
`2026-01-01T00:00:00` read as _local_ time). [`order::compare_instants`] reads
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

**quoin#477 widens this entry rather than adding a second.** The loader has a
_second_ reader of the same grammar: `graph-portfolio-load.ts:35` refuses a
collection whose timestamp `!Number.isFinite(Date.parse(…))`, and
[`loader::readable_or_undated`] reads it with the same
[`quoin_measurement::Rfc3339DateTime`]. The consequence differs from the
ordering one above: there, a V8-heuristic timestamp reorders history; here it
turns a collection the retained loader _reads_ into one this loader refuses
with `…: collection timestamp is not a valid instant`. Same grammar, same
inputs, one more place it is applied. The refusal sentence itself is
byte-identical and `tests/tc_477_graph_loader.rs` compares it as such, over
`graph-loader-tree/echo`, whose `e-undated.json` both grammars refuse.

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

This matters for what the golden proves. The _inputs_ are shared data; the
_outputs_ come from the retained TypeScript, so the comparison is not a
self-fixture tautology. But it means **the loader itself is not covered by this
wave** — `loadGovernedGraphPortfolio`'s own behaviour (which repositories it
discovers, which refusals it synthesises) belongs to whichever wave ports
`graph-portfolio-load.ts`, not to this one.

That wave is quoin#477, and it did not extend this golden. It captured its own,
`tests/goldens/graph-loader-oracle.json`, from `graph-portfolio-load.ts` over
its own tree `tests/fixtures/graph-loader-tree/` — a second tree rather than
more repositories in this one, so that a change to the loader's fixtures cannot
move this projection's captured bytes. §9 states what that golden compares.

## §9 — a graph refusal's _verdict_ is compared; its _sentence_ is not

`loadStructuralGraph` (`graph-portfolio-load.ts:113-124`) answers a failed
graph load with three things: an availability, a path, and a reason. The first
two are this port's own and are compared byte for byte. The third is written by
whoever refused, and for two of the three refusal classes that is not either
tree:

| refusal                                 | retained sentence                                                             | here                                                                   |
| --------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| the file is absent                      | `cannot read export input <p>: ENOENT: no such file or directory, open '<p>'` | `cannot read export input <p>: No such file or directory (os error 2)` |
| the document is not an assurance export | zod's issue prose                                                             | this workspace's sentences (quoin#403)                                 |
| the mapping was refused                 | `no graph export, premises, or audit mapping was supplied`                    | the same, byte for byte                                                |

Neither side can be made to write the other's. The first quotes an operating
system through two different runtimes; the second quotes two different schema
validators, and quoin#403 already declared that difference where it lives, in
`quoin-graph-analysis`. Restating it as prose to be matched would mean this
crate carrying a copy of another crate's error text.

**What is therefore compared, and where.** `tests/tc_477_graph_loader.rs`
splits the golden into two case lists rather than exempting a field inside one:

- `tc_477_010_byte_identical_report` runs over `alpha`, `bravo` and `echo` —
  a graph that loads, a mapping with no documents, and a collection no instant
  grammar reads — and compares the **whole** canonical JSON and the **whole**
  rendered Markdown. Every sentence in those, including the two refusals this
  loader synthesises itself, is byte-identical.
- `tc_477_011_graph_refusal_verdicts` runs over `charlie` (an export that is
  not there) and `delta` (an export that is not an assurance export) and
  compares availability and path only, with an anti-vacuity assertion that the
  two verdicts differ from each other.

An exception inside a byte gate is not a byte gate; two gates with stated
scopes are.

**The distinction itself is not prose-matched.** `graph-portfolio-load.ts:117`
separates `missing` from `unreadable` with
`/ENOENT|no such file/i.test(loaded.error.message)`, a regular expression over
whichever sentence node happened to produce. [`structural::RecordingReader`]
keeps the [`std::io::ErrorKind`] the [`quoin_graph_analysis::GraphInputReader`]
seam already reported, so the same decision is read off a type. The two agree
on every input except one that cannot arise here: a refusal that is _not_ a
`NotFound` and whose message nevertheless contains `no such file` — for
instance a permissions error whose text quoted a missing path. There, the
retained loader says `missing` and this one says `unreadable`. No such input
exists in the fixture tree and none can be constructed through the seam without
a reader that lies about its own `ErrorKind`.

## §10 — `portfolio omitted resolved repository` is unreachable, not declared

`graph-portfolio-load.ts:74-77` throws
`portfolio omitted resolved repository ${root}` when the ungoverned portfolio
report has no entry for a root the mapping parser resolved. It cannot fire, on
either side, and the reason is structural rather than incidental:

1. Every root handed to `buildPortfolioReportFromCollections` comes from the
   same `mappings` list that is then iterated — [`loader`] builds both from one
   `parse_graph_portfolio_mappings` result.
2. Both sides resolve a root exactly once, with the same function
   ([`quoin_measurement::portfolio::location::resolve_against`]), so the string
   looked up is the string stored.
3. `build_portfolio_report_from_collections` emits one entry per snapshot it is
   given and drops none.

Following §7's rule, it is **not** declared as a divergence: a declaration that
cannot fire is a licence rather than a measurement. It is also not `unwrap`ed
away — [`loader::repository_input`] returns
[`crate::GraphAdapterErrorCode::Measurement`] with the retained sentence — because
a library that panics on an impossible state has still panicked. The refusal is
an unreachable arm carrying the retained words, not a live divergence.
