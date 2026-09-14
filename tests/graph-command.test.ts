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
  STORE_SCHEMA_VERSION,
  writeBindings,
} from "./support/evidence-store.js";

/**
 * The relationship vocabulary an export has to declare for the walk to run.
 *
 * Inlined rather than imported: `src/graph-analysis/` is gone (quoin#500) and
 * the constant it exported is now `quoin_graph_analysis::DEFAULT_RELATION_KINDS`,
 * on the far side of the `quoin-core` boundary. This is fixture data for a
 * command test, and the command tests here assert nothing about its contents —
 * that the two lists agree is `quoin-graph-analysis`'s own
 * `tc_500_restated_criteria.rs`, not this file's.
 */
const DEFAULT_RELATION_KINDS = [
  "depends_on",
  "derives_from",
  "implements",
  "mitigates",
  "refines",
  "requires",
  "satisfies",
  "traces_to",
];

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

  // Trace: FR-062-AC-11
  it("keeps producers, writes, frontmatter, and a second graph outside every view", () => {
    // The analysis half of this criterion moved to `quoin-graph-analysis`
    // (quoin#500) and is censused there by `tc_500_restated_criteria.rs` ::
    // `tc_500_047`, over that crate's whole `src/` tree rather than a list.
    // What is left here is the command half: the five files that ask
    // `quoin-core` for a report and print it.
    const paths = [
      "src/commands/graph/common.ts",
      "src/commands/graph/index.ts",
      "src/commands/graph/fan-out.ts",
      "src/commands/graph/change-impact.ts",
      "src/commands/graph/churn.ts",
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
