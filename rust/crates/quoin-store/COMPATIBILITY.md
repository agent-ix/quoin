# quoin-store - frozen compatibility surface

`agent-ix/quoin#380`, under EPIC `#373`.

This crate produces bytes that **already exist on disk**. A value this crate
canonicalizes differently from the TypeScript it replaces is a change to
retained evidence, not a change to code. Everything below is therefore frozen.

**What this crate does and does not do.** It parses stored documents,
canonicalizes values, and writes bytes a caller hands it (`write_atomic`,
`write_canonical`). It hashes no
change-assurance record, attestation or receipt: those are addressed by
`(record_id, revision)` or an assigned identity, and git history is the tamper
evidence (FR-063, FR-064, FR-065).

**What is asserted, and by what.** The serializations below are asserted by
`tests/tc_jcs_adversarial.rs` and the unit tests in `src/json/`.

---

## Frozen: the serializations

There are **two** canonical serializations and they are not interchangeable.

|                  | RFC 8785 JCS                                        | `canonicalJson`                  |
| ---------------- | --------------------------------------------------- | -------------------------------- |
| shape            | compact, no whitespace                              | 2-space indent, `": "` separator |
| trailing newline | no                                                  | yes                              |
| member order     | UTF-16 code unit                                    | ECMAScript own-property order    |
| used for         | the `change-assurance` family on disk               | every other store file           |

1. **`STORE_SCHEMA_VERSION == 1`.**
2. **JCS member order is UTF-16 code unit order.** Not Unicode scalar order.
   `U+10000` (surrogate pair `D800 DC00`) sorts _before_ `U+FFFD`; Rust's
   `str: Ord` puts it after. A `BTreeMap`-ordered port silently reverses those
   two members and changes the bytes.
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

---

## Layout

`store_root(repo) == <repo>/spec/evidence`.

`change-assurance/records/<record_id>/<revision>.json` holds one record, in JCS
bytes. A successor revision is a new file; the predecessor is never replaced.

`change-assurance/attestations/<attestation_id>/{attestation.json,output.bin}`
is made visible by a single directory rename, so a half pair is never
observable. Files written under the previous content-named layout are not read.

## Deliberate behaviour changes

Every write this crate offers is crash-atomic: temporary in the destination
directory, `fsync`, rename, `fsync` the directory. `write_new_or_identical`
publishes with `hard_link` rather than `rename`, so a concurrent writer's
differing bytes are refused rather than clobbered. Asserted by
`tests/tc_store_durability.rs`.

---

## Known divergence: `__proto__` in the pretty form

`canonicalJson` **silently deletes every `__proto__` member, at any depth, for
any value type.** Its `sortKeys` helper rebuilds each object as a plain `{}` and
assigns members into it, so `out["__proto__"] = v` reaches the
`Object.prototype.__proto__` setter instead of creating an own property.
`JSON.parse` keeps `__proto__` as an own property (it uses `CreateDataProperty`,
not assignment), so `readJson` → `writeCanonical` on a store file containing one
destroys it with no diagnostic. `canonicalizeJcs` is unaffected — it never
assigns into an object — so the JCS bytes are unaffected.

This crate keeps the member. Reproducing a prototype-pollution artefact in a
language with no prototypes would be encoding a defect as a contract.

The behaviour is asserted by `tc_380_proto_member_is_kept_as_data`.

---

---
