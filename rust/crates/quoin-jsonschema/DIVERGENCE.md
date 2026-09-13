<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

This crate carries two things: the ajv-parity validator extracted from
`quoin-semantic` (quoin#470), and the two measurement schema documents ported
from `src/measurement/intervention-schema.ts` and
`src/measurement/operational-schema.ts`.

The retained TypeScript is still present and is still the oracle. Nothing in it
was edited by this wave — a port declares its divergences, it does not retrofit
the implementation it is replacing.

Two sections below are **owner rulings**: deliberate, recorded changes in what
is accepted. The rest are shape differences between `ajv` and the Rust
`jsonschema` crate that do not move a verdict.

---

## §1 — Ruling: RFC 3339 accepts lowercase `t` and `z` (quoin#440)

**Input that separates them:** `"2026-01-01t00:00:00z"` as `observed_at`.

The retained tree has two RFC 3339 grammars and they disagree:

| implementation | separator | zulu | used by |
|---|---|---|---|
| `src/measurement/date-time.ts:2` | `[Tt]` | `[Zz]` | the operational schema's `format: "date-time"` |
| `src/measurement/intervention.ts:349` | `T` | `Z` | the intervention schema's `format: "date-time"` |
| `src/measurement/intervention-schema.ts:14-16` | `T` | `Z` | the intervention schema's `observed_at` **pattern** |

So that value is a valid `observed_at` in an operational record and an invalid
one in an intervention record, today, on `main`.

**Ruling: the permissive grammar wins, and the pattern moves with it.**

1. RFC 3339 §5.6 explicitly permits lowercase `t` and `z`. The strict grammar is
   narrower than the standard it is named after; unifying on it would ship a
   validator that refuses valid RFC 3339.
2. Widening is the only direction that cannot invalidate a record that
   validates today. Narrowing the operational side could invalidate records in
   stores this repository cannot enumerate. That asymmetry decides it
   independently of the RFC.
3. It is already the behaviour of eight of the nine fields carrying
   `format: "date-time"` in the operational schema.

Leaving the uppercase-only `pattern` in the vendored document would have given
one function and two behaviours — the same divergence relocated from a function
to a regex. So the vendored intervention schema's `observed_at.pattern` reads

```
^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt][0-9]{2}:[0-9]{2}:[0-9]{2}(?:[.][0-9]+)?(?:[Zz]|[+-][0-9]{2}:[0-9]{2})$
```

which expresses the same language `quoin_measurement::date_time::Rfc3339DateTime::parse`
accepts. `tests/tc_470_formats.rs` asserts the pattern and the registered check
accept the same set, including the `…t…z` case that separated them, so the two
expressions cannot drift apart again.

**Measured:** a scan of every `.json` under `spec/evidence/` for a lowercase `t`
separator or a trailing lowercase `z` returns nothing. No retained verdict in
this repository moves.

**Scope:** intervention only. The operational schema is vendored verbatim and
was already permissive.

## §2 — Ruling: `immutableVersion` drops the `blake3:` alternative (quoin#409)

**Input that separates them:** `producer.tool_version` =
`"blake3:cccc…"` (64 hex) in an intervention record. ajv accepts; this crate
refuses.

`intervention-schema.ts:6-16` admits
`(sha256|blake3):[a-f0-9]{64}` in `immutableVersion`. Measured on `main`:

* the only digest producer in `src/measurement/` writes `sha256:` and nothing
  else (`intervention.ts:123`, `:158`);
* `git grep "blake3:"` returns eight hits and **none is data** — five doc
  comments in `quoin-store/src/digest.rs`, one negative source guard, one
  *rejection* fixture, one `blake3::hash` call;
* no `blake3:`-prefixed value exists in `spec/evidence/`, `tests/fixtures/` or
  `corpus/`;
* `quoin_store::DigestDomain` mints no prefixed blake3 value at all —
  `RawBytesDigest::to_labelled` produces one and its own doc says it is never
  the stored form.

The type system already refuses to produce what the schema admits. The vendored
pattern is therefore

```
^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|sha256:[a-f0-9]{64})$
```

**Scope:** intervention only. `operational-evidence-v1.schema.json:337` carries
the same regex and is vendored **unchanged**, because that document is copied
byte-for-byte and no ruling was taken about it. Closing that gap is the
operational wave's to take, with its own measurement.

## §3 — These two are the only differences, and that is asserted

`intervention-schema.ts` is a program, not a document, so the vendored artifact
cannot be reviewed against it by eye. `oracle/capture-intervention-schema.mjs`
ran it once and serialized the result with the repository's own `canonicalJson`
into `tests/goldens/intervention-experiment-v1.captured.json`, committed with
the revision it ran at.

`tests/tc_470_vendored_schemas.rs` then **enumerates every JSON pointer** at
which the capture and the vendored document disagree and asserts the set equals
§1 and §2, with exact before and after text. A third, unrecorded difference
fails the build. A test that merely asserted the two differ would pass for a
typo.

## §4 — `format` asserts here only where ajv asserted it

ajv under `strict: false` treats an unknown `format` as an annotation and
asserts one the moment a check is registered. The Rust `jsonschema` crate is the
other way round: under Draft 2020-12 `format` is annotation-only until
`should_validate_formats(true)`.

The two constructors keep that distinction rather than papering over it:

| constructor | registers | asserts | used by |
|---|---|---|---|
| `SchemaValidator::compile_vendored` | nothing | nothing | `quoin-semantic`, whose goldens were captured against an ajv with no format registered |
| `SchemaValidator::compile_with_formats` | the named checks | yes | both measurement schemas, whose ajv instances registered `date-time` |

A straight transliteration that skipped `should_validate_formats(true)` would
compile cleanly, pass every structural test, and silently accept
`"2026-02-30T00:00:00Z"`. `tests/tc_470_formats.rs` asserts the verdict from
both sides — refused with the check registered, accepted without it — so the
wiring cannot quietly come loose.

## §5 — Diagnostic shape: three reshapings, no verdict moves

Carried over unchanged from `quoin-semantic/DIVERGENCE.md §1`, and documented at
length in `src/validator.rs`:

1. **`additionalProperties` is fanned out** — `jsonschema` emits one error
   naming every unexpected key, ajv emits one per key.
2. **`anyOf` / `oneOf` sub-errors are flattened** — `jsonschema` nests the
   failing branches inside the composite error's `context`, ajv reports them
   alongside it.
3. **Order is `(instance location, keyword)`**, not traversal order. ajv reports
   in schema-declaration order, `jsonschema` in its own, and two `jsonschema`
   releases do not agree with each other.

None of the three changes a verdict, and **the verdict is the contractual
half**. Error count, message text and order are explicitly not contractual for
this port. `tests/tc_470_parity.rs` measures verdicts only, on 125 corpus
documents, and asserts §1 and §2 are the only entries where the two disagree.

## §6 — `quoin-jsonschema` holds no RFC 3339 grammar

`VendoredSchema::compile` takes the `date-time` check as a parameter. It is not
a plug-in seam: it exists so that this crate can sit **below**
`quoin-measurement`, whose `date_time` module is the one grammar in the
measurement domain (quoin#468), without either duplicating it or depending back
on it and forming a cycle. Callers pass
`Rfc3339DateTime::parse(value).is_ok()`. Passing anything else would recreate
the defect §1 exists to close.
