<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

`src/measurement/` is still the oracle: quoin#468 is a **port wave**, not a
cutover, and no TypeScript was deleted. The cutover is quoin#479.

Everything the two trees agree on is checked, not asserted: all 48 collections
retained under `spec/evidence/measurements/` read through
`quoin_store::parse_strict_json`, re-serialise through
`quoin_store::canonical_json_bytes` **byte for byte**, and pass the ported
validator (`tests/tc_468_corpus.rs`). What follows is everything outside that
corpus where the two can be told apart, grouped by why.

## §1 — Divergences of strength: the Rust refuses more, or loses less

### §1.1 — publication is durable, and links rather than renames

`store.ts:31-53` and `atomic-file.ts:12-48` are two copies of one write-once
dance. `store.ts` publishes with `renameSync` after an `existsSync` test;
`atomic-file.ts`, the newer copy, already knows that rename **replaces** and
uses `linkSync` instead. Both port to
`quoin_store::store::write_content_addressed`, so:

- **`store.ts`'s rename becomes a link.** A concurrent writer whose differing
  bytes would have been silently clobbered in the window between the test and
  the rename now loses the race and gets the `ContentCollision` refusal the
  function exists to produce. Strictly a refusal the oracle sometimes fails to
  make.
- **The file and its directory entry are `fsync`ed.** The retained code can
  lose a published collection to a crash after the link returns; this cannot.
  Nothing observable changes on a machine that does not crash. This is the
  quoin#394 durability decision and the Stage 6 plan §7 ruling.

`atomic-file.ts`'s `writeFileAtomicNoReplace` has **no** separate Rust
counterpart: it and `store.ts`'s inline copy are the same function, and porting
it twice would reproduce the duplication the port exists to remove.

### §1.2 — duplicate JSON object names are refused

Collections are read with `quoin_store::parse_strict_json`, which refuses a
document whose object carries the same member name twice. `JSON.parse` accepts
one and keeps the last. No retained collection contains a duplicate name — all
48 parse — so this is unobservable on the corpus and is a refusal only for
input the oracle silently reinterprets.

### §1.3 — a single-element array is no longer a digest

`validate.ts:114,144,163` test their patterns against `String(value[key] ?? "")`
rather than against a string. Every non-string that reaches those lines fails
the pattern anyway (`String(5)` is `"5"`, `String(null)` is `""`,
`String({})` is `"[object Object]"`) with one exception: `["sha256:…"]`
coerces to `"sha256:…"` and is accepted. This crate reads strings as strings
and refuses it.

### §1.4 — `parse, don't validate` replaces the assertion functions

`validateStoredMeasurementCollection` and `validateMeasurementCollection` are
TypeScript assertion functions: they narrow the caller's `unknown` and return
nothing, so the caller keeps handling the loosely-typed object it passed in.
Here they return a `MeasurementCollection`, and identity values
(`CollectionId`, `NonEmptyText`, `FullGitRevision`, `RawEvidencePath`) are
newtypes whose existence is the proof. The refusals are the same; what a caller
holds afterwards is not. This is the Stage 6 plan §13.3 rule, modelled on
`quoin_store::RawFileSha256Digest::parse_stored`.

The write path still canonicalises the **caller's** value, exactly as
`store.ts:40` does, so nothing is round-tripped through the typed model on the
way to disk and no unmodelled member can be dropped.

## §2 — Divergences of unification: two retained spellings became one

### §2.1 — one RFC 3339 grammar, and it is the permissive one

The two retained grammars disagree:

| | separator | zulu |
| --- | --- | --- |
| `date-time.ts:1-2` | `[Tt]` | `[Zz]` |
| `intervention.ts:348-350` | `T` | `Z` |

On `2026-01-01t00:00:00z` the first says yes and the second says no. Per the
owner ruling in the Stage 6 plan §5 they unify on the **permissive** one: RFC
3339 §5.6 states the `T` and `Z` characters "may alternatively be lower case",
so the strict regex is a narrowing of the standard rather than an enforcement
of it.

The narrowing is not lost. `Rfc3339DateTime::narrowed_by_the_strict_grammar`
reports, per value, whether `intervention.ts`'s regex would have refused it, and
`tests/tc_468_date_time.rs` asserts the disagreement explicitly rather than
leaving it as a remark. Nothing in the retained corpus uses a lowercase
designator, so the change is unobservable there.

Every other refusal of `date-time.ts` is kept, including the date-only and
missing-seconds forms `Date.parse` would have accepted.

### §2.2 — one assurance-document walk

`plans.ts:29-33,86-96` and `profiles.ts:26-30,62-72` are byte-identical
duplicates on `origin/main`. Here the walk is
`MeasurementSource::assurance_documents` and the frontmatter extraction is
`discovery::frontmatter`; `plans.rs` and `profiles.rs` differ only in the `type`
discriminator they accept and the record they build. Same inputs, same outputs,
one implementation.

### §2.3 — no canonicalizer, digest, JSON parser or YAML reader of its own

Canonical JSON, strict JSON parsing, sha256 file digests and atomic writes are
`quoin_store`'s; frontmatter YAML is `quoin_yaml`'s. `tests/tc_468_boundary.rs`
asserts over this crate's own sources and manifest that no second one appeared
(FR-100-CON-4, Stage 6 plan §13.2), and that exactly one module reads an
instant.

## §3 — Divergences of representation: the same answer, differently shaped

### §3.1 — duplicate observations are detected structurally

`validate.ts:185-191` keys an observation on
`` `${metric}\0${JSON.stringify(sortedDimensionEntries)}` ``. Here `dimensions`
is a `BTreeMap` and `JsonValue` is `PartialEq`, so the duplicate scan compares
the values themselves. Two observations whose dimension *values* differ only in
object member order are duplicates here and distinct there. No retained
collection has dimension values that are objects.

`compare.ts`'s emission order, by contrast, **is** reproduced through a
rendering, because the retained code sorts the rendered keys as strings and the
emitted order is observable. That rendering is
`quoin_store::canonical_bytes` — the store's JCS writer — not a second
serializer.

### §3.2 — string comparisons are in UTF-8 byte order

Every `compare(a, b)` in the retained modules is JavaScript `<`, which orders by
UTF-16 code unit. `str::cmp` orders by UTF-8 byte, i.e. by code point. The two
disagree only for strings containing characters above U+FFFF alongside ones in
U+E000–U+FFFF. No metric, plan id, collection id, timestamp or assurance path
in the corpus contains a character outside ASCII.

### §3.3 — `population` and `dimensions` keep only what is modelled

`validate.ts` checks neither member. The typed parse keeps `examined`,
`matched`, `complete` and `identity` from a `population` object and drops any
other member; `dimensions` is kept whole. A caller that read an unmodelled
`population` member off the oracle's object cannot read it here. Nothing in the
corpus carries one.

### §3.4 — the error envelope is a code plus findings

The retained tree raises two shapes for one domain:
`MeasurementValidationError` carries one prose sentence, and
`InterventionIntakeError` carries a code plus a list of JSON-pointer findings
and builds its sentence from them. This crate keeps the structured half for
both: every refusal carries a `MeasurementErrorCode` and renders its sentence
from the code, the subject and the findings. Message *text* therefore differs in
places; no consumer may match on it, and the codes are the API.

### §3.5 — signatures that take what they need

- `assert_governing_definition` takes the already-loaded plans rather than a
  repository path. The retained `assertGoverningDefinition` re-walks
  `spec/assurance` on every call; the caller here walks once. Same predicate,
  same refusal codes (`governing_plan_absent`, `definition_mismatch`).
- `raw_evidence_for` and `verify_raw_evidence_references` take a
  `MeasurementSource` rather than a repository path, which is what lets the
  filesystem half of `resolveRawPath` stay on the source.

## §4 — A gap, named rather than papered over

`MemoryMeasurement::raw_evidence_file` **refuses** with
`QM-RAW-EVIDENCE-UNAVAILABLE` instead of digesting bytes it holds.
`quoin-store` exposes sha256 only over a path (`digest_file_sha256`);
`sha256_hex` is private and `digest_raw_bytes` is blake3, a different domain.
Adding a `sha2` dependency here to close the gap would be the second sha256 this
port is forbidden to mint, so the gap is reported instead: a public
bytes-wise sha256 is owed by `quoin-store` (already listed as a `quoin-store`
gap in the Stage 6 plan §3). Until it exists, raw-evidence accounting is a
disk-source capability.

## §5 — Module tree: where this crate deviates from the Stage 6 plan §12

Two deviations, both additive:

- **`compare.rs` was added.** The §12 tree omits it although `compare.ts` is in
  the Wave 1 file list. It is 210 lines and holds nothing else.
- **`validate.rs` was split into `validate/mod.rs` (297),
  `validate/stack.rs` (187) and `validate/read.rs` (78).**
  `validateVerificationStack` is a self-contained document of six required
  members, and the member readers are used by both halves. Folding them back
  into one module would put it near the 500-line soft ceiling for no gain.
  This is a **split**, not a deletion: nothing in `validate.ts` is unported.

`tests/tc_468_module_sizes.rs` holds the 700/500 ceilings with an empty
allow-list; no module in this crate is over the soft ceiling.
