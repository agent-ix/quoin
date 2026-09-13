// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

/**
 * Capture the retained TypeScript's answers for quoin#382, ONCE.
 *
 * Run from the repository root:
 *
 *   node --experimental-strip-types \
 *     rust/crates/quoin-combinatorial/oracle/capture-combinatorial.mjs \
 *     > rust/crates/quoin-combinatorial/tests/goldens/combinatorial.json
 *
 * The output is committed and is the oracle from then on. **No test spawns
 * node** (FR-101-AC-5): the retained implementation is not a runtime oracle,
 * and re-running this is a deliberate act with a diff to review, not something
 * a gate does behind your back.
 *
 * Every case here is a question about behaviour someone could get wrong, not a
 * sample of production data. The names say which question.
 */

import {
  parseSpace,
  demandedTuples,
  coveredBy,
  twayCoverage,
} from "../../../../src/auditor/combinatorial.ts";

/** @type {Array<{name: string, statement: string, configs: Array<Record<string, string>>}>} */
const CASES = [
  // --- parseSpace says "not one of these" -----------------------------------
  { name: "prose-is-not-a-space", statement: "The system shall do the thing.", configs: [] },
  { name: "header-is-anchored", statement: "see: 2-way over a(1|2) b(3|4)", configs: [] },
  { name: "header-needs-whitespace-after-over", statement: "2-way overa(1|2) b(3|4)", configs: [] },
  { name: "strength-zero-is-refused", statement: "0-way over a(1|2) b(3|4)", configs: [] },
  { name: "one-dimension-is-not-a-space", statement: "2-way over a(1|2)", configs: [] },
  {
    name: "an-infinite-strength-is-not-a-space",
    // `Number.parseInt` of 400 nines is `Infinity`, and the retained
    // `Number.isFinite` guard turns that into "not a combinatorial obligation"
    // rather than "an obligation of enormous strength".
    statement: `${"9".repeat(400)}-way over a(1|2) b(3|4)`,
    configs: [],
  },
  {
    name: "single-value-dimensions-do-not-count-toward-two",
    statement: "2-way over a(1|2) b(only)",
    configs: [],
  },
  { name: "empty-parens-are-not-a-dimension", statement: "2-way over a() b() c(1|2)", configs: [] },

  // --- parseSpace shapes ----------------------------------------------------
  {
    name: "leading-zeros-are-decimal",
    statement: "007-way over a(1|2) b(3|4) c(5|6) d(7|8) e(9|0) f(a|b) g(c|d)",
    configs: [],
  },
  {
    name: "a-single-value-dimension-is-skipped-not-fatal",
    statement: "2-way over a(1|2) b(3|4) c(only)",
    configs: [{ a: "1", b: "3", c: "only" }],
  },
  {
    name: "values-are-trimmed-and-empties-dropped",
    statement: "2-way over a( 1 | | 2 ) b(\t3\t| 4 )",
    configs: [{ a: "1", b: "3" }],
  },
  {
    name: "a-byte-order-mark-is-whitespace-to-trim",
    statement: "2-way over a(﻿1﻿|2) b(3|4)",
    configs: [{ a: "1", b: "3" }],
  },

  // --- exclusions -----------------------------------------------------------
  {
    name: "an-exclusion-is-never-read-as-a-dimension",
    statement: "2-way over a(1|2) b(3|4) excluding[a=1,b=3]",
    configs: [],
  },
  {
    name: "a-one-clause-exclusion-is-dropped",
    statement: "2-way over a(1|2) b(3|4) excluding[a=1]",
    configs: [],
  },
  {
    name: "a-clause-without-equals-is-dropped",
    statement: "2-way over a(1|2) b(3|4) excluding[a=1,nonsense,b=3]",
    configs: [],
  },
  {
    name: "a-value-may-contain-equals",
    statement: "2-way over a(x=y|2) b(3|4) excluding[a=x=y,b=3]",
    configs: [{ a: "x=y", b: "3" }],
  },
  {
    name: "two-exclusion-clauses-both-apply",
    statement: "2-way over a(1|2) b(3|4) c(5|6) excluding[a=1,b=3] excluding[b=4,c=6]",
    configs: [{ a: "1", b: "4", c: "5" }],
  },
  {
    name: "a-three-clause-exclusion-cannot-forbid-a-pair",
    statement: "2-way over a(1|2) b(3|4) c(5|6) excluding[a=1,b=3,c=5]",
    configs: [],
  },

  // --- coverage -------------------------------------------------------------
  {
    name: "a-full-two-way-array-leaves-no-gap",
    statement: "2-way over os(linux|mac) arch(x64|arm64)",
    configs: [
      { os: "linux", arch: "x64" },
      { os: "linux", arch: "arm64" },
      { os: "mac", arch: "x64" },
      { os: "mac", arch: "arm64" },
    ],
  },
  {
    name: "no-configuration-covers-nothing",
    statement: "2-way over os(linux|mac) arch(x64|arm64)",
    configs: [],
  },
  {
    name: "an-undeclared-value-covers-nothing",
    statement: "2-way over os(linux|mac) arch(x64|arm64)",
    configs: [{ os: "plan9", arch: "riscv" }],
  },
  {
    name: "a-partial-configuration-covers-only-what-it-names",
    statement: "3-way over a(1|2) b(3|4) c(5|6)",
    configs: [{ a: "1", b: "3" }],
  },
  {
    name: "three-way-over-four-dimensions",
    statement: "3-way over a(1|2) b(3|4) c(5|6) d(7|8)",
    configs: [
      { a: "1", b: "3", c: "5", d: "7" },
      { a: "2", b: "4", c: "6", d: "8" },
      { a: "1", b: "4", c: "5", d: "8" },
    ],
  },
  {
    name: "an-excluded-tuple-is-neither-demanded-nor-a-gap",
    statement: "2-way over a(1|2) b(3|4) excluding[a=1,b=3]",
    configs: [{ a: "1", b: "3" }],
  },
  {
    name: "a-run-covering-an-excluded-tuple-raises-nothing",
    statement: "2-way over a(1|2) b(3|4) c(5|6) excluding[a=1,b=3]",
    configs: [{ a: "1", b: "3", c: "5" }],
  },

  // --- ordering -------------------------------------------------------------
  {
    name: "the-key-sorts-dimensions-by-name-not-by-position",
    statement: "2-way over zeta(1|2) alpha(3|4)",
    configs: [],
  },
  {
    name: "gaps-sort-by-utf16-code-unit-not-scalar-value",
    // U+10000 is one scalar ABOVE U+FFFD and two code units BELOW it. A gap
    // list sorted by Rust's `str: Ord` puts these in the other order.
    statement: "2-way over a(\u{10000}|�) b(3|4)",
    configs: [],
  },
  {
    name: "ascii-punctuation-sorts-before-letters",
    statement: "2-way over a(-|Z|a) b(3|4)",
    configs: [],
  },

  // --- names that mean something to JavaScript ------------------------------
  {
    name: "a-dimension-named-constructor-covers-nothing-it-did-not-run",
    // `config["constructor"]` is inherited from Object.prototype and is NOT
    // undefined, so the retained `!== undefined` guard passes on a
    // configuration that never named the dimension. `values.includes(...)`
    // then rejects the function, which is why no coverage appears. This case
    // is here to hold that reasoning in place.
    statement: "2-way over constructor(1|2) b(3|4)",
    configs: [{ b: "3" }],
  },
  {
    name: "a-dimension-named-proto-behaves-like-any-other",
    statement: "2-way over __proto__(1|2) b(3|4)",
    // Written through `JSON.parse` on purpose. An object *literal* spelling
    // `__proto__: "1"` invokes the prototype setter and creates no own
    // property at all, whereas `JSON.parse` uses CreateDataProperty and does.
    // The production path reads configurations out of a run record, which is
    // parsed JSON — so the parsed spelling is the one under test, and the
    // literal spelling would have captured a fact about this script rather
    // than about the algebra.
    configs: JSON.parse('[{"__proto__":"1","b":"3"}]'),
  },
  {
    name: "two-dimensions-may-share-a-name",
    statement: "2-way over a(1|2) a(3|4)",
    configs: [{ a: "1" }],
  },
];

const cases = CASES.map(({ name, statement, configs }) => {
  const space = parseSpace(statement);
  if (space === null) {
    return { name, statement, configs, space: null, demanded: null, coveredBy: null, coverage: null };
  }
  return {
    name,
    statement,
    configs,
    space,
    demanded: [...demandedTuples(space)].sort(),
    coveredBy: configs.map((config) => [...coveredBy(space, config)].sort()),
    coverage: twayCoverage(space, configs),
  };
});

process.stdout.write(`${JSON.stringify({ cases }, null, 2)}\n`);
