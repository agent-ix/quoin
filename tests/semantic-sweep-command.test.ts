/**
 * `quoin semantic sweep` and the authoring pack it feeds (issue #293,
 * TASK-041).
 *
 * The sweep itself is Rust from quoin#452 on: `sweepCorpus` here is the thin
 * `quoin-core semantic.sweep_corpus` caller, and what this file proves is that
 * the command and the authoring pack still compose over it. The manifest-side
 * FR-074-AC-3 promotion guard moved with the engine —
 * `rust/crates/quoin-semantic/tests/tc_452_sweep_criteria.rs` and
 * `rust/crates/quoin-core/tests/tc_452_module_install_criteria.rs` restate it.
 */

import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import {
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

import SemanticSweep from "../src/commands/semantic/sweep";
import { sweepCorpus, type SweepReport } from "../src/core/semantic.js";
import { createAuthoringPack, formatAuthoringPack } from "../src/write.js";
import { loadCatalog } from "../src/catalog.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const MAPPING = join(
  repoRoot,
  "tests",
  "fixtures",
  "semantic-module",
  "mapping",
);
const MODULE = join(
  repoRoot,
  "tests",
  "fixtures",
  "semantic-module",
  "module-ok",
);

let config: Config;
let scratch: string;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "quoin-sweep-"));
});

afterEach(() => {
  rmSync(scratch, { recursive: true, force: true });
});

function reportValidator() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  return ajv.compile(
    JSON.parse(
      readFileSync(
        join(repoRoot, "src", "semantic", "sweep-report.schema.json"),
        "utf8",
      ),
    ),
  );
}

function corpus(): string {
  const root = join(scratch, "corpus");
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  for (const name of [
    "config-version.table.md",
    "legacy-bullets.md",
    "legacy-mixed.md",
    "config-version.fence.md",
  ])
    cpSync(join(MAPPING, name), join(root, "spec", "functional", name));
  cpSync(
    join(
      MAPPING,
      "..",
      "corpus",
      "config-service",
      "FR-006-config-version-entity.md",
    ),
    join(root, "spec", "functional", "FR-006.md"),
  );
  return root;
}

describe("FR-074 sweep report", () => {
  // Trace: FR-074-AC-5, NFR-017-AC-2
  it("produces a schema-valid report that counts each legacy form and only warning-severity findings", () => {
    const root = corpus();
    const report = sweepCorpus(
      [{ root, repository: "corpus", revision: "worktree" }],
      { package: "agent-ix/spec-objects-fixture", version: "0.1.0" },
    );
    expect(reportValidator()(report), "report validates").toBe(true);
    expect(report.counts.artifacts).toBe(5);
    expect(report.counts.legacy).toEqual({
      "bullet-list": 2,
      "free-column-table": 1,
    });
    expect(report.counts.forms["typed-table"]).toBe(1);
    expect(report.counts.forms["sysml-fence"]).toBe(1);
    const severities = new Set(
      report.findings.map((f) => f.diagnostic?.severity).filter(Boolean),
    );
    expect([...severities]).toEqual(["warning"]);
  });

  // Trace: FR-074-AC-5
  it("runs through the command, writing the report to --out inside a corpus root", async () => {
    const root = corpus();
    const out = join(root, "sweep.json");
    const lines: string[] = [];
    const spy = vi.spyOn(console, "log").mockImplementation((message) => {
      lines.push(String(message));
    });
    try {
      await SemanticSweep.run(
        [
          "--package",
          "agent-ix/spec-objects-fixture",
          "--module-version",
          "0.1.0",
          "--out",
          out,
          root,
        ],
        config,
      );
    } finally {
      spy.mockRestore();
    }
    const report = JSON.parse(readFileSync(out, "utf8")) as SweepReport;
    expect(reportValidator()(report)).toBe(true);
    expect(report.corpus).toEqual([
      { repository: "corpus", revision: "worktree" },
    ]);
    expect(lines.join("\n")).toContain("5 artifacts");
    expect(Object.keys(SemanticSweep.flags).sort()).toEqual([
      "module-version",
      "out",
      "package",
    ]);
    // --out outside the working directory and every corpus root is refused.
    const outside = join(scratch, "elsewhere.json");
    await expect(
      SemanticSweep.run(
        [
          "--package",
          "agent-ix/spec-objects-fixture",
          "--module-version",
          "0.1.0",
          "--out",
          outside,
          root,
        ],
        config,
      ),
    ).rejects.toThrow(/outside the working directory/);
  });

  // Trace: FR-074-AC-4, FR-070-AC-2
  it("shows the migration example once in the authoring pack and names the module's semantic identity", () => {
    const catalog = loadCatalog([MODULE]);
    const pack = formatAuthoringPack(
      createAuthoringPack(catalog, scratch, ["entity", "enumeration"]),
    );
    expect(pack.split("Properties migration (FR-074)").length - 1).toBe(1);
    expect(pack).toContain("| Field | Type | Multiplicity | Constraints |");
    // FR-070-AC-2: the block a module declares reaches the authoring pack,
    // which is the only place a user sees it. `tests/semantic-manifest.test.ts`
    // carried this against `readModuleSemantic` directly; it is restated here
    // because the pack is the TypeScript-side consumer of the parsed block,
    // and the parse itself is now `tc_452_611`.
    expect(pack).toContain(
      "semantic: agent-ix/spec-objects-fixture (semantic-core 0.1.0)",
    );
    expect(pack).toContain("data_schema: schemas/Entity.json");
  });
});
