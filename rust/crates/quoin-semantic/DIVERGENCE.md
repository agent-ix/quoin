# ajv ↔ Rust `jsonschema`: measured divergence

**Status:** measured, 2026-09-12. **Scope:** every JSON Schema quoin compiles in
`src/semantic/` and `src/completeness/`. **Issue:** agent-ix/quoin#378, EPIC #373
Stage 3.

EPIC #373 names this as a known risk: _"ajv ↔ `jsonschema` crate parity — error
shape and order are user-visible (`semantic/manifest.ts` `mapAjvError`;
`quire/validate.ts` orders lines as ajv reported them), and ecosystem pins
disagree."_ This document is the measurement. It exists so the next stage — which
carries the measurement domain, where a verdict flip is unrecoverable — starts
from evidence rather than from a hope.

Everything below was produced by running both libraries over the same 171
documents and comparing. Nothing here is inferred from documentation.

---

## 0. The headline

| Question                                                               | Answer                                                                                                                            |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Do ajv and `jsonschema` ever disagree on a **verdict**?                | **No.** 171/171 documents agree, on both versions tested.                                                                         |
| Do they agree on the **set** of `(instance location, keyword)` errors? | Not natively — 4/118 invalid documents differ. The adapter closes all 4.                                                          |
| Do they agree on error **order**?                                      | 110/118 identical; 4 differ. Order is **not** stable even between two `jsonschema` releases, so it is not treated as contractual. |
| Do they agree on error **text**?                                       | No, and it is not contractual.                                                                                                    |

**The pin:** `jsonschema = "=0.56.0"`, `default-features = false`.

---

## 1. The version decision

### What the ecosystem actually pins

| Repository               | Pin                                                                                 | File                                       |
| ------------------------ | ----------------------------------------------------------------------------------- | ------------------------------------------ |
| `quire-rs`               | `~0.18` (resolves 0.18.3), `default-features = false`, `features = ["draft202012"]` | `Cargo.toml:43`                            |
| `quire-contract-ir`      | `=0.17.1`, `default-features = false`                                               | `Cargo.toml:17`                            |
| `quire-spec-language`    | `=0.17.1`, `features = ["draft202012"]`                                             | `Cargo.toml:28`                            |
| `quire-contract-codegen` | `=0.17.1`, `features = ["draft202012"]`                                             | `Cargo.toml:24`                            |
| `quire-analyze`          | `=0.17.1`                                                                           | `Cargo.toml:17`                            |
| `filament-core-data`     | none — declares explicitly that it pins no `jsonschema`                             | `crates/extraction-frontend/Cargo.toml:11` |

So the split is not two-way. It is **0.18.3 in one repository and 0.17.1 in four**,
with the current release at 0.56.0.

### What quoin picks, and why

**`jsonschema = "=0.56.0"`, `default-features = false`.**

1. **Verdict parity is contractual, and 0.18.3 is measurably non-conformant on
   2020-12.** Both versions pass all 171 corpus documents, so the corpus alone
   does not separate them. A targeted feature probe does: against
   `{"$defs": {"item": {"$dynamicAnchor": "T", "type": "string"}}, "items": {"$dynamicRef": "#T"}}`,
   the document `[1]` is **accepted by 0.18.3** and rejected by 0.56.0, by ajv,
   and by the 2020-12 specification. Neither vendored schema uses `$dynamicRef`
   today — but `module-manifest.schema.json` is owned by filament-core-service and
   `package-manifest.schema.json`/`common.schema.json` by filament-core-data, so
   quoin does not control when one appears, and the failure mode is a manifest
   silently accepted rather than an error.

   Everything else probed agrees on all three: lookahead patterns (both
   `jsonschema` versions use `fancy-regex`, not the `regex` crate, so
   `common.schema.json`'s `sourceLocus.path` compiles),
   `unevaluatedProperties`, `prefixItems` + `items: false`, `dependentRequired`,
   integer-valued floats (`10.0` satisfies `type: integer`), and the
   `oneOf` + `not` + `required` combination `ObjectTypeEntry.data_schema` uses.

2. **Nothing links quoin's crates to quire-rs or quire-contract-ir.** EPIC #373
   fixes the boundary as _"subprocess + JSON over stdin/stdout to a single
   `quoin-core` binary. Not napi-rs, not WASM."_ There is therefore no Cargo
   feature-unification or type-compatibility constraint pulling quoin onto 0.17
   or 0.18; matching them would buy consistency of appearance and nothing else.

3. **`default-features = false` removes the network.** 0.56's defaults are
   `resolve-http`, `resolve-file`, `tls-aws-lc-rs`, `idna`. Turning them off drops
   `reqwest` and `rustls` from the graph entirely and makes "no network read on a
   command path" a property of the dependency rather than of a code review. In
   0.18 the equivalent is a hand-written `SchemaResolver` — and its _default_
   resolver reads files and HTTP — so the safe configuration is the one you
   remember to write.

4. **External schemas are a first-class API in 0.56.** `package-manifest.schema.json`
   references `common.schema.json#/$defs/…` relatively, which ajv resolves via
   `addSchema`. 0.56 has `Registry::new().add(uri, doc).prepare()`. 0.18 has no
   equivalent: it requires implementing the `SchemaResolver` trait, and the first
   run of the comparison harness — written without one — rejected **every valid
   package manifest** with a bare resolver error. That was a harness bug, not a
   library defect, and it is recorded here precisely because it is the shape of
   mistake the 0.18 API invites.

5. **Exact `=` rather than `~` or `^`.** The adapter in `src/schema.rs` matches
   `ValidationErrorKind` **exhaustively, with no `_` arm**. A variant added by a
   minor bump therefore breaks the build and has to be classified deliberately.
   An `=` pin makes that a reviewed change rather than one that arrives with
   `cargo update`. This matches the `=0.17.1` habit in four of the five repositories
   above; only quire-rs uses a range.

### Recommendation to the ecosystem (not actioned by this ticket)

Four repositories pin a release two majors behind one with a demonstrated
2020-12 conformance defect. That is a separate ticket in each repository, and
`quire-rs`'s `~0.18` is the one that matters most because it is the engine. This
port does not touch them.

---

## 2. The corpus

Captured once from the TypeScript by `scripts/capture-semantic-goldens.mjs` at
**quoin@4d27dcf1621d8c28da0961a5521a5be7b6d1cd28**, ajv **8.20.0**.

| Schema                                                                | Documents |  Valid | Invalid | Call site                                                      |
| --------------------------------------------------------------------- | --------: | -----: | ------: | -------------------------------------------------------------- |
| `semantic` block (`module-manifest.schema.json#/properties/semantic`) |        65 |     24 |      41 | `src/semantic/manifest.ts` `semanticBlockValidator()`          |
| `sweep-report.schema.json`                                            |        47 |     15 |      32 | `src/semantic/manifest.ts` `sweepReportValidator()`            |
| `package-manifest.schema.json` + `common.schema.json`                 |        59 |     14 |      45 | `src/semantic/package-manifest.ts` `validatePackageManifest()` |
| **Total**                                                             |   **171** | **53** | **118** |                                                                |

Each corpus was built to exercise the keywords the schema actually uses, not a
uniform sample: every `targets` enum member, every `compatibility_posture` and
`legacy_forms` value, every required key removed individually, every `$ref`ed
`common.schema.json` definition violated individually, and a `many-faults`
document per schema that fails on eight to eleven keywords at once so that
ordering and multiplicity are observable rather than degenerate.

ajv options are reproduced exactly per call site, including `verbose: true` and
`useDefaults: false` for the `semantic` block — `mapAjvError`'s `enum` branch
reads `error.data`, which only exists under `verbose`.

---

## 3. Divergence D1 — `additionalProperties` multiplicity

**Shape.** ajv, under `allErrors: true`, emits **one error per unexpected
property**, each carrying `params.additionalProperty`. `jsonschema` emits **one
error carrying `unexpected: Vec<String>`**.

**Measured.** 1 of 118 invalid documents (`semantic-block/two-unknown-keys`):
ajv 2 errors at `""`/`additionalProperties`, `jsonschema` 1.

**User-visible consequence if absorbed.** `mapAjvError` turns each ajv error into
its own `semantic.unknown-key` diagnostic naming the key. Absorbing the
divergence would print one diagnostic for a manifest with two typos — so a user
fixes the first key, re-runs, and is told about the second. On a manifest with
five stray keys that is five round trips.

**What the port does.** `schema.rs` `push_error` fans the `unexpected` vector out
into one `SchemaError` per key. `UnevaluatedProperties` is fanned out the same
way, pre-emptively: nothing in the current schemas reaches it, but
`Entity.json`-style `unevaluatedProperties` already appears in shipped module
bundles.

**Non-divergence, checked:** `required` behaves the _same_ in both — one error per
missing property. `sweep-report/many-faults` produces three `/counts required`
errors in both libraries.

---

## 4. Divergence D2 — `anyOf` / `oneOf` sub-error visibility

**Shape.** ajv reports the composite failure **and** each failing branch's errors
as siblings. `jsonschema` reports only the composite, nesting the branch errors
inside `ValidationErrorKind::AnyOf { context }`.

**Measured.** 3 of 118 invalid documents, all in the package-manifest corpus,
all through `common.schema.json#/$defs/manifestTarget`
(`anyOf: [target, representationFormat]`):
`profile-unknown-target`, `targets-unknown`, `many-faults`. ajv emits two `enum`
errors plus one `anyOf`; `jsonschema` natively emits one `anyOf`.

**User-visible consequence if absorbed.** `targets: ["cobol"]` currently reads as
_"must be equal to one of the allowed values"_ with the allowed values attached.
Absorbed, it becomes _"is not valid under any of the schemas listed in the anyOf
keyword"_ — the list of legal targets disappears from the message, and quoin's
own `semantic.unknown-target` code (which `mapAjvError` selects on
`keyword === "enum"` at a `targets` locus) would never be selected. The user
would get `semantic.invalid-value` instead.

**What the port does.** `push_error` recurses into `context` for `AnyOf`,
`OneOfNotValid`, `OneOfMultipleValid` and `PropertyNames`, emitting the composite
first and then every nested error. After that the identity multisets match
exactly on all 3 documents.

**Caveat.** This is the divergence most likely to reappear in a later stage.
`jsonschema` composes `context` per failing branch; ajv's traversal is not
guaranteed to visit the same branches on a schema with nested composites. The
current schemas have one level of `anyOf` over two `$ref`s. A deeper composite
should be re-measured, not assumed.

---

## 5. Divergence D3 — error ordering

**Shape.** ajv reports in **schema-declaration order**: the keyword applied at a
level before the errors of its children, properties in the order the schema
declares them. `jsonschema` reports in its own traversal order, which is roughly
instance-location order with parent-level keywords last.

**Measured, `jsonschema` 0.56.0 vs ajv:**

| Corpus           | Invalid | Byte-identical order | Differs in order only |
| ---------------- | ------: | -------------------: | --------------------: |
| `semantic` block |      41 |                   38 |                     2 |
| sweep report     |      32 |                   30 |                     2 |
| package manifest |      45 |                   42 |                     0 |
| **Total**        | **118** |              **110** |                 **4** |

Concretely, `semantic-block/many-faults`:

- ajv: `additionalProperties`, `/contract_version pattern`, `/semantic_core pattern`, `/package pattern`, `/exports/0 minLength`, `/exports/1 minLength`, `/exports uniqueItems`, `/targets/0 enum`, `/compatibility_posture enum`, `/legacy_forms enum`
- `jsonschema`: `/compatibility_posture enum`, `/contract_version pattern`, `/exports uniqueItems`, `/exports/0 minLength`, `/exports/1 minLength`, `/legacy_forms enum`, `/package pattern`, `/semantic_core pattern`, `/targets/0 enum`, `additionalProperties`

**Order is not even stable across `jsonschema` releases.** On
`semantic-block/exports-two-faults`, 0.18.3 emits `/exports uniqueItems` _after_
the two `/exports/N minLength` errors and 0.56.0 emits it _before_. A port that
pinned its user-visible output to one library's traversal would have that output
change under a patch bump.

**User-visible consequence.** Two places surface ajv order:

1. `src/semantic/manifest.ts` pushes `mapAjvError` output in ajv order, and
   `formatDiagnostics` joins it with newlines. The diagnostic **lines** change
   order; their content does not.
2. `src/quire/validate.ts` — **out of this ticket's scope** (it is `src/quire/`,
   EPIC #373 Stage 1) — documents `errors: string[]` as _"one line per failing
   path, in the order ajv reported them."_ That comment is a promise the Stage 1
   port cannot keep. **Flagged for #375/Stage 1: the comment should be changed to
   describe the order actually produced, before the port, not after.**

**What the port does.** `SchemaValidator::errors` sorts by
`(instance location, keyword)` and says so in its doc comment. That is stable
across dependency bumps and across libraries, which neither native order is. The
golden tests compare identity **multisets** and deliberately do not compare
order.

`sweepReportProblem` prints ajv's _first_ error; the Rust prints the first in the
sorted order. For a report failing on one keyword — every real case — that is the
same error. For a report failing on several it may name a different one, and the
message is advisory in both.

---

## 6. D4 — YAML scalar resolution (**closed**, was a verdict-level defect)

Found while porting. Recorded here because it is the same class of risk and
nothing else would have caught it. **It is no longer a divergence: the port now
reads YAML 1.2.** What follows is what was actually measured, because the
earlier version of this section was wrong on both halves.

### What was wrong before

This section used to claim the blast radius was "measured as zero" because "the
schema refuses a non-string before any of these could matter". Both halves were
false:

- **There is no schema.** `quoin-completeness`'s frontmatter reader
  (`src/bundle.rs`) parses a document's frontmatter and projects vocabulary
  fields with no schema validation at any point. Nothing refuses a non-string;
  `strings_at` silently drops one. The TypeScript's `stringsAt` drops it too, so
  the drop itself is parity-correct — the divergence is one step earlier, in
  which scalars become strings in the first place.
- **The zero was measured over an empty population.** The golden corpus
  contained no ambiguous token, so the check had nothing to find.

The old table was also factually wrong about the reader it described.
`serde_norway 0.9.42` resolves `no`, `on`, `yes`, `y`, `12:30`, `1:30` and
`2026-09-12` as **strings**, exactly like the oracle. Those rows named
divergences that did not exist.

### What was actually divergent

Measured token by token, `yaml` npm (the oracle) against `serde_norway 0.9.42`:

| Source  | `yaml` npm (YAML 1.2 core) | `serde_norway` (YAML 1.1) | Effect on a string-typed field  |
| ------- | -------------------------- | ------------------------- | ------------------------------- |
| `017`   | integer `17`               | string `"017"`            | oracle drops it, Rust kept it   |
| `010`   | integer `10`               | string `"010"`            | oracle drops it, Rust kept it   |
| `0b101` | string `"0b101"`           | integer `5`               | oracle kept it, Rust dropped it |

(`on`/`off`/`yes`/`no`/`y`/`n`, `null`/`~`, sexagesimals and dates agree between
the two. `0o17`, `0x1F`, `1.10`, `+1` and `1_000` also agree.)

That is a **verdict-level** difference, not a diagnostic-text one. Reproduced on
a bundle whose `quality_attributes_not_applicable` excuses `017`: the oracle
resolves `017` to a number, drops it, finds nothing, and returns `PASS`; the
libyaml reader kept `"017"`, raised a high-severity `undeclared-exclusion`, and
returned `FAIL`. Verbatim, before the fix:

```
---- tc_378_304_assess_bundle_matches_the_oracle_end_to_end stdout ----
assertion `left == right` failed: advisory: verdict
  left: "PASS"
 right: "FAIL"
```

and on the projection itself:

```
---- tc_378_303_bundle_frontmatter_matches_the_oracle stdout ----
assertion `left == right` failed: projected claims
  left: [..., ("ambiguous/claims.md", ["security", "on", "no", "1:30", "2026-09-12", "0b101"], []), ...]
 right: [..., ("ambiguous/claims.md", ["security", "on", "no", "1:30", "2026-09-12", "017", "010"], []),
         ("ambiguous/excuse.md", [], ["017"]), ...]
```

### How it was closed

Fix shape (i), a YAML 1.2 core-schema reader, rather than (ii), coercing scalars
back to their source text. (ii) would have matched the oracle only for fields
that are strings on both sides; a manifest field the oracle genuinely reads as a
number (`version: 1.10`, a numeric threshold) would then be a string here, which
moves the divergence rather than closing it. (i) makes the two readers agree on
every scalar, not just the string-typed ones.

`crates/quoin-yaml` is now the single YAML reader on the boundary. It wraps
`saphyr 0.0.12` — a native YAML 1.2 parser, not a libyaml binding — and yields
`serde_json::Value`. All three call sites read through it:
`quoin-semantic::read_manifest_yaml`, `quoin-completeness`'s declaration loader,
and `quoin-completeness`'s frontmatter reader. `serde_norway` is gone from the
workspace. Adding a second YAML reader re-opens D4.

Corpus: `scripts/capture-semantic-goldens.mjs` now writes `ambiguous/claims.md`
and `ambiguous/excuse.md` into the bundle corpus, carrying `on`, `no`, `null`,
`1:30`, `2026-09-12`, `017`, `010` and `0b101` as module and keyword values.
`tc_378_303` asserts, over the goldens, that the oracle keeps the first five and
drops `017`/`010` — so the population cannot be silently emptied again — and
`tc_378_304` asserts the resulting verdict.

### What is still divergent, and why it is accepted

- **Parse-error text.** A malformed frontmatter block produces `saphyr`'s message
  (`while parsing a flow sequence, expected ',' or ']'`) where the oracle
  produces the `yaml` package's. The set of documents reported unreadable is
  identical and asserted; only the human-readable reason differs. Consequence:
  a user reading the "why" of an unreadable document sees different wording in
  the Rust CLI than in the TypeScript one.
- **`.inf` / `.nan`.** `serde_json` cannot hold either, so both become `null`
  here. The oracle holds them as JavaScript `Infinity`/`NaN`, which its own
  `JSON.stringify` also writes as `null`. Consequence: none at the boundary; a
  difference exists only in-memory, on the TypeScript side, and nothing reads it.
- **Duplicate mapping keys.** Last wins here; the `yaml` package warns and also
  takes the last. Consequence: none, but the warning is not reproduced.

## 7. Divergence D5 — map iteration order

Not a schema question either, but the same "invisible until it is a number"
class.

- `resolveImports` walks `Object.keys(module.block.imports)` — **insertion
  order**, i.e. manifest order. The port uses a `BTreeMap`, i.e. **sorted** order.
  The _set_ of cycles found is identical (the DFS is complete either way, and the
  golden corpus's 9 import cases all match). What can differ is which cycle is
  printed first, and the trail a cycle prints, for a module with two imports that
  both close cycles. Sorted was chosen because manifest order is not reproducible
  across a manifest rewrite.
- `derivePackageManifest` sorts imports with `localeCompare`; the port relies on
  `BTreeMap` byte order. These agree for every value the schema admits
  (`^[a-z0-9][a-z0-9._-]*/[a-z0-9][a-z0-9._-]*$` is ASCII), and the 7 golden
  derivation cases compare the full derived document byte-for-byte.
- `completeness` sorts findings with three `localeCompare` calls; the port uses
  byte order, for the same reason and with the same ASCII guarantee.

---

## 8. What is _not_ divergent

Recorded because a parity claim over an unexercised feature is worth nothing, and
these were all measured rather than assumed:

- **Pattern semantics, including lookaheads.** `common.schema.json`'s
  `sourceLocus.path` uses four negative lookaheads. The Rust `regex` crate cannot
  compile those, but `jsonschema` uses `fancy-regex` (0.13 in 0.18, 0.19 in 0.56),
  which can. All four probe cases agree with ajv.
- **`unevaluatedProperties`** — the vendored `Entity.json` fixture uses
  `{"unevaluatedProperties": {"not": {}}}`; both agree.
- **Integer-valued floats.** `10.0` satisfies `type: integer` in ajv, in both
  `jsonschema` versions, and in the specification. (`10.5` and `-1` are refused by
  all.) This is the divergence a port _expects_ to find between a
  JavaScript number model and `serde_json`'s, and it is not there.
- **`const`, `uniqueItems`, `minItems`, `minLength`, `oneOf` + `not` + `required`,
  relative `$ref` resolution against `$id`** — all exercised by the corpus, all
  agreeing.
- **`required` multiplicity** — one error per missing key in both (unlike D1).

---

## 9. How to re-measure

```bash
# 1. Rebuild the oracle and recapture the goldens (only when the TypeScript changes).
pnpm install --frozen-lockfile
node_modules/.bin/tsc -p tsconfig.json --outDir .oracle \
  --declaration false --declarationMap false
cp -r src/semantic/schemas .oracle/semantic/schemas
cp src/semantic/sweep-report.schema.json .oracle/semantic/
node scripts/capture-semantic-goldens.mjs
rm -rf .oracle

# 2. Re-run parity.
# Both commands run from the repository root. The workspace root is
# rust/Cargo.toml, so --manifest-path is required; without it cargo walks up
# from the repository root and finds no manifest at all.
cargo +1.98.1 test --manifest-path rust/Cargo.toml \
  -p quoin-semantic -p quoin-completeness -p quoin-yaml --target-dir <dir>
```

A verdict disagreement fails `tc_378_100`/`101`/`102` with the document id. **Do
not allowlist one.** A verdict disagreement is either a TypeScript defect or a
specification gap, and both matter more than the port does.

---

## 10. `serde_json/preserve_order` — a workspace-level hazard, not a crate choice

This crate compares derived package manifests byte-for-byte against the
oracle's output, and the oracle emits object keys in **insertion** order. The
obvious reach is `serde_json`'s `preserve_order` feature, which swaps
`serde_json::Map` from `BTreeMap` to `IndexMap`. This branch originally enabled
it. **It has been removed, and it must not come back.**

Cargo features are unified per-build across the whole workspace. Enabling
`preserve_order` in _any_ member silently turns off key sorting for _every_
member, including `quoin-core::protocol::canonical_json`, whose whole job is to
sort. Measured, not assumed — with `preserve_order` on, and nothing else
changed:

- `quoin-core` loses `protocol::tests::canonical_json_sorts_keys_and_emits_no_whitespace`
  and `tc_375_stdout_is_canonical_json_one_line`.
- `quoin-semantic` (15 tests) and `quoin-completeness` (6 golden-parity tests)
  pass either way — the byte-parity goldens this crate owns do not in fact need
  the feature.

**User-visible consequence if it is ever enabled:** the CLI's canonical JSON
stops being canonical. Two runs over the same data can emit different byte
sequences, so anything digesting that output — an evidence record, a cache key
— silently changes identity. The feature is a global, load-bearing switch
disguised as a per-crate convenience; any crate that wants insertion order must
get it from an explicitly ordered type, not from this feature. Raised on
quoin#375 as a workspace-level concern.
