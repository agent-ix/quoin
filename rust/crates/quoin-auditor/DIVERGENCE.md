<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

This crate is the port of `src/auditor/` and `src/advisor/` (quoin#383) —
1,806 lines after `src/auditor/combinatorial.ts` left for `quoin-combinatorial`
in quoin#382 — plus `src/method-catalog.ts`, which both halves read and neither
owns.

The retained TypeScript is still present and is still the oracle. Nothing in it
was edited by this wave: a port declares its divergences, it does not retrofit
the implementation it is replacing (FR-018). **In particular, the defect in §1
was not fixed in the TypeScript.** It is reported on quoin#383 so it can be
noted against the ticket that shipped it.

There are **four** divergences, **one** non-divergence that would look like one
in a diff, and **one** inherited refusal. Each is named below with the assertion
that would fail if it stopped being true.

---

## §1 — `headCommit` staleness: the retained code throws where this one answers

**Input that separates them:** an obligation bound to two suites, one backed by
a finding-shaped scan and one by a run, with `headCommit` set.

`src/auditor/audit.ts:412-419`:

```ts
const runBindings = bindings.filter((b) => runsBySuite.has(b.suite));
if (runBindings.length === 0) continue;
const runs = runBindings.map((b) => runsBySuite.get(b.suite)!);
if (input.headCommit) {
  const behind = bindings.filter((b, i) => runs[i].commit !== input.headCommit);
```

`runs` is built from `runBindings`. The filter that indexes it walks
`bindings`. Whenever any binding is scan-backed, `bindings` is longer than
`runs`, the last index is out of range, `runs[i]` is `undefined`, and reading
`.commit` throws a `TypeError` — for every obligation in the report, not just
that one, because the throw escapes `audit()` entirely. Before the last index
it is worse than a crash: binding _i_ is compared against a run that belongs to
a different suite, so a finding can name the wrong suite.

This is **the same defect that was found and fixed in the block immediately
below it**. `src/auditor/audit.ts:444-446` carries its own comment saying the
pairing must be per-binding for exactly this reason. The fix was applied there
and not here.

This crate pairs each binding with its own run — `Vec<(&Binding, &RunRecord)>`
built in one pass — so there is no second array to index and the mismatch is not
expressible. The reported suite is always the suite whose run is behind.

---

## §2 — `unreadable[].reason` is the reader's own sentence

**Input that separates them:** any module whose `manifest.yaml` is missing or
malformed.

`src/method-catalog.ts` records `cause.message` from whatever threw. Under Node
that is `ENOENT: no such file or directory, open '…'` for a missing file and
the `yaml` package's own sentence — _"Flow sequence in block collection must be
sufficiently indented and end with a ] at line 3, column 1"_ — for a malformed
one. This crate records `std::io::Error`'s message and `quoin_yaml`'s, which
are different sentences for the same two facts.

**Which modules are refused, and in which order, is identical.** Only the prose
differs, and it is prose no caller parses: it is printed for an operator, which
is the entire reason quoin#106 made an unreadable module data rather than a
crash.

---

## §3 — `readdir` order is sorted here

**Input that separates them:** a module root that is a _parent_ directory
holding more than one child with a `manifest.yaml`, or an `IX_HOME` whose
`filament/modules` holds several.

`locateModuleRoot` and `defaultModuleRoots` in `src/module-roots.ts` both walk
`readdirSync`, which returns filesystem order. `DiskModuleCatalogSource` sorts
the names before walking them. The merge is **first-wins by method id**, so
without the sort two machines could merge the same installed set in two orders
and produce two different catalogs — one of which recommends a method the other
does not.

This is the same reasoning `quoin-completeness::locate_module_root` already
documents for the same directory, and it is a divergence only in the case where
the retained side is non-deterministic. Where `readdirSync` happens to return
sorted order — which it does on ext4 for short names, and always for a single
child — the two agree.

---

## §4 — Not a divergence: key order and canonicalization

`JSON.stringify` writes object keys in insertion order; `serde` writes them in
declaration order; and neither is the order `quoin_store::canonical_json`
writes. A naive text comparison of the two reports would differ on every case
and prove nothing about their content.

Canonical JSON bytes are the comparison form. That is not a weakening: a missing field, an extra field, a renamed
field, a different number formatting, a different string or a different array
order all still fail. Only key order is normalized away, and key order is not
part of the report's meaning — every consumer reads it with a parser.

Number formatting is `jsvalue::number_text` via the shared canonical encoder, the
ECMAScript `Number::toString` algorithm, everywhere a score or a count reaches a
string. Rust's `{}` is not that algorithm, and a mutation score of `0.1` would
be the first place it showed.

**Must-have-fired:** `jsvalue::tests::a_number_is_formatted_by_ecmascript_and_not_by_rust`
states the formatter directly.

---

## §5 — Inherited: a demand past the ceiling is refused

An obligation whose statement declares a configuration space demanding more than
`quoin_combinatorial::MAX_DEMANDED_TUPLES` tuples is refused here with
`QCB-DEMAND-TOO-LARGE`, where the retained side exhausts the heap. That is
`quoin-combinatorial`'s `DIVERGENCE.md` §1, and it reaches this crate because
`audit()` calls `tway_coverage`.

The visible consequence is the signature: `audit()` returns
`Result<AuditReport, AuditorError>` rather than a report. The retained `audit()`
fails at the same point; it simply fails by dying.

---

## §6 — A rung the retained ladder never had: failed/errored latest run

`audit/stale.rs`'s `failed_run` rung (FR-032-AC-17, PLAT-1086) has no
retained-source counterpart. The retained ladder's only outcome-aware check
was vacuity, which asks `entry.outcome === "skip"` and nothing else; a bound
symbol whose newest run recorded `fail`/`error` passed through unflagged in
both trees until this rung was added here. This is a fix to a real gap in
FR-032, not a behavioural difference this port introduced by translating the
retained source wrongly — there is no retained behaviour to diverge from,
because the retained side never asked the question.

**Must-have-fired:** `tests/tc_1963_failed_run_stale_evidence.rs`
is the coverage for it.

---

## What is NOT divergent

- **The ladder order**, including the two checks that deliberately do not stop
  it: the pre-guard `unknown-method` check and the `headCommit` staleness check.
  An unbound obligation with an uncatalogued method is BOTH undischarged AND
  unknown-method (quoin#165), and the corpus asserts both findings appear.
  Excepting §6's `failed_run` rung, which the retained ladder never had.
- **`healthy`.** The retained source pushes it as the last statement of the loop
  body, so the one silent `continue` — `runBindings.length === 0` at
  `audit.ts:413` — leaves an obligation neither healthy nor found. `Pass` here
  carries a `completed` flag for that exact reason. Inferring "healthy" from "no finding" reports a stronger result than
  the retained code does.
- **String ordering.** `js::compare` is UTF-16 code-unit order, which is what
  `Array.prototype.sort` and `<` give in JavaScript, and it is not Rust's
  `str: Ord`.
- **`String(...)` coercion** of eight manifest fields. A YAML author writing
  `name: 1.0` gets `"1"` on both sides. See `src/jsvalue.rs`.
- **The regex table.** Every retained `/…/i` without the `u` flag is compiled
  with a `(?i-u)` prefix, so `\b`, `\w`, `\d` and case folding are ASCII exactly
  as in JavaScript. The three patterns that also carry `\s` or `≤`/`≥` are
  assembled with `(?i-u:…)` around the ASCII parts only, with
  `quoin_combinatorial::js::JS_WHITESPACE_CLASS` in place of `\s`, preserving
  match extents.
- **`Severity`.** An open newtype, not a closed enum, and `Finding.severity` is
  `Option`. `quoin-assurance`'s inputs carry `severity: "error"` and
  findings with no `severity` key at all, and a TypeScript union is not a
  runtime check: where the retained data is wider than the declared type, the
  retained data wins.
- **`report.independence`.** The producer writes it as a list of
  `IndependenceAssessment`s and this port does too, but it is carried in
  `quoin_finding_types::AuditReport::other` rather than declared as a typed
  field. Declaring it made the workspace gate refuse
  `quoin-graph-analysis`'s inputs, which hold
  `independence: [{"obligation": "…", "axes": ["author"]}]` — not an assessment
  at all, and accepted by every retained reader because nothing between the
  producer and the view validates the member. Same ruling as `Severity`: where
  the retained data is wider than the declared type, the retained data wins.
  The bytes are unchanged either way; the assessments stay typed on the
  producer's own side of the boundary, where they are built.
