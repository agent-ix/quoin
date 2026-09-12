#!/usr/bin/env node
// Assert every `// Trace:` tag in the test tree names a criterion that exists
// (agent-ix/quoin#390).
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

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const CRITERION = /(?:FR|NFR|StR|IT|US)-\d+-[A-Z]+-\d+/g;

function walk(dir, ext, out = []) {
  for (const entry of readdirSync(dir)) {
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

const declared = declaredCriteria();
const failures = [];
let tags = 0;
let ids = 0;

for (const file of walk(join(root, "tests"), ".ts")) {
  const lines = readFileSync(file, "utf8").split("\n");
  for (const [index, line] of lines.entries()) {
    if (!/^\s*\/\/\s*Trace:/.test(line)) continue;
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

if (declared.size === 0 || tags === 0) {
  // An empty population passes every assertion, which is not evidence.
  console.error(
    `check-trace-tags: refusing an empty population — ${declared.size} criteria declared, ${tags} tags found`,
  );
  process.exit(2);
}

for (const failure of failures) console.error(`check-trace-tags: ${failure}`);

console.log(
  `check-trace-tags: ${ids} criterion references across ${tags} tags, against ${declared.size} declared criteria — ${failures.length} unresolved`,
);
process.exit(failures.length === 0 ? 0 : 1);
