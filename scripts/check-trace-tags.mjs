#!/usr/bin/env node
// Assert every `// Trace:` tag in every test tree names a criterion that exists
// (agent-ix/quoin#390).
//
// THERE ARE TWO TREES AND THEY ARE COUNTED SEPARATELY. `tests/**/*.ts` is the
// retained TypeScript suite; `rust/**/*.rs` is the Rust workspace, whose tests
// write the tag as a `///` doc comment. A combined population passes forever:
// the TypeScript side carries hundreds of tags, so an empty-or-untagged Rust
// tree disappears into it. That is exactly how Stage 0 landed 30 passing Rust
// tests carrying nine `/// Trace: FR-096`-style tags — bare REQUIREMENT ids,
// binding no criterion — under a check that reported `0 unresolved` and exited
// 0, because `rust/` was never in the population it was reporting on. Each
// population therefore carries its own empty guard (NFR-027-AC-9: a check that
// scanned nothing is inconclusive, not clean).
//
// WHAT THIS CHECKS, and it is deliberately one thing: a tag resolves. It
// catches a criterion id that names nothing — a typo, a renamed requirement, a
// criterion deleted out from under its test — and it catches a `// Trace:` line
// carrying no id at all.
//
// WHAT IT CANNOT CATCH, stated here because a green run must not be read as
// more than it is: a tag that resolves to the WRONG criterion. Both halves of
// the FR-043/FR-095 defect (#390) were well-formed ids resolving to real
// criteria — a whole test file tagged FR-043-AC-3..AC-8 while testing FR-095's
// subject, and the matrix TC table agreeing with the wrong half. This check
// would have passed it, twice. Only reading the criterion's own words against
// the test's assertion finds that class, which is why the cutover gate reads
// criterion text rather than joining ids.
//
// It also asserts nothing about EXECUTION. A tag on a skipped test resolves
// exactly as well as a tag on a passing one. Binding-to-a-run is the evidence
// store's job and it is not yet fed (#413).
//
// The criterion population is joined from two places, because neither alone is
// complete:
//   1. criterion rows declared in spec/ tables — any suffix, see below
//   2. NFR metric rows, which carry no id in the table: the engine mints
//      `NFR-<n>-M-<k>` per row in document order and that is what tests tag
//
// The id pattern does NOT enumerate suffixes. `requirementOf`
// (src/assurance/graph.ts) is `/^([A-Za-z]+-\d+)/` — the requirement prefix is
// the whole grammar and the suffix is unconstrained. A suffix allow-list is a
// standing bet that nobody mints a new kind, and the engine took no such bet.
// An earlier list of AC|CON|VC|EX|SC silently skipped every `-M-` obligation.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

// `--root <dir>` scans a tree other than this one — a worktree of a port
// branch, say, whose spec/ and tests/ must be read together: a tag is
// unresolved only against the criteria ITS OWN tree declares.
const rootFlag = process.argv.indexOf("--root");
const root =
  rootFlag === -1
    ? dirname(dirname(fileURLToPath(import.meta.url)))
    : process.argv[rootFlag + 1];
if (!root) {
  console.error("check-trace-tags: --root needs a directory");
  process.exit(2);
}
const CRITERION = /(?:FR|NFR|StR|IT|US)-\d+-[A-Z]+-\d+/g;

// Directories that hold no authored source. `target/` is the Cargo build tree:
// it is large, it is full of vendored `.rs`, and walking it would put another
// crate's tags in this repository's population.
const SKIP_DIRS = new Set(["target", "node_modules", ".git", "dist"]);

function walk(dir, ext, out = []) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    // A tree that does not exist is an EMPTY population, not an absent check.
    // The per-population guard below is what turns that into a failure.
    return out;
  }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, ext, out);
    else if (entry.endsWith(ext)) out.push(full);
  }
  return out;
}

/** Every criterion id the spec declares, by either route. */
function declaredCriteria() {
  const declared = new Set();
  for (const file of walk(join(root, "spec"), ".md")) {
    if (file.includes(`${"/"}reviews${"/"}`)) continue;
    for (const line of readFileSync(file, "utf8").split("\n")) {
      const row = /^\|\s*((?:FR|NFR|StR|IT|US)-\d+-[A-Z]+-\d+)\s*\|/.exec(line);
      if (row) declared.add(row[1]);
    }
  }
  // NFR metric rows carry no id column; the engine mints one per row.
  for (const file of walk(join(root, "spec", "non-functional"), ".md")) {
    const text = readFileSync(file, "utf8");
    const id = /^id:\s*(NFR-\d+)/m.exec(text);
    if (!id) continue;
    let k = 0;
    let inTable = false;
    for (const line of text.split("\n")) {
      if (/^\|\s*Metric\s*\|/.test(line)) {
        inTable = true;
        continue;
      }
      if (!inTable) continue;
      if (/^\|\s*-+/.test(line)) continue;
      if (!line.startsWith("|")) {
        inTable = false;
        continue;
      }
      declared.add(`${id[1]}-M-${++k}`);
    }
  }
  return declared;
}

// One entry per tree that carries tracking tags. Each is scanned, counted and
// GUARDED on its own; see the header for why a combined guard is worthless.
//
// The tag line is `//` in TypeScript and `///` in Rust — a Rust tracking tag is
// a doc comment on the test function (`.claude/skills/rust-style/SKILL.md`), so
// a `//`-only pattern matches none of them. `//!` is an inner doc comment on
// the MODULE and is deliberately not accepted: a tag there names no test.
const TRACE_LINE = /^\s*\/\/\/?(?!\/)\s*Trace:/;

const POPULATIONS = [
  { name: "typescript", dir: "tests", ext: ".ts" },
  { name: "rust", dir: "rust", ext: ".rs" },
];

const declared = declaredCriteria();
const failures = [];
const scanned = [];

for (const population of POPULATIONS) {
  const files = walk(join(root, population.dir), population.ext);
  let tags = 0;
  let ids = 0;
  for (const file of files) {
    const lines = readFileSync(file, "utf8").split("\n");
    for (const [index, line] of lines.entries()) {
      if (!TRACE_LINE.test(line)) continue;
      tags += 1;
      const where = `${relative(root, file)}:${index + 1}`;
      const named = line.match(CRITERION) ?? [];
      if (named.length === 0) {
        failures.push(`${where}: a Trace tag naming no criterion`);
        continue;
      }
      for (const id of named) {
        ids += 1;
        if (!declared.has(id)) {
          failures.push(`${where}: ${id} — no such criterion in spec/`);
        }
      }
    }
  }
  scanned.push({ ...population, files: files.length, tags, ids });
}

// The population report, printed whether the run passes or fails: a count that
// is never shown cannot be noticed going to zero (NFR-027-AC-9).
for (const p of scanned) {
  console.log(
    `check-trace-tags: ${p.name} — ${p.files} ${p.ext} files under ${p.dir}/, ${p.tags} tags, ${p.ids} criterion references`,
  );
}

const inconclusive = [];
if (declared.size === 0) {
  inconclusive.push("0 criteria declared in spec/");
}
for (const p of scanned) {
  // Separately, per tree. An empty population passes every assertion, which is
  // not evidence — and one large population must never vouch for another.
  if (p.files === 0) {
    inconclusive.push(`${p.name}: no ${p.ext} file under ${p.dir}/`);
  } else if (p.tags === 0) {
    inconclusive.push(
      `${p.name}: ${p.files} ${p.ext} files under ${p.dir}/ and not one Trace tag`,
    );
  }
}
if (inconclusive.length > 0) {
  for (const reason of inconclusive) {
    console.error(`check-trace-tags: INCONCLUSIVE — ${reason}`);
  }
  console.error(
    "check-trace-tags: refusing to report zero findings over a population that is not what it claims",
  );
  process.exit(2);
}

for (const failure of failures) console.error(`check-trace-tags: ${failure}`);

const tags = scanned.reduce((n, p) => n + p.tags, 0);
const ids = scanned.reduce((n, p) => n + p.ids, 0);
console.log(
  `check-trace-tags: ${ids} criterion references across ${tags} tags, against ${declared.size} declared criteria — ${failures.length} unresolved`,
);
process.exit(failures.length === 0 ? 0 : 1);
