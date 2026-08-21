/** NFR-011 — owned bundle-scale work stays inside its budget (TC-290). */

import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { advise, type MethodCatalog } from "../src/advisor/index.js";
import { audit } from "../src/auditor/index.js";
import {
  assessBundle,
  type BundleReadEvent,
} from "../src/completeness/index.js";
import type { Binding, RunRecord } from "../src/evidence/index.js";
import type { Obligation } from "../src/quire/index.js";

const DOCUMENT_COUNT = 250;
const THRESHOLD_MS = 5_000;
const CORE_THRESHOLD_MS = 1_000;
const SCALE_COUNTS = [250, 1_000, 5_000, 10_000] as const;
const MAX_DOUBLING_RATIO = 3;
const COMMIT = "a".repeat(40);
const here = dirname(fileURLToPath(import.meta.url));

function timed<T>(run: () => T): { elapsedMs: number; value: T } {
  const started = performance.now();
  const value = run();
  return { elapsedMs: performance.now() - started, value };
}

function medianTimed<T>(run: () => T): { elapsedMs: number; value: T } {
  // Warm module/JIT state isolates the owned algorithm from CLI startup, which
  // is outside this core timer. A median rejects one scheduler interruption.
  run();
  const samples = Array.from({ length: 3 }, () => timed(run));
  const elapsed = samples
    .map((sample) => sample.elapsedMs)
    .sort((a, b) => a - b)[Math.floor(samples.length / 2)];
  return { elapsedMs: elapsed, value: samples.at(-1)!.value };
}

function bundleFixture(): { spec: string; moduleRoot: string } {
  const root = mkdtempSync(join(tmpdir(), "quoin-scale-"));
  const spec = join(root, "spec");
  const moduleRoot = join(root, "module");
  mkdirSync(spec, { recursive: true });
  mkdirSync(join(moduleRoot, "schemas"), { recursive: true });
  writeFileSync(
    join(moduleRoot, "schemas", "nfr.schema.json"),
    JSON.stringify({
      type: "object",
      properties: {
        quality_attribute: {
          enum: ["performance", "reliability", "security"],
        },
      },
    }),
  );
  writeFileSync(
    join(moduleRoot, "manifest.yaml"),
    [
      "manifest_version: 1",
      "name: scale-fixture",
      "artifact_types:",
      "  - name: NFR",
      "    frontmatter_schema_ref: schemas/nfr.schema.json",
      "traceability:",
      "  vocabulary_coverage:",
      "    - name: quality",
      "      from: NFR",
      "      field: quality_attribute",
      "      check: unowned-quality",
      "",
    ].join("\n"),
  );
  for (let index = 0; index < DOCUMENT_COUNT; index += 1) {
    const id = String(index + 1).padStart(3, "0");
    writeFileSync(
      join(spec, `NFR-${id}.md`),
      [
        "---",
        `id: NFR-${id}`,
        "type: NFR",
        "quality_attribute: performance",
        "---",
        "",
        "## Statement",
        "",
        "The operation completes within 5 seconds.",
        "",
      ].join("\n"),
    );
  }
  return { spec, moduleRoot };
}

function obligations(count = DOCUMENT_COUNT): Obligation[] {
  return Array.from({ length: count }, (_, index) => {
    const id = `NFR-${String(index + 1).padStart(3, "0")}-M-1`;
    return {
      source: "measurement-row" as const,
      id,
      document: `spec/NFR-${String(index + 1).padStart(3, "0")}.md`,
      statement: "Response time remains within 5 ms.",
      statement_hash: String(index).padStart(64, "0"),
      method: "benchmark",
      parameters: { threshold: "< 5 ms" },
    };
  });
}

function catalog(): MethodCatalog {
  return {
    methods: [
      {
        id: "benchmark",
        name: "Benchmark",
        class: "Test",
        definition: "Measures a quantified property.",
        evidenceKind: "Performance",
        applicability: {
          characteristics: ["latency", "quantified-threshold"],
        },
        tooling: [],
        moduleName: "scale-fixture",
      },
    ],
    duplicates: [],
    unreadable: [],
  };
}

function sharedSuiteEvidenceFor(items: Obligation[]): {
  bindings: Binding[];
  runs: RunRecord[];
} {
  const bindings: Binding[] = [];
  const entries: RunRecord["entries"] = [];
  for (const [index, obligation] of items.entries()) {
    const symbol = `tests::scale_${index + 1}`;
    bindings.push({
      obligation: obligation.id,
      statementHashAtBinding: obligation.statement_hash,
      suite: "SUITE-SHARED",
      commit: COMMIT,
      symbols: [symbol],
    });
    entries.push({ symbol, outcome: "pass" });
  }
  return {
    bindings,
    runs: [
      {
        schemaVersion: 1,
        suite: "SUITE-SHARED",
        commit: COMMIT,
        tool: "fixture",
        timestamp: "2026-08-21T20:00:00Z",
        entries,
      },
    ],
  };
}

describe("TC-290 the 250-document/obligation budget", () => {
  // Trace: NFR-011-AC-1
  // Trace: NFR-011-AC-3
  it("walks 250 documents once and completes the owned completeness phase", () => {
    const fixture = bundleFixture();
    const events: BundleReadEvent[] = [];
    const measured = timed(() =>
      assessBundle({
        bundleRoot: fixture.spec,
        moduleRoots: [fixture.moduleRoot],
        observeBundleRead: (event) => events.push(event),
      }),
    );
    expect(measured.value.vocabularies).toEqual(["quality"]);
    expect(events.filter((event) => event.kind === "pass")).toHaveLength(1);
    expect(events.filter((event) => event.kind === "document")).toHaveLength(
      DOCUMENT_COUNT,
    );
    expect(measured.elapsedMs).toBeLessThan(THRESHOLD_MS);
  });

  // Trace: NFR-011-AC-2
  it("keeps advisor growth bounded through 10,000 obligations", () => {
    const methods = catalog();
    const elapsed = SCALE_COUNTS.map((count) => {
      const items = obligations(count);
      const measured = medianTimed(() =>
        items.map((obligation) =>
          advise(methods, {
            id: obligation.id,
            statement: obligation.statement,
            authoredMethod: obligation.method,
            archetype: "NFR",
            parameters: obligation.parameters,
          }),
        ),
      );
      expect(measured.value).toHaveLength(count);
      expect(measured.value.every((item) => !item.inconclusive)).toBe(true);
      expect(measured.elapsedMs).toBeLessThan(CORE_THRESHOLD_MS);
      return measured.elapsedMs;
    });
    expect(elapsed.at(-1)! / elapsed.at(-2)!).toBeLessThanOrEqual(
      MAX_DOUBLING_RATIO,
    );
  }, 15_000);

  // Trace: NFR-011-AC-2
  it("keeps shared-suite audit growth bounded through 10,000 obligations", () => {
    const elapsed = SCALE_COUNTS.map((count) => {
      const items = obligations(count);
      const evidence = sharedSuiteEvidenceFor(items);
      const measured = medianTimed(() =>
        audit({
          obligations: items,
          bindings: evidence.bindings,
          runs: evidence.runs,
          headCommit: COMMIT,
        }),
      );
      expect(measured.value.findings).toEqual([]);
      expect(measured.value.healthy).toHaveLength(count);
      expect(measured.elapsedMs).toBeLessThan(CORE_THRESHOLD_MS);
      return measured.elapsedMs;
    });
    expect(elapsed.at(-1)! / elapsed.at(-2)!).toBeLessThanOrEqual(
      MAX_DOUBLING_RATIO,
    );
  }, 15_000);

  // Trace: NFR-011-AC-4
  it("keeps the Quire and delegated workflow boundaries outside this timer", () => {
    const matrix = readFileSync(
      join(here, "..", "src", "commands", "matrix.ts"),
      "utf8",
    );
    expect(matrix).toContain("extends FlowCommand");
    expect(matrix).toContain("await this.launch(flags)");
    expect(matrix).not.toMatch(/readBundle|runQuire/);

    for (const path of [
      join(here, "..", "src", "commands", "advise.ts"),
      join(here, "..", "src", "commands", "evidence", "audit.ts"),
    ]) {
      expect(readFileSync(path, "utf8")).toContain("runQuire(");
    }
  });
});
