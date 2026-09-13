<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

This crate was ported in two waves and the two halves diverge from the
TypeScript for different reasons. §1–§5 are wave 1 (quoin#468: plans,
collections, validation, the store seam, raw-evidence accounting); §6–§10 are
wave 2 (quoin#469: the intervention and operational record types and the two
report renderers).

# Wave 1 — plans, collections, validation, store, raw evidence (quoin#468)

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

### §3.3 — `population` and `dimensions` keep what is modelled, and the rest verbatim

**Amended by quoin#473.** As first written this section said the typed parse
drops unmodelled `population` members and that "nothing in the corpus carries
one". The second half was false: 25 of the 14,644 observations under
`spec/evidence/measurements/` carry `exclusions` and `namedMisses`, and dropping
them was a real byte divergence on re-serialisation. Wave 6 found it while
building the reporting port, because `renderMeasurementReportJson` writes the
population straight back out. See §13.4.

What stands now: [`types::observation::MeasurementPopulation`] models
`examined`, `matched`, `complete` and `identity` as typed members and keeps
every other member verbatim in an `unmodelled` map that is flattened back on the
way out. `dimensions` is kept whole, and its absence is modelled rather than
flattened to an empty map — also §13.4.

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

# Wave 2 — record types and the two report renderers (quoin#469)

`src/measurement/` is still the oracle for this port and is still the shipping
implementation: quoin#469 deletes nothing. `tests/fixtures/report-oracle.json`
is a frozen capture of what `buildInterventionReport`,
`renderInterventionReport`, `buildOperationalReport` and
`renderOperationalReport` produce at the revision the file names, and every one
of its 15 cases is reproduced here byte for byte — both the projected entries,
compared as JSON, and the rendered markdown.

What follows is everything outside that capture where the two trees can be told
apart. Each entry names the input that separates them.

## §6 — string ordering beyond the Basic Multilingual Plane

Both report builders order records by `(observed_at, record_id)` and order their
inner lists by string comparison. The TypeScript comparator is

```ts
a === b ? 0 : a < b ? -1 : 1
```

and JavaScript's `<` on strings compares **UTF-16 code units**. Rust's `Ord` for
`str` compares **UTF-8 bytes**, which is code-point order.

The two agree on every string whose characters are all in the Basic
Multilingual Plane. They disagree only when a supplementary character
(U+10000 and above, encoded in UTF-16 as a surrogate pair beginning
0xD800..=0xDBFF) is compared against a character in U+E000..=U+FFFF:
JavaScript sorts the supplementary character **first**, because 0xD800 < 0xE000;
this crate sorts it **last**, because its code point is larger.

Separating input: two records whose `record_id`s are `"\u{10000}"` and
`"\u{e000}"`. The TypeScript orders them `"\u{10000}"`, `"\u{e000}"`; this crate
orders them `"\u{e000}"`, `"\u{10000}"`.

Not reconciled. No identity in this domain is written in that range — record ids
are the `p-`/`b-` ASCII namespaces of `intervention.ts:33-45`, metric names and
arm ids are ASCII, and evidence paths are store-relative paths — and the fix
would be a second comparator whose only purpose is to reproduce a UTF-16
artifact. It is recorded here rather than reconciled so that a later wave
reading these lists does not discover it fresh.

## §7 — a JSON integer beyond what a double can hold

`measured_effects[].baseline_value` and `.treatment_value` are held as
`serde_json::Number`, which keeps an integer literal exactly. They **render**
through `js_number_string`, which converts to `f64` first, because that is what
`JSON.parse` does before `String()` ever sees the value — so the rendered text
agrees with the TypeScript even for `12345678901234567890`, which both spell
`12345678901234567000`.

Where the two differ is the record on the way back out. `JSON.parse` has already
rounded, so the TypeScript re-serializes `12345678901234567000`; this crate
re-serializes the literal it was handed, `12345678901234567890`.

Separating input: a record whose `baseline_value` is `12345678901234567890`.

Not reconciled, and deliberately: rounding a stored record on a read-write round
trip is the defect, not the fidelity. `tc_469_reports` asserts the round trip is
exact for every record in the capture.

## §8 — a numeric field written as a float

`size_bytes`, `sample_size`, `repetitions` and `deadline_seconds` are `u64`.
TypeScript's `number` admits `12.0`, and `JSON.parse` cannot tell it from `12`;
this crate refuses it, because serde will not read `12.0` into a `u64`.

Separating input: `"size_bytes": 12.0`.

Not reconciled. The intervention and operational JSON schemas
(`src/measurement/schemas/`) declare these `integer`, so a record carrying
`12.0` is one the schema validation of quoin#470 refuses anyway; this crate
refuses it one step earlier and with a less specific message.

## §9 — `exercise?: never` and `capability?: never` have no counterpart

`operational-types.ts:67,74` keep the two record shapes disjoint by declaring
the other shape's field as `never`. A record carrying both fields is a
TypeScript type error but is structurally representable, and nothing at run time
rejects it — `buildOperationalReport` dispatches on `record_shape` and ignores
the extra field.

`OperationalEvidenceRecord` is an enum tagged on `record_shape`, so the two
payload fields are not fields of one type at all. A record carrying both
deserializes: the tag selects the variant and the other field is ignored,
exactly as the TypeScript ignores it. The difference is that the ill-formed
record is unrepresentable *after* it is read, rather than merely unspellable in
the type checker.

No separating input at the report boundary. Recorded because a reviewer
comparing the two type declarations will find two fields here with no port.

## §10 — what this wave did not port, and is not pretending to

Three things these four files gesture at are deliberately absent rather than
approximated, so that no later wave finds a second implementation to unify:

- **No instant grammar.** `observed_at`, `started_at`, `completed_at` and
  `deadline_at` are `WireInstant`, which holds and does not validate. quoin
  already carries six hand-rolled instant validators across four crates,
  accepting three different languages; this crate gets exactly one, in
  `date_time` (quoin#468), and intake parses through it (quoin#471/#472).
- **No digest parsing.** `Digest` holds the stored spelling.
  `quoin_store::RawFileSha256Digest::parse_stored` is the right type and
  verification is quoin#471/#472; this wave reaches outside the crate for
  nothing.
- **No schema validation.** `intervention-schema.ts` and
  `operational-schema.ts` are quoin#470.

## §11 — the operational port (quoin#472)

Wave 5 ports `src/measurement/operational.ts` and
`src/measurement/github-release-operational.ts`. The producer reproduces the one
retained pair byte for byte (`tests/tc_472_github_release.rs`), so the
divergences below are all in *how* a refusal or a write happens, never in what
is written.

### §11.1 — an unreadable store refuses with a code, not a bare `Error`

`operational.ts:139-146` lets a malformed retained record surface as a raw
`Error` from `JSON.parse`. `read::read_operational_records` maps it to
`InterventionRefusalCode::InvalidRecord` with `"<path>: unreadable operational
record: <detail>"`. The retained text is not machine-readable and this is;
nothing in the retained code branches on the difference.

### §11.2 — the producer's two error shapes

`github-release-operational.ts` throws a bare `Error` for every input-contract
failure and lets `writeOperationalPair`'s `InterventionIntakeError` propagate.
Here those are `GitHubReleaseError::Input` and `GitHubReleaseError::Intake` —
the same two failures, told apart by the type rather than by catching and
inspecting.

### §11.3 — `linked` compares values, not canonical JSON strings

`operational.ts:280-289` decides whether an exercise matches its capability by
canonicalising the subject and the scope and comparing the two strings. Here it
is `==` on `Subject` and `OperationalScope`. Both are exact, and the structural
comparison cannot fail for a reason canonical JSON would invent (a nesting
limit, a number spelling); it also does not allocate two strings per candidate.

### §11.4 — findings are sorted in Rust byte order

`operational.ts:324` sorts the accumulated findings with
`Array.prototype.sort`, which orders by UTF-16 code unit. `validate.rs` uses
`sort_unstable` + `dedup`, which orders by Unicode scalar. The two disagree only
for findings containing an astral character above `U+FFFF` next to one in
`U+E000..U+FFFF`; every finding this code produces is a JSON pointer plus ASCII
prose. Declared rather than reconciled, because reconciling it would mean a
second sort comparator in this crate when `quoin_store::json::order::cmp_utf16`
already exists and is not exported for this.

### §11.5 — schema finding *text* is ajv's message, not quoin's

`quoin_jsonschema` renders `"<instance path or />: <message>"`. The message is
the validator's own and is not byte-identical to ajv's for every keyword. The
*set* of refused records is what is contractual; a message is not, and no
retained record's acceptance depends on one.

### §11.6 — `write_content_addressed` links and fsyncs; the retained code renames

Same strengthening `store.rs` took in quoin#468, and already declared in §4. The
operational pair path goes through it too: the write is refused rather than
silently replaced when the retained bytes differ, which is
`operational.ts:273-278`'s intent stated by the filesystem instead of by an
`existsSync` that has a window in it.

### §11.7 — the third evidence representation is gone

quoin#468 landed `RawEvidenceClaim` (bare `String`s) alongside
`RawEvidenceReference`, because quoin#469's serde-read
`RecordedEvidenceReference` did not exist yet. This wave deleted it:
`verify_raw_evidence_references` takes the recorded reference directly.
`tests/tc_472_evidence_representations.rs` fails if a third type describing a
retained evidence file ever appears. This obligation was also carried on
quoin#471; it was discharged here.

### §11.8 — the write lock's deadline is on an injected clock

`operational.ts:296-318` spins against `Date.now() + 10_000` with
`Atomics.wait(…, 5)`. `operational/lock.rs` takes a `Clock`, and
`source::SystemClock` is what production passes — the same two calls. The
deadline and the retry interval are unchanged; only their source is injectable,
so `tests/tc_472_operational_lock.rs` can state the refusal instead of sleeping
ten seconds for it.

# Wave 4 — intake, the record-id encoding, and the agent-eval producer (quoin#471)

`src/measurement/intervention.ts` and `src/measurement/agent-eval-intervention.ts`,
ported under `src/intervention/`. The retained TypeScript is unchanged and
remains the oracle until the cutover (quoin#479).

The strongest evidence that these divergences are the only ones that matter:
`tc_471_the_producer_reproduces_the_retained_record` runs the ported producer
on the retained definition and the two retained agent-eval reports and gets the
retained record back **byte for byte**. Everything below is a difference that
the one retained production does not exercise.

## §12 — Wave 4 divergences

### §12.1 — the `blake3:` immutable-version alternative is dropped

`agent-eval-intervention.ts:11-12` admits `blake3:<64 hex>` beside `sha256:`,
a git object name and a semantic version. `ImmutableVersion` admits the other
three and refuses that one.

Separating input: `"cli_agent_evals_version": "blake3:<64 hex>"` in a producer
definition — accepted there, refused here.

**Narrowing, and deliberate.** quoin#409 recorded that no quoin producer emits a
BLAKE3 digest for this field, the Stage 6 plan §4 rules it dropped rather than
carried, and nothing in the retained corpus uses it. The crate's
`tc_468_boundary` census reads the lower-case token as a second hasher, so
`version.rs` spells the algorithm in upper case and its test builds the prefix
by concatenation. Held as an assertion —
`the_dropped_digest_alternative_is_gone` — rather than only as this paragraph.

### §12.2 — `observed_at` is validated by one RFC 3339 grammar, not two

`intervention.ts:24-26` registers `isRfc3339DateTime` as ajv's `date-time`
format and `semanticFindings` calls the same function again. Both here go
through `crate::date_time::Rfc3339DateTime`, the crate's one grammar, which
§2.1 already records as the permissive one of quoin's six. So a record whose
`observed_at` is accepted by the permissive grammar and refused by a stricter
one is accepted here and in the retained code alike; what changed is that there
is no second grammar to disagree with.

### §12.3 — schema finding text is ajv's there and `jsonschema`'s here

`intervention.ts:51-56` renders `` `${instancePath || "/"}: ${message}` ``. The
instance path and the verdict agree; the sentence after the colon is each
library's own. quoin#470's `DIVERGENCE.md` records this for the vendored
schemas generally and this wave inherits it — a caller matching on refusal text
rather than on `InterventionRefusalCode` is relying on something neither
implementation promises.

### §12.4 — scenario ids are ordered by UTF-8 bytes

`reportFrom` builds a `Map` and the producer sorts its keys with the default
comparator, which is UTF-16 code-unit order. `AgentEvalReport` holds a
`BTreeMap`, which is UTF-8 byte order. The two orders differ only for scenario
ids containing characters outside the Basic Multilingual Plane — the same
divergence §6 records for record ordering, with the same separating input and
the same conclusion: not reconciled, and no retained report contains one.

### §12.5 — a report with no `results` array gets a sentence

`agent-eval-intervention.ts:159` reaches `(report.results as unknown[]).entries()`
with no check, so a report lacking `results` throws a bare `TypeError` whose
message is V8's. `AgentEvalReport::parse` refuses with
`"{label} agent-eval report lacks a results array"` under
`InterventionRefusalCode::InvalidRecord`.

Unreachable through the producer: `quoin_evidence::adapters::parse_agent_eval`
is the FR-042 boundary and runs first, and it already refuses a report with no
`results` array. Reachable only by calling `AgentEvalReport::parse` directly,
which the producer is the only caller of.

### §12.6 — what `digest_file_sha256` refuses that `readFileSync` does not

`intervention.ts:120` digests whatever `readFileSync` returns.
`quoin_store::digest_file_sha256` (`quoin-store/src/digest.rs:470-514`) stats
first and refuses three things the retained reader accepts:

- a file larger than `MAX_DIGESTED_FILE_BYTES` (256 MiB),
- a path that is not a regular file — a FIFO, a device, a socket,
- a file that shrinks between the stat and the read (a short read).

**The symlink case, honestly:** quoin#407 is filed as a symlink divergence, and
`digest_file_sha256` does refuse a symlink. On *this* path it never sees one.
`DiskMeasurement::resolve_raw_evidence` canonicalises before digesting — it has
to, because that canonicalisation is how `resolveRawPath`'s escape check is
performed — so a symlink that stays inside the evidence store is followed by
both implementations and digests to the same bytes, and one that leaves the
store is refused earlier and for a different reason. Both halves are asserted in
`tc_471_raw_evidence_resolution_refuses_every_escape` on a real tree with real
symlinks. The reachable part of quoin#407 here is the 256 MiB ceiling and the
non-regular-file refusal; the symlink refusal is unreachable through raw-evidence
resolution.

Not reconciled. All three refusals are strengthenings on evidence quoin retains
and digests, and no retained evidence file is affected.

### §12.7 — the retained record's file name predates the `p-`/`b-` namespacing

Not a port divergence: the two implementations agree, and the store does not.

`intervention.ts:39-45` names a record's file `p-<record_id>.json` or
`b-<base64url>.json`. The one retained record is at
`spec/evidence/interventions/quoin-270-cli-eval-sentinel-contract.json`, with no
prefix — it was published before that encoding landed. Reads are unaffected
(both implementations list the directory), but a re-publication of that record
by either implementation writes a second file beside it rather than over it.

Filed as **quoin#486** against the retained store, not absorbed here.
`tc_471_the_producer_reproduces_the_retained_record` asserts the produced path
is the `p-` one *and* that the unprefixed name is no longer what the writer
chooses, so renaming the retained file fails this test and points at this
paragraph.

## §13 — Wave 6 divergences: the reporting port (quoin#473)

`report.ts` and `portfolio.ts`, ported into [`report`] and [`portfolio`]. All
six gated functions — `renderMeasurementReport`, `renderMeasurementReportJson`,
`renderPortfolioReport`, `renderPortfolioReportJson`, `comparisonFor` and
`seriesFor` — are byte-identical to the retained TypeScript on the committed
fixture tree (`tc_473_reporting`, FR-100-AC-4). The divergences below are
therefore either outside those bytes, or reachable only on inputs the fixture
states cannot occur.

### §13.1 — the fixture compares with one path substitution

`root` and `path` are absolute in both the rendered report and its JSON, so the
captured bytes would otherwise record whoever's checkout produced them. The
capture script and `tc_473_reporting` both replace the fixture tree's own
absolute root with the token `@@TREE@@` before comparing, and nothing else is
edited. Neither side canonicalises: node's `path.resolve` absolutises and
normalises lexically without following symlinks, and
[`portfolio::location::resolve`] does the same, so a checkout reached through a
symlink produces the same string on both sides.

This is a property of the comparison, not of the port. Everything to the right
of the substituted prefix — separators, ordering, the `n/a` fallbacks — is
compared as written.

### §13.2 — a caught reader error renders Rust's message, not node's

`portfolio.ts:99-104` catches whatever the store read threw and puts
`error.message` into the repository's `status`. Three of those messages are
constructed by quoin itself and are reproduced verbatim here:

- `<root>: repository does not exist`
- `<root>: repository location is not a directory`
- `<path>: collection timestamp is not a valid date`

The fourth class is not: a message raised by node's `fs` or by `JSON.parse` —
`ENOENT: no such file or directory, open '…'`, `Unexpected token } in JSON at
position 41` — becomes this crate's [`MeasurementError`] text instead. The
answer (that repository is unreadable, and the walk continues) is the same; the
sentence is not.

The fixture tree exercises all three constructed messages and no member of the
fourth class, which is why byte-identity holds. **A portfolio run over a
repository with a corrupt or unreadable collection file diverges in that one
string.** Not reconciled: reproducing node's `fs` and `JSON.parse` prose would
mean a second error vocabulary inside this crate, which §2.3 and the Stage 6
plan §5 both forbid.

### §13.3 — staleness reads instants with the crate's one grammar

`portfolio.ts` compares collection timestamps with `Date.parse`, which accepts
far more than RFC 3339 (`"March 1, 2026"`, `"2026"`, a bare
`"2026-03-01T00:00:00"` read as local time). [`portfolio::build::epoch_millis`]
uses [`date_time::Rfc3339DateTime`], the single instant reader this crate is
permitted (§2.1, Stage 6 plan §5).

Where the two disagree, this crate refuses: a collection whose `timestamp`
`Date.parse` would have accepted and `Rfc3339DateTime` will not makes that
repository `unreadable — <path>: collection timestamp is not a valid date`,
which is the same sentence `portfolio.ts:112` writes for a timestamp `Date.parse`
rejects outright. The fixture's `golf` repository carries
`"timestamp": "the-first-of-never"`, rejected by both, so the refusal path is
under the byte comparison; the widening is not, because no such collection
exists in the corpus — every `timestamp` under `spec/evidence/measurements/`
parses as RFC 3339.

### §13.4 — observation member fidelity, and the `Dimensions` newtype

Two defects wave 6 found in the wave-1 type model, both fixed here rather than
absorbed:

1. **Unmodelled `population` members were dropped.** 25 of 14,644 corpus
   observations carry `exclusions` and `namedMisses`.
   [`types::observation::MeasurementPopulation`] now keeps every unmodelled
   member verbatim and flattens it back on serialisation. §3.3 is amended.
2. **An absent `dimensions` was flattened to an empty map.** 305 of 14,644
   observations state no `dimensions` at all, and `{}` and absent are different
   bytes in the report JSON. [`types::observation::Dimensions`] now models the
   distinction: `ABSENT` serialises as absent, `stated(entries)` as the object.

A census over all 14,644 retained observations settles what is left: there is no
unmodelled observation-level or collection-level member, `dimensions` is never a
stated `{}`, `population` is never a non-object, and all four modelled
population members always carry their modelled type. Those are facts about the
corpus at this revision, not guarantees; a future observation that breaks one
would be refused by [`validate`] rather than silently reshaped.

### §13.5 — `dimensions` are ordered, not in insertion order

`dimensions` is a `BTreeMap<String, JsonValue>` here and a plain object in the
TypeScript, so it re-serialises in sorted-name order rather than insertion
order. This is invisible in the canonical JSON, which sorts names anyway, and it
is invisible in the rendered `[language=rust, tier=2]` suffix for the same
reason — `report.ts:106` sorts the entries before joining them.

It is visible nowhere in the corpus: all 23,214 `dimensions` and `population`
objects under `spec/evidence/measurements/` are already stored in sorted-name
order, because every one of them was written through `canonicalJson`.
