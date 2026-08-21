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
const COMMIT = "a".repeat(40);
const here = dirname(fileURLToPath(import.meta.url));

function timed<T>(run: () => T): { elapsedMs: number; value: T } {
  const started = performance.now();
  const value = run();
  return { elapsedMs: performance.now() - started, value };
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

function obligations(): Obligation[] {
  return Array.from({ length: DOCUMENT_COUNT }, (_, index) => {
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

function evidenceFor(items: Obligation[]): {
  bindings: Binding[];
  runs: RunRecord[];
} {
  const bindings: Binding[] = [];
  const runs: RunRecord[] = [];
  for (const [index, obligation] of items.entries()) {
    const suite = `SUITE-${String(index + 1).padStart(3, "0")}`;
    const symbol = `tests::scale_${index + 1}`;
    bindings.push({
      obligation: obligation.id,
      statementHashAtBinding: obligation.statement_hash,
      suite,
      commit: COMMIT,
      symbols: [symbol],
    });
    runs.push({
      schemaVersion: 1,
      suite,
      commit: COMMIT,
      tool: "fixture",
      timestamp: "2026-08-21T20:00:00Z",
      entries: [{ symbol, outcome: "pass" }],
    });
  }
  return { bindings, runs };
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
  it("processes 250 advisor obligations inside the owned-work budget", () => {
    const methods = catalog();
    const measured = timed(() =>
      obligations().map((obligation) =>
        advise(methods, {
          id: obligation.id,
          statement: obligation.statement,
          authoredMethod: obligation.method,
          archetype: "NFR",
          parameters: obligation.parameters,
        }),
      ),
    );
    expect(measured.value).toHaveLength(DOCUMENT_COUNT);
    expect(measured.value.every((item) => !item.inconclusive)).toBe(true);
    expect(measured.elapsedMs).toBeLessThan(THRESHOLD_MS);
  });

  // Trace: NFR-011-AC-2
  it("processes 250 obligations and bindings inside the audit budget", () => {
    const items = obligations();
    const evidence = evidenceFor(items);
    const measured = timed(() =>
      audit({
        obligations: items,
        bindings: evidence.bindings,
        runs: evidence.runs,
        headCommit: COMMIT,
      }),
    );
    expect(measured.value.findings).toEqual([]);
    expect(measured.value.healthy).toHaveLength(DOCUMENT_COUNT);
    expect(measured.elapsedMs).toBeLessThan(THRESHOLD_MS);
  });

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
