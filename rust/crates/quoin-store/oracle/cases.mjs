// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The adversarial canonicalization suite, authored once and consumed by both
// implementations.
//
// Every case is a *raw JSON document* plus a statement of what it probes. The
// documents are raw text, not JavaScript values, because half of what must be
// probed cannot be produced by `JSON.stringify` at all: duplicate member names,
// a leading byte-order mark, unpaired surrogate escapes, unescaped control
// characters, numbers that overflow to an infinity.
//
// Authoring rule: a backslash in the *document* is written `\\` here.
//
// `expect` is "accept" or "refuse" and is the authored intent. The capture
// records what the TypeScript oracle actually did; a disagreement between the
// two is a finding about the case, not about the implementation.

export const cases = [
  // ---------------------------------------------------------------- numbers
  {
    id: "num-integer-arrives-as-double",
    expect: "accept",
    probes:
      "Every JSON number is read into an IEEE-754 double, integers included. A Rust port that keeps integers as i64 would print the same text here and diverge only on the cases below — so this case fixes the model, and the next ones show what the model costs.",
    input: '{"n":1}',
  },
  {
    id: "num-negative-zero-loses-its-sign",
    expect: "accept",
    probes:
      'ECMAScript Number::toString maps both zeros to "0", so -0 and 0 canonicalize identically and share a digest. Rust\'s own formatting prints "-0" and would mint a second identity for the same record.',
    input: '{"n":-0}',
  },
  {
    id: "num-exponent-carries-an-explicit-plus",
    expect: "accept",
    probes:
      '1e21 serializes as "1e+21". ryu and Rust\'s Display both print "1e21" with no sign. One missing character changes every digest of every record containing a large number.',
    input: '{"n":1e21}',
  },
  {
    id: "num-fixed-notation-holds-to-1e20",
    expect: "accept",
    probes:
      "The fixed/exponential threshold is at 10^21, so 1e20 must print all 21 digits rather than switching to exponential. This is the exact boundary; 1e21 above is the other side of it.",
    input: '{"n":1e20}',
  },
  {
    id: "num-small-magnitude-threshold-1e-6",
    expect: "accept",
    probes:
      "The lower threshold: 1e-6 prints as 0.000001 and 1e-7 as 1e-7. A formatter with a different cutover produces different bytes for values a size or a ratio can easily take.",
    input: '{"a":1e-6,"b":1e-7}',
  },
  {
    id: "num-integer-precision-lost-beyond-2pow53",
    expect: "accept",
    probes:
      "9007199254740993 is not representable as a double and canonicalizes to 9007199254740992. The retained evidence already contains this loss, so the Rust port must reproduce it rather than fix it. A u64-backed number model would preserve the input and change the digest.",
    input: '{"n":9007199254740993}',
  },
  {
    id: "num-double-extremes",
    expect: "accept",
    probes:
      "The largest finite double and the smallest positive subnormal, where shortest-round-trip formatting is most likely to differ between implementations.",
    input: '{"max":1.7976931348623157e308,"min":5e-324}',
  },
  {
    id: "num-overflow-to-infinity-is-refused",
    expect: "refuse",
    probes:
      "1e400 is syntactically valid JSON and is not a finite double. I-JSON has no infinity, so the document is refused rather than canonicalized to something.",
    input: '{"n":1e400}',
  },
  {
    id: "num-input-spelling-is-normalized-away",
    expect: "accept",
    probes:
      "1.0E+2, 1.500 and 100 are three spellings of one double and must produce one canonical form. Canonicalization is what makes a digest independent of how the producer wrote the number.",
    input: '{"a":1.0E+2,"b":1.500,"c":100}',
  },
  {
    id: "num-shortest-round-trip-digits",
    expect: "accept",
    probes:
      "0.1 and 0.3 have no exact binary representation; the canonical form is the shortest decimal that round-trips, not the exact value. A naive 17-significant-digit formatter prints 0.10000000000000001 and diverges.",
    input: '{"a":0.1,"b":0.3,"c":0.3333333333333333}',
  },
  {
    id: "num-negative-large-and-small",
    expect: "accept",
    probes: "Sign handling across both notation thresholds.",
    input: '{"a":-1e21,"b":-1.5e-10,"c":-0.0}',
  },
  {
    id: "num-leading-zero-is-refused",
    expect: "refuse",
    probes:
      "01 is not a JSON number. A permissive reader that accepts it has two spellings for one value and therefore no canonical form.",
    input: '{"n":01}',
  },
  {
    id: "num-leading-plus-is-refused",
    expect: "refuse",
    probes:
      "+1 is not a JSON number. A reader built on a permissive numeric parse — Rust's str::parse::<f64> accepts it — reads a value the oracle refuses.",
    input: '{"n":+1}',
  },
  {
    id: "num-bare-decimal-point-is-refused",
    expect: "refuse",
    probes:
      '1. has no fraction digits. The oracle\'s number regex makes the fraction optional but not empty, so it matches "1" and the "." then fails as trailing content — refused either way, which is what must be reproduced.',
    input: "1.",
  },
  {
    id: "num-leading-decimal-point-is-refused",
    expect: "refuse",
    probes:
      ".5 has no integer part, so it is not a JSON number even though most language-level float parsers accept it.",
    input: ".5",
  },
  {
    id: "num-nan-literal-is-refused",
    expect: "refuse",
    probes:
      'NaN is a JavaScript literal, not a JSON one, and has no canonical form. Rust\'s str::parse::<f64> accepts the spelling "NaN", so a reader that delegates to it diverges here.',
    input: '{"n":NaN}',
  },
  {
    id: "num-hex-literal-is-refused",
    expect: "refuse",
    probes:
      "0x10 is a JavaScript literal, not a JSON one. A reader that accepts it has a second spelling for 16 and therefore no canonical form.",
    input: '{"n":0x10}',
  },

  // ----------------------------------------------------------- member order
  {
    id: "key-order-is-utf16-not-scalar-value",
    expect: "accept",
    probes:
      "THE ordering trap. U+10000 encodes as the surrogate pair D800 DC00; U+FFFD is the single unit FFFD. RFC 8785 sorts by UTF-16 code unit, so U+10000 comes FIRST. Rust's str: Ord compares scalar values and puts U+FFFD first. A BTreeMap-ordered port silently reverses these two members and changes the digest.",
    input: '{"\\uFFFD":1,"\\uD800\\uDC00":2}',
  },
  {
    id: "key-order-array-index-names-in-jcs",
    expect: "accept",
    probes:
      'In JCS, "10" sorts before "2" because ordering is lexicographic over code units with no numeric exception. Contrast the pretty form of this same document, which emits 2 before 10 — see the pretty-form expectation captured alongside.',
    input: '{"10":1,"2":2,"a":3,"1":4}',
  },
  {
    id: "key-order-array-index-boundary-at-2pow32-minus-2",
    expect: "accept",
    probes:
      '"4294967294" is an ECMAScript array index and "4294967295" is not. The pretty form hoists the first and leaves the second among the string keys; JCS treats both as plain strings. An off-by-one in that boundary reorders two members of the pretty form.',
    input: '{"4294967295":1,"4294967294":2,"!x":3}',
  },
  {
    id: "key-order-array-index-requires-canonical-spelling",
    expect: "accept",
    probes:
      '"01", "1.0" and "-1" look numeric and are not array indices; only "1" is. In the pretty form only "1" is hoisted.',
    input: '{"01":1,"1":2,"-1":3,"1.0":4}',
  },
  {
    id: "key-order-empty-name-sorts-first",
    expect: "accept",
    probes:
      "The empty member name is legal JSON and sorts before everything. In the pretty form it also loses to any array-index name, because index hoisting happens before sorted insertion order matters.",
    input: '{"":1,"0":2,"!x":3}',
  },
  {
    id: "key-order-is-case-sensitive-and-ascii-ordinal",
    expect: "accept",
    probes:
      '"A" (0x41) before "_" (0x5F) before "a" (0x61). Any locale-aware or case-insensitive comparison — JavaScript localeCompare, Rust\'s to_lowercase — reorders these.',
    input: '{"a":1,"A":2,"_":3,"b":4}',
  },
  {
    id: "key-order-prefix-before-extension",
    expect: "accept",
    probes:
      "A name that is a prefix of another sorts before it, because the shorter code-unit sequence runs out first. A comparator that pads or compares lengths first reorders these.",
    input: '{"ab":1,"a":2,"abc":3}',
  },
  {
    id: "key-no-unicode-normalization",
    expect: "accept",
    probes:
      "U+00E9 and e + U+0301 render identically and are two distinct members with two distinct positions. JCS explicitly does not normalize. A port that applies NFC merges them and destroys a member.",
    input: '{"\\u00e9":1,"e\\u0301":2}',
  },
  {
    id: "key-duplicate-name-is-refused",
    expect: "refuse",
    probes:
      "A permissive reader keeps one of the two and the digest then depends on which — first-wins and last-wins produce different evidence identities for the same bytes.",
    input: '{"a":1,"a":2}',
  },
  {
    id: "key-duplicate-after-unescaping-is-refused",
    expect: "refuse",
    probes:
      '"a" and "\\u0061" are the same member name. Duplicate detection must run on the decoded name, not on the source text.',
    input: '{"a":1,"\\u0061":2}',
  },
  {
    id: "key-prototype-names-are-data",
    expect: "accept",
    // MEASURED DIVERGENCE, reported with agent-ix/quoin#380.
    //
    // `canonicalizeJcs` handles these correctly: it maps over the key array and
    // never assigns into an object, so all three members survive.
    //
    // `canonicalJson` does NOT. Its `sortKeys` helper rebuilds each object as a
    // plain `{}` and assigns members into it, so `out["__proto__"] = v` hits the
    // `Object.prototype.__proto__` setter instead of creating an own property
    // and the member is **silently deleted** — at any depth, for any value type.
    // `JSON.parse` keeps `__proto__` as an own property (it uses
    // CreateDataProperty, not assignment), so `readJson` -> `writeCanonical` on
    // an evidence-store file that contains one destroys it with no diagnostic.
    //
    // The Rust port does not reproduce this. Rust has no prototype chain, the
    // name is ordinary data, and encoding a prototype-pollution artefact into
    // the replacement would be indefensible. The divergence is confined to the
    // pretty form and is declared here so the suite asserts it deliberately
    // rather than discovering it as a failure.
    divergence: {
      form: "pretty",
      summary:
        "TypeScript canonicalJson drops every __proto__ member; Rust canonical_json keeps it",
    },
    probes:
      "__proto__, constructor and toString as member names. The oracle's strict reader builds objects with a null prototype precisely so these stay data. This case is what found the canonicalJson data-loss divergence above.",
    input: '{"__proto__":1,"constructor":2,"toString":3}',
  },
  {
    id: "key-prototype-name-nested-and-alone",
    expect: "accept",
    // Same divergence, measured at depth and as the only member, to fix its
    // extent: it is not confined to the top level and it is not confined to
    // objects that have other members.
    divergence: {
      form: "pretty",
      summary:
        "TypeScript canonicalJson drops __proto__ at any depth, leaving {} behind",
    },
    probes:
      "A __proto__ member nested one level down and a __proto__ member that is an object's only member. Fixes the extent of the canonicalJson divergence: the TypeScript pretty form emits {} for the inner object and loses the member entirely.",
    input: '{"a":{"__proto__":1},"b":{"__proto__":{"x":1},"y":2}}',
  },

  // --------------------------------------------------------------- strings
  {
    id: "str-control-escapes-use-short-forms-then-lowercase-hex",
    expect: "accept",
    probes:
      "\\b \\t \\n \\f \\r have two-character forms; every other C0 control uses \\u00xx with LOWERCASE hex. Uppercase hex is a different byte and a different digest.",
    input: '{"s":"\\u0008\\u0009\\u000a\\u000c\\u000d\\u001f\\u0000\\u007f"}',
  },
  {
    id: "str-solidus-is-never-escaped-on-output",
    expect: "accept",
    probes:
      "\\/ is a legal input escape and must come back out as a bare /. Escaping it on output is legal JSON and wrong canonical JSON.",
    input: '{"s":"a\\/b"}',
  },
  {
    id: "str-input-escapes-are-normalized-to-literals",
    expect: "accept",
    probes:
      "\\u0041 must canonicalize to a bare A, and \\u00e9 to a literal é emitted as UTF-8. A port that passes escapes through unchanged produces a different byte sequence for the same string.",
    input: '{"s":"\\u0041\\u00e9\\u4e2d"}',
  },
  {
    id: "str-valid-surrogate-pair-becomes-one-character",
    expect: "accept",
    probes:
      "\\uD83D\\uDE00 is U+1F600 and is emitted as the four literal UTF-8 bytes, not as escapes.",
    input: '{"s":"\\ud83d\\ude00"}',
  },
  {
    id: "str-lone-high-surrogate-is-refused",
    expect: "refuse",
    probes:
      "\\uD800 alone has no UTF-8 encoding, so it has no canonical bytes and no digest. A port using a lossy decoder would silently substitute U+FFFD and mint a digest for a document the oracle refuses.",
    input: '{"s":"\\ud800"}',
  },
  {
    id: "str-lone-low-surrogate-is-refused",
    expect: "refuse",
    probes:
      "The mirror of the previous case: a low surrogate with nothing before it. Both halves must be refused, not just the one a pairing loop happens to notice.",
    input: '{"s":"\\udc00"}',
  },
  {
    id: "str-high-surrogate-followed-by-a-literal-astral-is-refused",
    expect: "refuse",
    probes:
      "The high surrogate is followed by a character, not by a low-surrogate escape. In UTF-16 terms the next unit is another high surrogate. Pairing must be decided over code units, not over decoded characters.",
    input: '{"s":"\\ud83d😀"}',
  },
  {
    id: "str-literal-astral-followed-by-a-lone-low-is-refused",
    expect: "refuse",
    probes:
      "The reverse: a complete character whose trailing unit is a low surrogate, followed by a stray low-surrogate escape. A port that only tracks a pending high surrogate, and forgets that a literal astral character consumed one, accepts this.",
    input: '{"s":"😀\\ude00"}',
  },
  {
    id: "str-unescaped-control-character-is-refused",
    expect: "refuse",
    probes:
      "A raw U+001F inside a string. JSON forbids it; a permissive reader accepts it and then re-emits it escaped, so read->write is not a fixed point.",
    input: '{"s":"a\u001fb"}',
  },
  {
    id: "str-unescaped-tab-is-refused",
    expect: "refuse",
    probes:
      "Tab is whitespace between tokens and forbidden inside a string. The two rules must not be confused.",
    input: '{"s":"a\tb"}',
  },
  {
    id: "str-unknown-escape-is-refused",
    expect: "refuse",
    probes:
      "\\x41 is not a JSON escape, though it is a JavaScript one. A reader that passes unknown escapes through unchanged produces bytes no other reader agrees with.",
    input: '{"s":"\\x41"}',
  },
  {
    id: "str-truncated-unicode-escape-is-refused",
    expect: "refuse",
    probes:
      "\\u must be followed by exactly four hex digits. A reader that takes as many as it finds reads a different string and a different digest.",
    input: '{"s":"\\u12"}',
  },
  {
    id: "str-noncharacters-are-preserved",
    expect: "accept",
    probes:
      "U+FFFF and U+FFFE are valid Unicode scalar values and are not replaced. A decoder that sanitizes them changes the string and the digest.",
    input: '{"s":"\\uffff\\ufffe\\ufffd"}',
  },
  {
    id: "str-bom-inside-a-string-is-content",
    expect: "accept",
    probes:
      "U+FEFF is refused only as the first character of the document. As string content it is an ordinary character and is emitted literally.",
    input: '{"s":"a\\ufeffb"}',
  },

  // -------------------------------------------------------------- documents
  {
    id: "doc-leading-byte-order-mark-is-refused",
    expect: "refuse",
    probes:
      "A BOM is not JSON whitespace. Accepting it means the same logical document has two byte spellings, one of which a naive reader skips and the other it does not.",
    input: '\ufeff{"a":1}',
  },
  {
    id: "doc-trailing-content-is-refused",
    expect: "refuse",
    probes:
      "A second value after the first. A reader that stops at the first value canonicalizes a prefix of the file and reports a digest for bytes it did not read.",
    input: '{"a":1} {"b":2}',
  },
  {
    id: "doc-trailing-comma-in-object-is-refused",
    expect: "refuse",
    probes:
      "A trailing comma is legal in JavaScript object literals and in JSON5, and is refused here. Accepting it gives one member set two byte spellings.",
    input: '{"a":1,}',
  },
  {
    id: "doc-trailing-comma-in-array-is-refused",
    expect: "refuse",
    probes:
      "A trailing comma is legal in JavaScript array literals and in JSON5, and is refused here.",
    input: "[1,]",
  },
  {
    id: "doc-comment-is-refused",
    expect: "refuse",
    probes:
      "JSON has no comments. JSON5 and several tolerant readers accept them, and a store file carrying one must be refused rather than digested as if the comment were not there.",
    input: '{"a":1}//c',
  },
  {
    id: "doc-single-quoted-string-is-refused",
    expect: "refuse",
    probes:
      "JavaScript string syntax is not JSON string syntax. A reader that accepts single quotes accepts documents no other reader in the ecosystem will.",
    input: "{'a':1}",
  },
  {
    id: "doc-unquoted-member-name-is-refused",
    expect: "refuse",
    probes:
      "JavaScript object syntax is not JSON object syntax; a member name must be a quoted string.",
    input: "{a:1}",
  },
  {
    id: "doc-empty-input-is-refused",
    expect: "refuse",
    probes:
      "An empty file is not a document. A zero-length store file must not digest to anything.",
    input: "",
  },
  {
    id: "doc-form-feed-is-not-whitespace",
    expect: "refuse",
    probes:
      "The oracle's whitespace set is space, tab, CR, LF. Form feed and vertical tab are not in it, though several JSON readers accept them.",
    input: '{"a":1}\f',
  },
  {
    id: "doc-permitted-whitespace-is-ignored",
    expect: "accept",
    probes:
      "Space, tab, CR and LF between every token, including around the top-level value, canonicalize away entirely.",
    input: ' \t\r\n{ "a" : [ 1 , 2 ] , "b" : null } \n',
  },
  {
    id: "doc-top-level-scalars-are-documents",
    expect: "accept",
    probes:
      "I-JSON permits a bare scalar as the whole document. Canonicalizing one must not require an enclosing object.",
    input: "  -1.5e3  ",
  },
  {
    id: "doc-empty-object-and-array",
    expect: "accept",
    probes:
      "The pretty form collapses empty containers to {} and [] rather than emitting an indented empty block. Getting this wrong changes every store file containing an empty list.",
    input: '{"a":{},"b":[],"c":[{}],"d":{"e":[]}}',
  },
  {
    id: "doc-nesting-and-mixed-containers",
    expect: "accept",
    probes:
      "Indentation depth accumulates in the pretty form and must not leak into the compact one. Arrays of objects are the shape most store files actually have.",
    input:
      '{"z":[{"b":[1,[2,[3]]],"a":{"deep":{"deeper":[[]]}}}],"a":[[],[[]],[[[]]]]}',
  },
  {
    id: "doc-null-false-true-are-distinct-values",
    expect: "accept",
    probes:
      "A port that coerces null to absent, or false to absent, drops a member and changes the digest.",
    input: '{"a":null,"b":false,"c":true,"d":0,"e":"","f":[],"g":{}}',
  },
  {
    id: "doc-deep-nesting-is-accepted",
    expect: "accept",
    probes:
      "Sixty-four levels of array nesting. A recursive descent reader without a stack budget is where an unbounded input turns into a crash rather than a refusal; the oracle accepts this depth, so the port must too.",
    input: `${"[".repeat(64)}1${"]".repeat(64)}`,
  },

  // ---------------------------------------------------------------- digests
  {
    id: "digest-strips-only-the-top-level-digest-member",
    expect: "accept",
    probes:
      'A nested member also named "digest" stays in the hashed bytes. Stripping it everywhere would make two different records hash the same.',
    input:
      '{"digest":"0000000000000000000000000000000000000000000000000000000000000000","a":{"digest":"kept"},"b":[{"digest":"kept"}]}',
  },
  {
    id: "digest-is-independent-of-source-member-order",
    expect: "accept",
    probes:
      "Paired with the next case: the same members written in the reverse order must produce the same canonical bytes and the same digest, which is the whole purpose of canonicalization.",
    input: '{"a":1,"b":2,"c":3}',
  },
  {
    id: "digest-is-independent-of-source-member-order-reversed",
    expect: "accept",
    probes:
      "The reversed spelling of the previous case. The two digests must be equal, or member order in whatever wrote the record leaks into the record's identity.",
    input: '{"c":3,"b":2,"a":1}',
  },
  {
    id: "digest-negative-zero-collides-with-zero",
    expect: "accept",
    probes:
      'Paired with num-negative-zero-loses-its-sign: {"n":-0} and {"n":0} are different documents with the same digest. This is a consequence of the frozen number model, recorded here so it is a known property rather than a surprise during an investigation.',
    input: '{"n":0}',
  },
];
