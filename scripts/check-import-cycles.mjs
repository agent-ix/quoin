#!/usr/bin/env node
/**
 * Refuse an import cycle anywhere in `src/` (agent-ix/quoin#376).
 *
 * Cargo forbids crate cycles. The crate topology in agent-ix/quoin#373 is drawn
 * along the top-level units of `src/` — a directory such as `src/evidence/`, or
 * a leaf file such as `src/catalog.ts` — so a cycle between two of those units
 * is a crate cycle waiting to happen and has to fail here rather than at the
 * first `cargo build` of a staged port.
 *
 * **Whole-graph, not named pairs.** #376 opened naming two cycles; a third —
 * `auditor -> evidence -> change-assurance -> auditor` — ran through the same
 * strongly connected component and survives cuts that satisfy the two named
 * ones literally. So the check condenses the entire import graph with Tarjan's
 * algorithm and refuses any component holding more than one unit. A pairwise
 * assertion is exactly what would let the third one through.
 *
 * The graph is sound because it is complete: every edge is a static `import`,
 * `export ... from` or `import(...)` specifier, and the repository has no
 * dynamic specifier for one to miss.
 *
 * Deliberately NOT a file-level check. ESM resolves a file-level cycle inside
 * one module at runtime, and forbidding those would refuse code that is legal
 * in both languages. The unit of the check is the unit of the eventual crate.
 *
 * No new dependency: `scripts/` is already the home of the repository's own
 * gates, and this one reads the same import syntax `tsc` does without a parser.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = join(ROOT, "src");

/** Every `from "..."` / `import("...")` specifier in a TypeScript source. */
const SPECIFIER =
  /(?:^|[\s;{(])(?:import|export)\s[^;]*?from\s*["']([^"']+)["']|\bimport\s*\(\s*["']([^"']+)["']\s*\)/g;

function sources(directory) {
  const out = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) out.push(...sources(path));
    else if (entry.endsWith(".ts") && !entry.endsWith(".d.ts")) out.push(path);
  }
  return out;
}

/**
 * The top-level unit of `src/` a file belongs to: `evidence`, `catalog`, ...
 *
 * Both extensions are stripped, because the same leaf unit is named twice in
 * two spellings: the file on disk is `src/catalog.ts`, while every specifier
 * that imports it is the ESM-resolved `../catalog.js`. Stripping only `.ts`
 * left the importer's target as the unit `catalog.js`, which joined to no
 * source file, so every top-level leaf module was invisible as an import
 * target and its cycles went unreported (agent-ix/quoin#397 review).
 */
function unit(src, path) {
  const parts = relative(src, path).split(sep);
  return parts.length === 1 ? parts[0].replace(/\.(ts|js)$/, "") : parts[0];
}

/**
 * Condense `src` into its unit-level import graph.
 *
 * `root` only prefixes the reported edge sites, so a fixture tree can report
 * paths relative to itself.
 */
export function importGraph(src, root = src) {
  const graph = new Map();
  const edgeSites = new Map();

  for (const file of sources(src)) {
    const from = unit(src, file);
    if (!graph.has(from)) graph.set(from, new Set());
    const text = readFileSync(file, "utf8");
    const lines = text.split("\n");
    for (const match of text.matchAll(SPECIFIER)) {
      const specifier = match[1] ?? match[2];
      if (!specifier.startsWith(".")) continue;
      const target = resolve(dirname(file), specifier);
      if (!target.startsWith(src + sep)) continue;
      const to = unit(src, target);
      if (to === from) continue;
      graph.get(from).add(to);
      const key = `${from} -> ${to}`;
      if (!edgeSites.has(key)) {
        const at = match.index + match[0].indexOf(specifier);
        const index = text.slice(0, at).split("\n").length;
        edgeSites.set(
          key,
          `${relative(root, file)}:${index}: ${lines[index - 1].trim()}`,
        );
      }
    }
  }
  return { graph, edgeSites };
}

/**
 * Tarjan's strongly connected components, iteratively (the graph is small, but
 * a gate that overflows its own stack fails for the wrong reason).
 */
function stronglyConnectedComponents(graph) {
  const index = new Map();
  const low = new Map();
  const onStack = new Set();
  const stack = [];
  const components = [];
  let next = 0;

  for (const root of [...graph.keys()].sort()) {
    if (index.has(root)) continue;
    const work = [{ node: root, edges: [...(graph.get(root) ?? [])].sort() }];
    index.set(root, next);
    low.set(root, next);
    next++;
    stack.push(root);
    onStack.add(root);

    while (work.length > 0) {
      const frame = work[work.length - 1];
      if (frame.edges.length > 0) {
        const child = frame.edges.shift();
        if (!index.has(child)) {
          index.set(child, next);
          low.set(child, next);
          next++;
          stack.push(child);
          onStack.add(child);
          work.push({
            node: child,
            edges: [...(graph.get(child) ?? [])].sort(),
          });
        } else if (onStack.has(child)) {
          low.set(frame.node, Math.min(low.get(frame.node), index.get(child)));
        }
        continue;
      }
      work.pop();
      if (work.length > 0) {
        const parent = work[work.length - 1].node;
        low.set(parent, Math.min(low.get(parent), low.get(frame.node)));
      }
      if (low.get(frame.node) === index.get(frame.node)) {
        const component = [];
        let member;
        do {
          member = stack.pop();
          onStack.delete(member);
          component.push(member);
        } while (member !== frame.node);
        components.push(component);
      }
    }
  }
  return components;
}

/** Every component of the unit graph holding more than one unit. */
export function cyclicComponents(graph) {
  return stronglyConnectedComponents(graph).filter(
    (component) => component.length > 1,
  );
}

/**
 * The gate itself: `{ ok, report }` over one source tree, so a test can assert
 * the verdict on a fixture graph without shelling out or exiting.
 */
export function checkImportCycles(src, root = src) {
  const { graph, edgeSites } = importGraph(src, root);
  const cyclic = cyclicComponents(graph);
  const report = [];

  if (cyclic.length === 0) {
    report.push(
      `check-import-cycles: ${graph.size} src modules, no cyclic component`,
    );
    return { ok: true, report, components: cyclic };
  }

  report.push(
    `check-import-cycles: ${cyclic.length} cyclic component(s) in the src import graph.`,
    "Cargo forbids crate cycles (agent-ix/quoin#376). Extract the shared surface",
    "into a lower module instead of cutting an edge arbitrarily.\n",
  );
  for (const component of cyclic) {
    const members = [...component].sort();
    report.push(`  component: ${members.join(", ")}`);
    for (const from of members) {
      for (const to of [...graph.get(from)].sort()) {
        if (!members.includes(to)) continue;
        report.push(`    ${from} -> ${to}`);
        const site = edgeSites.get(`${from} -> ${to}`);
        if (site) report.push(`      ${site}`);
      }
    }
    report.push("");
  }
  return { ok: false, report, components: cyclic };
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  const { ok, report } = checkImportCycles(SRC, ROOT);
  for (const line of report) (ok ? console.log : console.error)(line);
  process.exit(ok ? 0 : 1);
}
