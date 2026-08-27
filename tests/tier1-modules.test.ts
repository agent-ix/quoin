import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, test } from "vitest";

import { ratchet } from "../scripts/lib/tier1-comparison.mjs";
import { validateCanonicalInventory } from "../scripts/lib/tier1-corpus.mjs";
import { createTier1Executor } from "../scripts/lib/tier1-execution.mjs";
import { renderTier1 } from "../scripts/lib/tier1-render.mjs";
import { localisationRate } from "../scripts/lib/tier1-scoring.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const roots: string[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true });
});

function report() {
  return {
    provenance: {
      engine: "quire test (engine test)",
      corpus: "0123456789abcdef0123456789abcdef01234567",
      declaration: {
        root: "corpus/modules",
        digest: `sha256:${"a".repeat(64)}`,
        sources: null,
      },
    },
    bounds: { gap_count: 0, declared_cells: 1 },
    families: [
      {
        family: "example",
        truePositives: 1,
        falsePositives: 0,
        misses: 0,
        precision: 1,
        recall: 1,
      },
    ],
    excluded: [],
    collateral: [],
    positional: 1,
    finding_localisation_rate: 1,
    actionability: { rate: 1, actionable: 1, total: 1 },
    "minting.section_hit_rate": null,
    cost_per_confirmed_insight: {
      toolCallsPer: 2,
      toolCalls: 2,
      truePositives: 1,
    },
    "sentinel.silent_zero": { count: 0, instances: [], unread_population: [] },
    corpora: 1,
    by_language: [{ language: "rust", corpora: 1, families: [] }],
    pending: [],
    findings: 1,
  };
}

describe("Tier-1 module contracts", () => {
  test("TC-1009 execution preserves structured locations and counts subprocesses", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-tier1-exec-"));
    roots.push(root);
    const quire = join(root, "quire");
    writeFileSync(
      quire,
      `#!/bin/sh
if [ "$1" = "coverage" ]; then
  printf '%s\\n' '{"diagnostics":[{"reason":"located","path":"spec/a.md","line":7}],"metrics":[]}'
elif [ "$1" = "properties" ]; then
  printf '%s\\n' '{"documents":[{"document":"spec/a.md","archetype":"FR","criteria":[]}],"engine":{"cli":"test","engine":"test"}}'
else
  printf '%s\\n' '{"kind":"ValidationError","message":"spec/a.md: line 9: bad [validated]"}' >&2
  exit 1
fi
`,
    );
    chmodSync(quire, 0o755);
    writeFileSync(join(root, "manifest.yaml"), "archetypes: []\n");

    const execution = createTier1Executor();
    const result = execution.findingsFor(quire, root, root, {
      families: {
        coverage: { source: "coverage.diagnostics", key: "located" },
        validate: { source: "validate.findings", key: "validated" },
      },
    });

    expect(result.findings).toMatchObject([
      {
        family: "coverage",
        reason: "located",
        path: "spec/a.md",
        line: 7,
        declaration: null,
        message: "",
      },
      {
        family: "validate",
        reason: "validated",
        path: "spec/a.md",
        line: 9,
        message: "spec/a.md: line 9: bad [validated]",
      },
    ]);
    expect(result.findings[0]).toMatchObject({
      sourceClass: "quire",
      producer: "quire",
      channel: "coverage.diagnostics",
      rawProducerOutput: { reason: "located", path: "spec/a.md", line: 7 },
    });
    expect(execution.properties(quire, root, root)).toMatchObject({
      documents: [{ document: "spec/a.md" }],
      engine: { cli: "test", engine: "test" },
    });
    expect(execution.toolCalls()).toBe(3);
  });

  test("TC-1010 modules are acyclic and rendering is deterministic from one report", () => {
    const files = [
      "tier1-comparison.mjs",
      "tier1-corpus.mjs",
      "tier1-execution.mjs",
      "tier1-measurement.mjs",
      "tier1-render.mjs",
      "tier1-scoring.mjs",
    ];
    const lib = join(repoRoot, "scripts", "lib");
    const graph = new Map<string, string[]>();
    for (const file of files) {
      const source = readFileSync(join(lib, file), "utf8");
      const imports = [...source.matchAll(/from\s+["'](.+?)["']/g)]
        .map((match) => match[1])
        .filter((path) => path.startsWith("./"))
        .map((path) => `${path.slice(2).replace(/\.mjs$/, "")}.mjs`)
        .filter((path) => files.includes(path));
      expect(source).not.toMatch(/from\s+["']\.\.\/bench-tier1\.mjs["']/);
      graph.set(file, imports);
    }
    const visit = (file: string, stack: string[]): void => {
      expect(stack, `cycle: ${[...stack, file].join(" -> ")}`).not.toContain(
        file,
      );
      for (const dependency of graph.get(file) ?? []) {
        visit(dependency, [...stack, file]);
      }
    };
    for (const file of files) visit(file, []);

    const value = report();
    const verdicts = ratchet(value, value, {
      metrics: {
        finding_precision: { direction: "higher-is-better" },
        finding_recall: { direction: "higher-is-better" },
        actionability_rate: { direction: "higher-is-better" },
        "sentinel.silent_zero": { direction: "gate-zero" },
      },
    });
    expect(renderTier1(value, verdicts)).toBe(renderTier1(value, verdicts));
    expect(localisationRate({ positional: 1, families: value.families })).toBe(
      1,
    );
    expect(
      validateCanonicalInventory({
        bounds: { gap_count: 0 },
        cases: [
          {
            id: "one",
            dir: "cases/one",
            expect: "cases/one/expect.yaml",
            module: "m",
            language: "rust",
          },
        ],
      }).cases,
    ).toHaveLength(1);
  });

  test("TC-1066 audit-sourced families execute the store-backed command path and preserve its locus", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-tier1-audit-exec-"));
    roots.push(root);
    const quire = join(root, "quire");
    writeFileSync(
      quire,
      `#!/bin/sh
if [ "$1" = "coverage" ]; then
  printf '%s\n' '{"diagnostics":[],"metrics":[],"obligations":[{"id":"FR-001-AC-1"}]}'
else
  exit 0
fi
`,
    );
    chmodSync(quire, 0o755);
    writeFileSync(join(root, "manifest.yaml"), "archetypes: []\n");
    writeFileSync(join(root, "source.rs"), "#[test]\nfn confirms() {}\n");
    const quoin = join(root, "quoin.mjs");
    writeFileSync(
      quoin,
      `const args = process.argv.slice(2);
if (args.join(" ").includes("evidence audit")) {
  console.log(JSON.stringify({findings:[{kind:"mocked-confirmation",path:"src/lib.rs",line:4,summary:"located"}],healthy:[],unevaluated:[]}));
} else {
  console.log(JSON.stringify({ok:true}));
}
`,
    );

    const execution = createTier1Executor();
    const result = execution.findingsFor(
      quire,
      root,
      root,
      {
        families: {
          mocked: {
            source: "audit.findings",
            key: "mocked-confirmation",
          },
        },
      },
      quoin,
    );
    expect(result.findings).toMatchObject([
      {
        family: "mocked",
        reason: "mocked-confirmation",
        path: "src/lib.rs",
        line: 4,
        message: "located",
      },
    ]);
    expect(result.findings[0]).toMatchObject({
      sourceClass: "quoin",
      producer: "quoin",
      channel: "evidence.audit",
      rawProducerOutput: { kind: "mocked-confirmation" },
    });
    // Symbol discovery adds one production inspect-mocks dry run.
    expect(execution.toolCalls()).toBe(10);
  });
});
