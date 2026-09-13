# quoin-store — frozen compatibility surface and the cutover gate

`agent-ix/quoin#380`, under EPIC `#373`.

This crate produces bytes and digests that **already exist on disk**. A digest
here is not a checksum, it is an _identifier_: a change-assurance record is
filed under its own digest and an attestation references its retained output by
digest. A value this crate computes differently from the TypeScript it replaces
is evidence that can no longer be found by the name it was filed under, with no
migration back.

Everything below is therefore frozen: changing one is changing retained
evidence, not changing code.

**What this crate does and does not do.** It is a _reader and verifier_. It
parses stored documents, canonicalizes values, computes digests, replays a
store against the TypeScript oracle, and writes bytes a caller hands it
(`write_atomic`, `write_canonical`, `write_content_addressed`). It does **not**
implement the change-assurance _writers_: nothing in `src/` seals a record,
publishes an attestation pair, or files either under its digest.
`store::record_path` and `store::attestation_path` compute the layout's paths
and are called by no writer in this crate — `grep -rn 'record_path\|attestation_path' src/`
finds only their definitions.

**What is asserted, and by what.** Items 1-11 below are asserted by this
crate's tests (`tests/tc_jcs_adversarial.rs`,
`tests/tc_change_assurance_store.rs`, the unit tests in `src/json/`, and the
oracle case corpus in `oracle/cases.mjs`) and by the
differential replay. Items 12 and 13 are _descriptions of the retained layout
and of the retained TypeScript writer's behaviour_, not assertions about this
crate; each says below exactly how far this crate's tests reach. Where an item
is not asserted, it says so rather than implying otherwise.

---

## The hard gate

**No cutover may depend on this crate until every digest in every reachable
store has been replayed through both implementations with zero mismatches.**

```bash
# TypeScript half — the oracle. Streams newline-delimited JSON.
node --loader ts-node/esm oracle/capture-store-oracle.mjs \
  --out /tmp/store-oracle.ndjson <repo> [<repo> ...]

# Rust half — the comparison. Exit status is the gate.
cargo run --release --bin quoin-store-replay -- \
  --oracle /tmp/store-oracle.ndjson <repo> [<repo> ...]
```

The gate is the _number_, not the tool: a run reports digests replayed and
mismatches, and only `mismatches = 0` clears it. A mismatch is never reconciled
by adjusting the Rust until it agrees — it is reported, and a person decides.

A run reports the population it compared, and **the compared population is part
of the gate**: `gate_passes()` requires a non-zero comparison count equal to the
store entities walked, with no unmatched entry on either side. A run that
compared nothing reports `GATE: FAIL`, never `PASS` (`FR-098-CON-3`,
`FR-098-AC-7`).

### Gate result, 2026-09-12

The full capture is recorded in [`oracle/GATE-RESULT.md`](oracle/GATE-RESULT.md),
with the oracle capture's sha256, the reproduction commands, the verbatim output
of both halves, and the list of stores.

|                                                    |                                                           |
| -------------------------------------------------- | --------------------------------------------------------- |
| stores replayed                                    | **90**                                                    |
| store files                                        | **464**, all parsed by both                               |
| **compared population**                            | **471 of 471** store entities (464 files + 7 raw outputs) |
| oracle entries unmatched / store entries unmatched | **0 / 0**                                                 |
| digests replayed                                   | **10,748,598**                                            |
| **digest mismatches**                              | **0**                                                     |
| sealed records verified against their own digest   | 8                                                         |
| retained outputs verified in the raw-bytes domain  | 7                                                         |
| read → re-serialize byte-identical                 | 413 of 464                                                |
| files Rust refused to read                         | 0                                                         |
| store-integrity findings                           | 0                                                         |
| **gate**                                           | **PASS** (exit status 0)                                  |

Every JSON node of every store file is digested on both sides and compared —
scalars included, because number formatting and string escaping are where the
two implementations were most likely to disagree.

Two results need stating rather than summarising:

- **51 store files are not in canonical form on disk**, and both implementations
  agree that they are not. They were written by producers that do not go through
  `writeCanonical`: some differ by member order only, the rest also by shape —
  raw GitHub API payloads stored compact and verbatim, and measurement records
  containing `0.0`, which the ECMAScript number model prints as `0`. Rewriting
  any of them through the canonical writer would change their bytes. If one is
  ever referenced by a digest over its on-disk bytes, that rewrite breaks the
  reference. (The per-cause split is not recomputed here; the replay reports the
  total and names each divergent file under `--verbose`.)
- **Production change-assurance instances now exist and were replayed.** Seven
  attestation pairs and one sealed record, all under
  `quire-contract-ir/target/assurance/store`, written by the shipped TypeScript
  writers. All 8 sealed records verified against their own digest and all 7
  retained outputs verified in the raw-bytes domain, with zero integrity
  findings. This supersedes an earlier statement in this document that no
  reachable repository held one. The committed fixture at
  `tests/fixtures/change-assurance-store/`, built by
  `oracle/build-change-assurance-fixture.mjs` and replayed by
  `tc_change_assurance_store.rs`, remains the hermetic test of the same shape.

---

## Frozen: the serializations

There are **two** canonical serializations and they are not interchangeable.

|                  | RFC 8785 JCS                                        | `canonicalJson`                  |
| ---------------- | --------------------------------------------------- | -------------------------------- |
| shape            | compact, no whitespace                              | 2-space indent, `": "` separator |
| trailing newline | no                                                  | yes                              |
| member order     | UTF-16 code unit                                    | ECMAScript own-property order    |
| used for         | digest bytes; the `change-assurance` family on disk | every other store file           |

1. **`STORE_SCHEMA_VERSION == 1`.**
2. **JCS member order is UTF-16 code unit order.** Not Unicode scalar order.
   `U+10000` (surrogate pair `D800 DC00`) sorts _before_ `U+FFFD`; Rust's
   `str: Ord` puts it after. A `BTreeMap`-ordered port silently reverses those
   two members and changes the digest.
3. **The pretty form's member order is ECMAScript own-property order.**
   `sortKeys` re-inserts sorted names into a fresh JavaScript object, and
   insertion hoists **array-index** names — the canonical decimal spelling of
   `0..=2^32-2` — ahead of everything else, in ascending numeric order, leaving
   the rest in sorted order. `{"10":_,"2":_,"a":_}` is `2, 10, a` here and
   `10, 2, a` in JCS. `"4294967295"` is _not_ an array index; `"4294967294"` is.
4. **Numbers are ECMAScript `Number::toString`.** `1e+21` with its `+`;
   `100000000000000000000` for `1e20`; `0.000001` for `1e-6` and `1e-7` for
   `1e-7`; `0` for negative zero. Rust's own formatting differs on all four.
5. **Every JSON number is an IEEE-754 double, integers included.**
   `9007199254740993` canonicalizes to `9007199254740992`. The loss is already
   in retained evidence; the port reproduces it rather than fixing it.
6. **No Unicode normalization**, ever. `U+00E9` and `e` + `U+0301` are two
   distinct members.
7. **String escaping is `JSON.stringify`'s**: `"` and `\`, the short forms
   `\b \t \n \f \r`, `\u00xx` with **lowercase** hex for every other C0 control,
   and everything else — `U+007F` and all non-ASCII included — literal.
8. **The strict reader refuses** a byte-order mark, duplicate member names
   (compared after unescaping), unescaped C0 controls in strings, unpaired
   surrogate escapes, numbers that overflow to an infinity, and trailing content.

## Frozen: the digests

9. **Two algorithms, because Quoin has two.** Change-assurance digests are
   **blake3**, stored as 64 lowercase hex characters with **no prefix**.
   Measurement raw evidence and contract pins are **sha256**, stored
   **`sha256:`-prefixed**. In the retained stores those two are told apart only
   by the presence of that prefix. This crate makes the algorithm part of the
   type instead.
10. **A sealed record's digest is taken over the record with its top-level
    `digest` member removed** — and only the top-level one. A nested member
    named `digest` stays in the hashed bytes.

## Frozen: the layout

11. `store_root(repo) == <repo>/spec/evidence`. Used, not asserted: every test
    that builds a store path goes through `store_root`, and the replay finds
    nothing outside it, but no test pins the literal layout string.
12. `change-assurance/records/<digest>.json` — one sealed record, JCS bytes.
    The _bytes on disk_ are asserted: `tc_change_assurance_store.rs` reads the
    fixture's records and checks they are byte-identical to this crate's JCS
    serialization, and the replay above did the same over the production
    instances. What is **not** asserted is that this crate writes them: it has
    no record writer. Its only file-writing canonicalizer, `write_canonical`,
    emits the _pretty_ form, which is the correct form for every other store
    file and the wrong form for this family. `record_path` computes the path and
    nothing in `src/` calls it.
13. `change-assurance/attestations/<digest>/{attestation.json,output.bin}` —
    in the retained TypeScript the pair is made visible by a single directory
    rename, so a half-pair is never observable
    (`src/change-assurance/store.ts`). **This crate does not implement that
    publish step and no test here asserts it.** It reads the pair, verifies the
    attestation against the retained output's raw-bytes digest, and reports a
    mismatch; producing the pair remains the TypeScript writer's job.

---

## Digest domains

Three, as separate types with no `From`, no `Into`, no shared trait yielding a
value another accepts, and no constructor that converts between them. Passing
one where another belongs does not compile.

| type                  | algorithm | digest _of_                                                                          | stored as      |
| --------------------- | --------- | ------------------------------------------------------------------------------------ | -------------- |
| `RawBytesDigest`      | blake3    | a producer's retained output bytes, exactly as supplied                              | bare hex       |
| `CanonicalDigest`     | blake3    | the RFC 8785 canonical bytes of a JSON value                                         | bare hex       |
| `RawFileSha256Digest` | sha256    | a file's complete bytes on disk, in the _measurement raw-evidence_ role specifically | `sha256:<hex>` |

These are this crate's three domains, not the tree's. The retained TypeScript
uses sha256 for at least two further roles that this crate does not model:
assurance-record identity and file names (`src/evidence/assurance-records.ts`)
and schema pinning (`src/quire/contract.ts`). See the module documentation of
`src/digest.rs`.

`quire-protocol`'s `FR-201-canonical-identity-domain` is the rule this follows:
a digest over raw bytes and a digest over canonical bytes are **not
substitutable**, even under the same algorithm and even when, for a given input,
they are the same 64 characters — which for the first two they are, whenever the
raw bytes happen to be canonical bytes. Nothing in the value tells you which
question it answers. That is why the type does.

## Deliberate behaviour changes

Three, each a hardening rather than a format change. None alters any byte or any
digest of an input both implementations accept.

1. **Every write this crate offers is crash-atomic.** Temporary in the
   destination directory, `fsync`, rename, `fsync` the directory. The oracle's
   `writeCanonical` is a plain `writeFileSync`; its change-assurance writer was
   already atomic. `write_content_addressed` publishes with `hard_link` rather
   than `rename`, so a concurrent writer's differing bytes are refused rather
   than clobbered. Asserted by `tests/tc_store_durability.rs`.
2. **`digest_file_sha256` refuses** a symbolic link, a non-regular file, a file
   over `MAX_DIGESTED_FILE_BYTES` (256 MiB), and a file whose length changed
   between the stat and the read. The TypeScript it replaces
   (`rawEvidenceFor`, `src/measurement/intervention.ts:115`) is a bare
   `readFileSync` with none of those guards, so a symlink pointing anywhere is
   digested happily. **This can make an input that previously digested now
   refuse.**
3. **`__proto__` is kept.** See below.

## Known divergence: `__proto__` in the pretty form

`canonicalJson` **silently deletes every `__proto__` member, at any depth, for
any value type.** Its `sortKeys` helper rebuilds each object as a plain `{}` and
assigns members into it, so `out["__proto__"] = v` reaches the
`Object.prototype.__proto__` setter instead of creating an own property.
`JSON.parse` keeps `__proto__` as an own property (it uses `CreateDataProperty`,
not assignment), so `readJson` → `writeCanonical` on a store file containing one
destroys it with no diagnostic. `canonicalizeJcs` is unaffected — it never
assigns into an object — so **no digest is involved**.

This crate keeps the member. Reproducing a prototype-pollution artefact in a
language with no prototypes would be encoding a defect as a contract.

The divergence is declared in `oracle/cases.mjs` and asserted, in both
directions, by `tc_380_proto_member_is_data_in_rust_and_deleted_by_the_typescript_pretty_form`.
The replay reports how many store files carry such a member. **At the
2026-09-12 run, across 464 files in 90 stores: zero.** The divergence is
therefore currently unreachable in practice. If a future run reports a non-zero
count, that is data loss that has already happened and it must be investigated
before cutover, not reconciled here.

## Known divergence: nesting depth

The reader and both writers are recursive and Rust does not grow the stack, so
this crate refuses past `json::MAX_NESTING_DEPTH` (1,000) with
`StoreError::JsonNestingTooDeep` rather than aborting the process on a stack
overflow. The oracle has no stated bound: it refuses by exhausting V8's stack,
at a threshold that varies between runs (measured on node 22.15.0: 5,119-6,143
for `parseStrictJson`, 2,943 for the JCS path, 2,573 for the pretty path).

A fixed number cannot equal a varying one, so the residual band is declared
rather than hidden: **for documents nested deeper than 1,000 and no deeper than
the oracle's own threshold, the oracle accepts and this crate refuses.** That is
the safe direction — this crate never accepts a document the oracle refuses —
and the band is unreachable in retained evidence. The full measurement is in the
documentation of `json::MAX_NESTING_DEPTH`; the refusal is asserted by
`tc_380_nesting_past_the_budget_is_refused_rather_than_overflowing`,
`tc_380_nesting_at_the_budget_is_accepted`, and
`tc_380_both_writers_refuse_past_the_budget_they_share_with_the_parser`.

---

## Regenerating the fixtures

Both capture scripts need a checkout with `node_modules` installed; a bare git
worktree has none. `QUOIN_SRC_ROOT` points them at one.

```bash
QUOIN_SRC_ROOT=/path/to/quoin node --loader ts-node/esm oracle/capture-cases.mjs
QUOIN_SRC_ROOT=/path/to/quoin node --loader ts-node/esm \
  oracle/build-change-assurance-fixture.mjs <dir>
```

The captured fixtures are committed so **no TypeScript runs as a test oracle**
(EPIC #373 AC-5). These scripts exist to regenerate them when the case list
grows, not to run in CI.
