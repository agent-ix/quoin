/**
 * `quoin semantic sweep` and the FR-074 promotion guard (issue #293, TASK-041).
 */

import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
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
import { parse as parseYaml, stringify as stringifyYaml } from "yaml";

import SemanticSweep from "../src/commands/semantic/sweep";
import { installPlugin } from "../src/plugins.js";
import { readModuleSemantic } from "../src/semantic/manifest.js";
import { sweepCorpus, type SweepReport } from "../src/semantic/sweep.js";
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
type Json = Record<string, unknown>;

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
  // Trace: FR-074-AC-5
  // Trace: TC-1386
  // Trace: NFR-017-AC-2
  // Trace: TC-1380
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
  // Trace: TC-1386
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

  // Trace: FR-074-AC-3
  // Trace: TC-1369
  it("lets legacy_forms: error through only with a shipped report for the same package and version", () => {
    const withReport = (
      mutate: (report: Json) => void,
      sweepPath = "semantic/sweep.json",
    ) => {
      const module = join(scratch, `m-${Math.random().toString(36).slice(2)}`);
      cpSync(MODULE, module, { recursive: true });
      mkdirSync(join(module, "semantic"), { recursive: true });
      const report = sweepCorpus(
        [{ root: corpus(), repository: "corpus", revision: "worktree" }],
        { package: "agent-ix/spec-objects-fixture", version: "0.1.0" },
      ) as unknown as Json;
      mutate(report);
      writeFileSync(
        join(module, "semantic", "sweep.json"),
        JSON.stringify(report),
      );
      const manifestPath = join(module, "manifest.yaml");
      const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Json;
      (manifest.semantic as Json).legacy_forms = "error";
      (manifest.semantic as Json).sweep_report = sweepPath;
      writeFileSync(manifestPath, stringifyYaml(manifest));
      return readModuleSemantic(module).diagnostics.map((d) => d.code);
    };
    expect(withReport(() => undefined)).not.toContain(
      "semantic.sweep-report-required",
    );
    expect(withReport(() => undefined, "semantic/missing.json")).toContain(
      "semantic.sweep-report-required",
    );
    expect(withReport((r) => (r.package = "agent-ix/other"))).toContain(
      "semantic.sweep-report-required",
    );
    expect(withReport((r) => (r.version = "9.9.9"))).toContain(
      "semantic.sweep-report-required",
    );
    // A report with the right package and version but the wrong shape is
    // rejected by the schema, not by key presence.
    expect(
      withReport((r) => {
        delete (r.counts as Json).forms;
      }),
    ).toContain("semantic.sweep-report-required");
    expect(
      withReport((r) => {
        (r.corpus as Json[]).push({ repository: "x" });
      }),
    ).toContain("semantic.sweep-report-required");
    // The guard runs on the install path and leaves nothing behind.
    const home = join(scratch, "home");
    mkdirSync(home, { recursive: true });
    const unguarded = join(scratch, "unguarded");
    cpSync(MODULE, unguarded, { recursive: true });
    const unguardedManifest = join(unguarded, "manifest.yaml");
    const raw = parseYaml(readFileSync(unguardedManifest, "utf8")) as Json;
    raw.name = "unguarded";
    (raw.semantic as Json).legacy_forms = "error";
    writeFileSync(unguardedManifest, stringifyYaml(raw));
    expect(() => installPlugin(`path:${unguarded}`, home)).toThrow(
      /semantic\.sweep-report-required/,
    );
    const module = join(scratch, "no-report");
    cpSync(MODULE, module, { recursive: true });
    const manifestPath = join(module, "manifest.yaml");
    const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Json;
    (manifest.semantic as Json).legacy_forms = "error";
    writeFileSync(manifestPath, stringifyYaml(manifest));
    expect(readModuleSemantic(module).diagnostics.map((d) => d.code)).toContain(
      "semantic.sweep-report-required",
    );
  });

  // Trace: FR-074-AC-4
  // Trace: TC-1370
  it("shows the migration example once in the authoring pack for a semantic module", () => {
    const catalog = loadCatalog([MODULE]);
    const pack = formatAuthoringPack(
      createAuthoringPack(catalog, scratch, ["entity", "enumeration"]),
    );
    expect(pack.split("Properties migration (FR-074)").length - 1).toBe(1);
    expect(pack).toContain("| Field | Type | Multiplicity | Constraints |");
  });
});
