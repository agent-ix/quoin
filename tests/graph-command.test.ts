/** `quoin graph` reachable command path (FR-045-AC-9, TC-299). */

import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import Graph from "../src/commands/graph.js";
import { STORE_SCHEMA_VERSION, writeBindings } from "../src/evidence/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

function fakeQuireDir(): string {
  const dir = mkdtempSync(join(tmpdir(), "quoin-graph-quire-"));
  const bin = join(dir, "quire");
  const payload = JSON.stringify({
    unbacked_rows: [],
    status_lies: [],
    untracked_symbols: [],
    groups: [],
    totals: { backed: 1, total: 1 },
    obligations: [
      {
        source: "Acceptance Criteria",
        id: "FR-001-AC-1",
        document: "spec/functional/FR-001.md",
        statement: "The graph reports impact.",
        statement_hash: "a".repeat(64),
      },
    ],
    implements: [
      {
        path: "src/graph.ts",
        symbol: "analyze",
        trace_id: "FR-001",
        form: "ts-implements-comment",
      },
    ],
  });
  writeFileSync(
    bin,
    [
      "#!/bin/sh",
      'if [ "$1" = "--version" ]; then echo "quire 0.42.0"; exit 0; fi',
      "cat <<'PAYLOAD'",
      payload,
      "PAYLOAD",
    ].join("\n"),
  );
  chmodSync(bin, 0o755);
  return dir;
}

describe("TC-299 graph command integrates the authored and evidence graphs", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
    vi.restoreAllMocks();
  });

  // Trace: FR-045-AC-9
  it("renders change impact through the real command surface", async () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-graph-command-"));
    const spec = join(root, "spec", "functional");
    mkdirSync(spec, { recursive: true });
    writeFileSync(
      join(spec, "FR-001.md"),
      [
        "---",
        "id: FR-001",
        "type: FR",
        "relationships: []",
        "---",
        "# Requirement",
      ].join("\n"),
    );
    writeBindings(root, {
      schemaVersion: STORE_SCHEMA_VERSION,
      bindings: [
        {
          obligation: "FR-001-AC-1",
          statementHashAtBinding: "a".repeat(64),
          suite: "unit",
          commit: "1".repeat(40),
          symbols: ["graph test"],
        },
      ],
    });
    process.env.PATH = `${fakeQuireDir()}:${savedPath}`;
    const lines: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      lines.push(String(line)),
    );

    await Graph.run(
      [
        "--repo",
        root,
        "--view",
        "change-impact",
        "--changed",
        "FR-001",
        "--json",
      ],
      config,
    );

    expect(JSON.parse(lines.join("\n"))).toMatchObject({
      view: "change-impact",
      changed: ["FR-001"],
      suspectObligations: ["FR-001-AC-1"],
      affectedSuites: ["unit"],
      affectedImplementations: [
        {
          id: "src/graph.ts#analyze",
          requirements: ["FR-001"],
        },
      ],
      complete: true,
    });
  });
});
