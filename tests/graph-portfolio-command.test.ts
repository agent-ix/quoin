/** Report-command integration for governed graph portfolio wiring. */

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, expect, test, vi } from "vitest";

import ReportCommand from "../src/commands/report.js";
import {
  STORE_SCHEMA_VERSION,
  writeBindings,
} from "./support/evidence-store.js";

/**
 * The relationship vocabulary an export has to declare, inlined for the same
 * reason as in `graph-command.test.ts` (quoin#500).
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

// The byte-for-byte embedding half of TC-1311 used the retained TypeScript as
// its own oracle, so it could not survive that TypeScript's deletion (quoin#500)
// and could not be restated in terms of it either. Its criterion already has a
// Rust home that does not compare an implementation with itself:
// `quoin-measurement-graph` `tc_480_fr067_criteria.rs` :: `tc_480_043`, which
// asserts the embedded objects against the portfolio's own contract. What is
// left here is the half that is genuinely about this command: the argument
// check that runs before any read.

test("partial triples are incompatible before a missing export is read", async () => {
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
