/**
 * The import-cycle gate, checked against fixture graphs with known verdicts.
 *
 * **Why fixtures and not `src/`.** A test that only runs the gate over the real
 * tree and asserts "clean" passes over whatever population the gate managed to
 * see. That is precisely how the defect this file exists for survived: `unit()`
 * stripped only `.ts`, so the specifier `../catalog.js` condensed to the unit
 * `catalog.js` while the file `src/catalog.ts` condensed to `catalog`, the two
 * never joined, and twelve top-level leaf modules were invisible as import
 * targets. The tree was reported clean because half the edges were missing, not
 * because there were no cycles. So every case below is a graph built here whose
 * cycles are known by construction, and each asserts the verdict the gate must
 * reach — including one cycle that is only visible when a `.js` specifier
 * resolves onto the `.ts` file it names.
 *
 * `scripts/` is outside vitest's coverage `include`, but not outside its test
 * collection: the runner's default include picks up `tests/**` and this file
 * needs no configuration change, exactly like `tests/span-breadth.test.ts`
 * importing `scripts/verify-span-breadth.mjs`.
 */

import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { checkImportCycles } from "../scripts/check-import-cycles.mjs";

const trees: string[] = [];

afterEach(() => {
  while (trees.length > 0)
    rmSync(trees.pop()!, { recursive: true, force: true });
});

/** Materialise `{ "a.ts": "...", "b/index.ts": "..." }` as a source tree. */
function tree(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-cycles-"));
  trees.push(root);
  const src = join(root, "src");
  for (const [path, text] of Object.entries(files)) {
    const file = join(src, path);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, text, "utf8");
  }
  return src;
}

describe("check-import-cycles", () => {
  it("refuses a cycle between two top-level leaf files", () => {
    // Both edges are leaf-to-leaf, and both specifiers are `.js`. Before the
    // extension fix neither edge existed at all: `./beta.js` condensed to the
    // unit `beta.js`, which matches no file, so this graph read as two isolated
    // units and the gate passed a plain two-node cycle.
    const src = tree({
      "alpha.ts": 'import { b } from "./beta.js";\nexport const a = b;\n',
      "beta.ts":
        'import type { A } from "./alpha.js";\nexport const b: A = 1;\n',
    });

    const result = checkImportCycles(src);

    expect(result.ok).toBe(false);
    expect(result.components).toHaveLength(1);
    expect([...result.components[0]].sort()).toEqual(["alpha", "beta"]);
    expect(result.report.join("\n")).toContain("component: alpha, beta");
  });

  it("refuses a directory/leaf cycle that only a resolved .js specifier closes", () => {
    // `gamma -> delta` is a directory-to-leaf edge and was always visible.
    // The return edge `delta -> gamma` is the leaf file `delta.ts` importing
    // `./gamma/index.js`; the cycle closes only because `delta.ts` is itself
    // reached as the unit `delta` through the `.js` specifier below. Stripping
    // only `.ts` broke that join and the gate reported no cyclic component.
    const src = tree({
      "gamma/index.ts": 'export { d } from "../delta.js";\n',
      "gamma/helper.ts":
        'import { d } from "../delta.js";\nexport const h = d;\n',
      "delta.ts":
        'import { x } from "./gamma/index.js";\nexport const d = x;\n',
      "epsilon.ts": 'import { d } from "./delta.js";\nexport const e = d;\n',
    });

    const result = checkImportCycles(src);

    expect(result.ok).toBe(false);
    const members = result.components.map((c: string[]) => [...c].sort());
    expect(members).toEqual([["delta", "gamma"]]);
    // `epsilon` imports into the cycle but is not part of it: a component must
    // hold only the units that are mutually reachable.
    expect(members.flat()).not.toContain("epsilon");
  });

  it("names the file and line of each edge inside a refused component", () => {
    const src = tree({
      "one.ts": '\nimport { t } from "./two.js";\nexport const o = t;\n',
      "two.ts": '\n\nimport { o } from "./one.js";\nexport const t = o;\n',
    });

    const report = checkImportCycles(src, src).report.join("\n");

    expect(report).toContain('one.ts:2: import { t } from "./two.js";');
    expect(report).toContain('two.ts:3: import { o } from "./one.js";');
  });

  it("refuses a three-unit cycle no pair of units reveals", () => {
    // The gate condenses the whole graph rather than asserting named pairs;
    // this component holds no two-node cycle at all.
    const src = tree({
      "red/index.ts": 'export { g } from "../green.js";\n',
      "green.ts": 'import { b } from "./blue/index.js";\nexport const g = b;\n',
      "blue/index.ts":
        'import { r } from "../red/index.js";\nexport const b = r;\n',
    });

    const result = checkImportCycles(src);

    expect(result.ok).toBe(false);
    expect(result.components).toHaveLength(1);
    expect([...result.components[0]].sort()).toEqual(["blue", "green", "red"]);
  });

  it("accepts an acyclic graph whose every edge is a .js specifier", () => {
    // The counterpart case: the extension fix must add the missing edges, not
    // invent them. Every unit here is reachable as an import target and the
    // graph is still a DAG.
    const src = tree({
      "top.ts": 'import { m } from "./middle/index.js";\nexport const t = m;\n',
      "middle/index.ts":
        'import { l } from "../leaf.js";\nexport const m = l;\n',
      "leaf.ts": "export const l = 1;\n",
    });

    const result = checkImportCycles(src);

    expect(result.ok).toBe(true);
    expect(result.components).toEqual([]);
    expect(result.report.join("\n")).toContain("3 src modules");
  });

  it("ignores package specifiers and paths outside the source tree", () => {
    const src = tree({
      "solo.ts":
        'import { z } from "zod";\nimport { x } from "../../outside.js";\nexport const s = [z, x];\n',
      "other.ts": 'import { s } from "./solo.js";\nexport const o = s;\n',
    });

    const result = checkImportCycles(src);

    expect(result.ok).toBe(true);
  });
});
