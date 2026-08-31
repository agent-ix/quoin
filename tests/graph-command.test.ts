/** FR-062 command and static boundary coverage. */

import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import GraphFanOut from "../src/commands/graph/fan-out.js";
import GraphChangeImpact from "../src/commands/graph/change-impact.js";
import GraphChurn from "../src/commands/graph/churn.js";
import {
  bindingsPath,
  STORE_SCHEMA_VERSION,
  writeBindings,
} from "../src/evidence/index.js";
import {
  DEFAULT_RELATION_KINDS,
  analyzeFanOut,
  loadGraphAnalysisInput,
} from "../src/graph-analysis/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const digest = "a".repeat(64);
const revision = "b".repeat(40);
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

afterEach(() => {
  vi.restoreAllMocks();
});

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "quoin-graph-command-"));
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
        statement: "The command reports fan-out.",
        statement_hash: digest,
        target_ids: ["TC-1249"],
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
        symbols: ["fan-out test"],
      },
    ],
  });
  return { root, exportPath, premisesPath, auditPath };
}

describe("FR-062 graph command", () => {
  it("runs fan-out over the three explicit accepted inputs", async () => {
    const paths = fixture();
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      lines.push(String(line)),
    );
    await GraphFanOut.run(
      [
        "--repo",
        paths.root,
        "--export",
        paths.exportPath,
        "--premises",
        paths.premisesPath,
        "--audit",
        paths.auditPath,
        "--json",
      ],
      config,
    );
    expect(JSON.parse(lines.join("\n"))).toMatchObject({
      view: "fan-out",
      state: "complete",
      rows: [{ suite: "unit", obligationCount: 1 }],
    });
  });

  it("runs the non-JSON path with the inherited update nudge disabled", async () => {
    const paths = fixture();
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      lines.push(String(line)),
    );
    await GraphFanOut.run(
      [
        "--repo",
        paths.root,
        "--export",
        paths.exportPath,
        "--premises",
        paths.premisesPath,
        "--audit",
        paths.auditPath,
      ],
      config,
    );
    expect(lines.join("\n")).toContain("Suite");
    expect(
      readFileSync(join(repoRoot, "src/commands/graph/fan-out.ts"), "utf8"),
    ).toContain("skipUpdateNudge = true");
  });

  it("runs churn over the same accepted inputs", async () => {
    const paths = fixture();
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      lines.push(String(line)),
    );
    await GraphChurn.run(
      [
        "--repo",
        paths.root,
        "--export",
        paths.exportPath,
        "--premises",
        paths.premisesPath,
        "--audit",
        paths.auditPath,
        "--json",
      ],
      config,
    );
    expect(JSON.parse(lines.join("\n"))).toMatchObject({
      view: "churn",
      state: "complete",
      rows: [{ obligation: "FR-001-AC-1", eventCount: 0 }],
    });
  });

  it("runs change-impact with required requirement seeds", async () => {
    const paths = fixture();
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      lines.push(String(line)),
    );
    await GraphChangeImpact.run(
      [
        "--repo",
        paths.root,
        "--export",
        paths.exportPath,
        "--premises",
        paths.premisesPath,
        "--audit",
        paths.auditPath,
        "--requirement",
        "FR-001",
        "--json",
      ],
      config,
    );
    expect(JSON.parse(lines.join("\n"))).toMatchObject({
      view: "change-impact",
      state: "complete",
      requested: ["FR-001"],
      rows: [{ requirement: "FR-001", depth: 0 }],
    });
  });

  it("TC-1257 fails required paths before rows and reports an unreadable retained store", () => {
    const paths = fixture();
    const missing = join(paths.root, "inputs", "missing.json");
    const noExport = loadGraphAnalysisInput({
      repo: paths.root,
      exportPath: missing,
      premisesPath: paths.premisesPath,
      auditPath: paths.auditPath,
    });
    const noPremises = loadGraphAnalysisInput({
      repo: paths.root,
      exportPath: paths.exportPath,
      premisesPath: missing,
      auditPath: paths.auditPath,
    });
    const noAudit = loadGraphAnalysisInput({
      repo: paths.root,
      exportPath: paths.exportPath,
      premisesPath: paths.premisesPath,
      auditPath: missing,
    });
    expect(noExport).toMatchObject({ ok: false, error: { input: "export" } });
    expect(noPremises).toMatchObject({
      ok: false,
      error: { input: "premises" },
    });
    expect(noAudit).toMatchObject({ ok: false, error: { input: "audit" } });

    writeFileSync(bindingsPath(paths.root), "not JSON");
    const invalidJson = loadGraphAnalysisInput({
      repo: paths.root,
      exportPath: paths.exportPath,
      premisesPath: paths.premisesPath,
      auditPath: paths.auditPath,
    });
    expect(invalidJson).toMatchObject({
      ok: true,
      value: { bindings: { availability: "unreadable" } },
    });

    writeFileSync(bindingsPath(paths.root), "{}");
    const malformed = loadGraphAnalysisInput({
      repo: paths.root,
      exportPath: paths.exportPath,
      premisesPath: paths.premisesPath,
      auditPath: paths.auditPath,
    });
    expect(malformed).toMatchObject({
      ok: true,
      value: { bindings: { availability: "unreadable" } },
    });
    if (!malformed.ok) throw new Error("unreachable");
    expect(analyzeFanOut(malformed.value)).toMatchObject({
      state: "not_computed",
      rows: [],
    });
  });

  // Trace: FR-062-AC-11
  it("TC-1259 keeps producers, writes, frontmatter, and a second graph outside every view", () => {
    const paths = [
      "src/commands/graph/common.ts",
      "src/commands/graph/index.ts",
      "src/commands/graph/fan-out.ts",
      "src/commands/graph/change-impact.ts",
      "src/commands/graph/churn.ts",
      "src/graph-analysis/analysis.ts",
      "src/graph-analysis/input.ts",
      "src/graph-analysis/load.ts",
    ];
    const sourceText = paths
      .map((path) => readFileSync(join(repoRoot, path), "utf8"))
      .join("\n");
    expect(sourceText).not.toMatch(
      /node:child_process|runQuire|quireExecutable|readBundleFrontmatter|writeFileSync|appendFileSync|execFileSync|spawnSync|fetch\s*\(|\baudit\s*\(/,
    );
    expect(sourceText).not.toContain("buildTraceGraph");
    expect(sourceText).not.toContain("--view");
    for (const command of [
      "src/commands/graph/index.ts",
      "src/commands/graph/fan-out.ts",
      "src/commands/graph/change-impact.ts",
      "src/commands/graph/churn.ts",
    ]) {
      expect(readFileSync(join(repoRoot, command), "utf8")).toContain(
        "skipUpdateNudge = true",
      );
    }
  });
});
