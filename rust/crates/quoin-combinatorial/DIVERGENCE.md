<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

This crate is the port of `src/auditor/combinatorial.ts` (quoin#382): 195 lines
of t-way covering-array algebra with zero imports.

The retained TypeScript is still present and is still the oracle. Nothing in it
was edited by this wave — a port declares its divergences, it does not retrofit
the implementation it is replacing (FR-018).

There are **two** divergences and **one** near-divergence that turns out not to
be one. Each is named below with the assertion that would fail if it stopped
being true.

---

## §1 — A demand past the ceiling is refused, not exhausted

**Input that separates them:** any statement whose declared space demands more
than `MAX_DEMANDED_TUPLES` (1,000,000) tuples — for instance thirty binary
dimensions at `30-way`, which demands 2^30.

The retained `demandedTuples` accumulates into an unbounded `Set<string>`. On
such a statement the process allocates roughly a billion strings and dies: there
is no diagnostic, no partial report, and no way for a caller to tell heap
exhaustion from any other crash. That is not an _answer_ the retained side
gives; it is the absence of one.

This crate refuses by name instead, with `QCB-DEMAND-TOO-LARGE`. Two guards
produce it:

1. a bound on the number of index combinations, computed with a binomial that
   saturates at one past the ceiling rather than overflowing; and
2. a running leaf count in the value walk.

There is also a third, which is the same bound stated at the top: every
dimension carries at least two values by construction, so a reachable strength
of 20 or more already demands at least 2^20 > 1,000,000 tuples. Refusing there
caps the value walk's recursion depth at 19 instead of at the dimension count,
which the statement alone controls — a stack overflow is no more of a diagnostic
than a heap exhaustion.

**The difference is refusal versus exhaustion, not two different answers.** For
every space below the ceiling the two sides agree exactly, and the golden corpus
is the evidence.

**Must-have-fired:** `tc_382_002_a_demand_past_the_ceiling_is_refused_where_the_retained_side_exhausts_the_heap`
asserts the refusal _and_ its code, and then asserts that a space under the
ceiling still computes. If the ceiling were ever removed, the first assertion
fails and this section stops being true out loud rather than quietly.

---

## §2 — A strength above `u64::MAX` saturates, and that cannot be observed

**Input that separates them:** `100000000000000000000-way over a(1|2) b(3|4)`.

The retained side holds `strength` as a double, so it is `1e20` there. This
crate holds it as a `u64` and saturates to `u64::MAX`. The parsed spaces
therefore differ in that one field.

**It is unreachable in any report, and that is proved rather than asserted.**
`CoverageResult::strength` has exactly one consumer — `src/auditor/audit.ts:700`
— and it interpolates the number solely inside a `covered < demanded` branch.
`covered <= demanded` always holds, so reaching that branch needs `demanded > 0`,
which needs `strength <= dimensions.len()`. A space holding `u64::MAX`
dimensions cannot be constructed: each dimension costs at least four bytes of
statement text.

Note what is **not** a divergence here. The retained guard is
`!Number.isFinite(strength) || strength < 1`, and both halves matter: a 400-digit
header parses to `Infinity` and is therefore _not a combinatorial obligation at
all_. This crate reproduces that exactly by reading the digits as an `f64`
first — Rust and ECMAScript both round decimal to the nearest double — rather
than approximating the boundary with a digit count. The golden corpus carries
the case (`an-infinite-strength-is-not-a-space`).

**Must-have-fired:** `tc_382_003_a_strength_above_u64_saturates_and_that_cannot_be_observed`
asserts that the saturation actually happened (so the test cannot pass by the
divergence having silently gone away), and then asserts the zero demand that
makes the finding branch unreachable.

---

## §3 — Not a divergence: JavaScript's inherited property names

**Input that looks like it separates them:** a space declaring a dimension named
`constructor`, measured against a configuration that never names it.

The retained `coveredBy` filters with
`config[d.name] !== undefined && d.values.includes(config[d.name])`. For
`d.name === "constructor"`, `config["constructor"]` is inherited from
`Object.prototype` and is **not** `undefined`, so the first guard passes on a
configuration that never mentioned the dimension. A `BTreeMap` has no such
inheritance, so the two sides reach the second guard by different routes.

They agree because `d.values.includes(...)` compares against strings and the
inherited value is a function. The agreement is therefore a consequence of the
second guard, not of the first — which is exactly why it is written down here
rather than left to be rediscovered the next time someone considers "simplifying"
the filter.

The same reasoning covers `__proto__`, with one wrinkle that belongs to the
capture script rather than to the algebra: an object _literal_ spelling
`__proto__: "1"` invokes the prototype setter and creates no own property, while
`JSON.parse` uses `CreateDataProperty` and does. The production path reads
configurations out of a parsed run record, so the oracle captures the parsed
spelling; `oracle/capture-combinatorial.mjs` says so at the case.

**Must-have-fired:** `tc_382_004_an_inherited_property_name_is_not_a_divergence_because_the_value_guard_catches_it`,
plus the two captured cases in the golden corpus.

---

## What the fixture covers, and what it cannot

`tests/goldens/combinatorial.json` holds 31 cases, captured once by
`oracle/capture-combinatorial.mjs` and committed. **No test spawns node**
(FR-101-AC-5).

It covers every `parseSpace` refusal path (no header, an unanchored header, no
whitespace after `over`, strength `0`, an infinite strength, fewer than two
multi-valued dimensions, empty parentheses); value trimming with the ECMAScript
whitespace set including U+FEFF; exclusion parsing (a one-clause list, a clause
with no `=`, a value containing `=`, two clause lists, a three-clause exclusion
that cannot forbid a pair); 2-way and 3-way demand; a partial configuration; an
undeclared value; gap ordering above the BMP; and three identifier shapes that
mean something to JavaScript.

It **cannot** cover the resource ceiling of §1, because the retained side
answers it by dying, and it cannot cover the saturation of §2, because the
retained side holds the number as a double. Both are handled as declared
divergences with must-have-fired assertions above. Widening the fixture until
those looked covered would have produced a corpus that agrees with itself.

What no fixture of any size can cover is a statement about _all_ spaces — that a
full factorial run always closes every gap, that adding a configuration never
removes coverage, that no excluded combination is ever demanded. Those are in
`tests/tc_382_properties.rs` as `proptest` properties, each with an anti-vacuity
assertion that fails at zero.
