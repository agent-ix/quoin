/** Report-command integration for governed graph portfolio wiring. */

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, expect, test, vi } from "vitest";

import ReportCommand from "../src/commands/report.js";
import { STORE_SCHEMA_VERSION, writeBindings } from "../src/evidence/index.js";
import {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  DEFAULT_RELATION_KINDS,
  loadGraphAnalysisInput,
} from "../src/graph-analysis/index.js";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const digest = "a".repeat(64);
const revision = "b".repeat(40);
let config: Config;
const roots: string[] = [];

beforeAll(async () => {
  config = await loadConfig({ root: projectRoot });
});

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "quoin-graph-portfolio-command-"));
  roots.push(root);
  mkdirSync(join(root, "inputs"), { recursive: true });
  const source = { repository: "agent-ix/example", revision };
  const modules = [
    {
      name: "example",
      version: "1.0.0",
      schemas: [{ archetype: "FR", schema_digest: "c".repeat(64) }],
    },
  ];
  const premises = { format: "quire-assurance", format_version: 1, modules };
  const exportValue = {
    ...premises,
    source,
    artifacts: [
      {
        id: "FR-001",
        artifact_type: "FR",
        locator: { path: "spec/FR-001.md", line: 1, digest },
      },
    ],
    obligations: [
      {
        source: "acceptance-criterion",
        id: "FR-001-AC-1",
        document: "spec/FR-001.md",
        statement: "The portfolio preserves graph reports.",
        statement_hash: digest,
        target_ids: ["TC-1311"],
        locator: { path: "spec/FR-001.md", line: 20, digest },
      },
    ],
    symbols: [],
    relation_kinds: DEFAULT_RELATION_KINDS.map((kind) => ({
      kind,
      availability: "available",
      sources: ["module_vocabulary"],
    })),
    relations: [],
    relation_observations: [],
  };
  const audit = {
    format: "quoin-audit-envelope",
    format_version: 1,
    source,
    export: premises,
    report: { findings: [], healthy: ["FR-001-AC-1"], unevaluated: [] },
  };
  const exportPath = join(root, "inputs", "assurance.json");
  const premisesPath = join(root, "inputs", "premises.json");
  const auditPath = join(root, "inputs", "audit.json");
  writeFileSync(exportPath, JSON.stringify(exportValue));
  writeFileSync(premisesPath, JSON.stringify(premises));
  writeFileSync(auditPath, JSON.stringify(audit));
  writeBindings(root, {
    schemaVersion: STORE_SCHEMA_VERSION,
    bindings: [
      {
        obligation: "FR-001-AC-1",
        statementHashAtBinding: digest,
        suite: "unit",
        commit: revision,
        symbols: ["portfolio test"],
      },
    ],
  });
  return { root, exportPath, premisesPath, auditPath };
}

test("TC-1311 report embeds the exact standalone structural report objects", async () => {
  const paths = fixture();
  const loaded = loadGraphAnalysisInput({
    repo: paths.root,
    exportPath: paths.exportPath,
    premisesPath: paths.premisesPath,
    auditPath: paths.auditPath,
  });
  if (!loaded.ok) throw new Error(loaded.error.message);
  const lines: string[] = [];
  vi.spyOn(console, "log").mockImplementation((line) =>
    lines.push(String(line)),
  );
  await ReportCommand.run(
    [
      "--portfolio",
      paths.root,
      "--graph-export",
      `${paths.root}=${paths.exportPath}`,
      "--graph-premises",
      `${paths.root}=${paths.premisesPath}`,
      "--graph-audit",
      `${paths.root}=${paths.auditPath}`,
      "--changed",
      `${paths.root}=FR-001`,
      "--format",
      "json",
    ],
    config,
  );
  const graph = JSON.parse(lines.join("\n")).repositories[0].graph;
  expect(graph.fanOut).toEqual(analyzeFanOut(loaded.value));
  expect(graph.churn).toEqual(analyzeChurn(loaded.value));
  expect(graph.changeImpact).toEqual([
    analyzeChangeImpact(loaded.value, ["FR-001"]),
  ]);
});

test("TC-1311 partial triples are incompatible before a missing export is read", async () => {
  const paths = fixture();
  const lines: string[] = [];
  vi.spyOn(console, "log").mockImplementation((line) =>
    lines.push(String(line)),
  );
  await ReportCommand.run(
    [
      "--portfolio",
      paths.root,
      "--graph-export",
      `${paths.root}=${join(paths.root, "does-not-exist.json")}`,
      "--format",
      "json",
    ],
    config,
  );
  expect(JSON.parse(lines.join("\n")).repositories[0].graph).toMatchObject({
    availability: "incompatible",
    reason: expect.stringContaining("export, premises, and audit"),
  });
});
