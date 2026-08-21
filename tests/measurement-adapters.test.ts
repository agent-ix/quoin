/** FR-045 — policy-free measurement intake and code-health adapters (TC-290..TC-296). */

import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it } from "vitest";

import EvidenceMeasure from "../src/commands/evidence/measure";
import {
  AdapterError,
  gitHotAgeAdapter,
  gitHotChurnAdapter,
  jscpdAdapter,
  lizardAdapter,
  listMeasurementCollectionPaths,
  measurementRecordsFromCollection,
  normalizedMeasurementAdapter,
  parseDependencyCruiser,
  radonAdapter,
  readMeasurementCollection,
  rustCodeAnalysisAdapter,
} from "../src/evidence/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

function rca(root: string): string {
  return JSON.stringify({
    name: join(root, "src", "lib.rs"),
    spaces: [
      {
        name: "Widget",
        kind: "impl",
        metrics: { cyclomatic: { sum: 99 } },
        spaces: [
          {
            name: "run",
            kind: "function",
            metrics: { cyclomatic: { sum: 7 } },
            spaces: [
              {
                name: "<anonymous>",
                kind: "function",
                metrics: { cyclomatic: { sum: 2 } },
                spaces: [],
              },
            ],
          },
        ],
      },
      {
        name: "top_level",
        kind: "function",
        metrics: { cyclomatic: { sum: 3 } },
        spaces: [],
      },
    ],
  });
}

describe("TC-290 structural adapters avoid unstable symbol identity", () => {
  it("filters Rust aggregates/anonymous nodes and emits a file distribution", () => {
    const result = rustCodeAnalysisAdapter.parse(rca("/repo"), "/repo");
    expect(result.observations).toEqual([
      {
        subject: { kind: "source-file", id: "src/lib.rs" },
        path: "src/lib.rs",
        value: 7,
        unit: "control-flow-path-count",
        distribution: { count: 2, minimum: 3, maximum: 7, mean: 5 },
      },
    ]);
  });

  it("transcribes Lizard CSV and Radon JSON without normalizing their values", () => {
    const lizard = lizardAdapter.parse(
      '10,5,50,2,10,"run@1-10@src/lib.ts","src/lib.ts","run","run(a, b)",1,10\n',
      "/repo",
    );
    expect(lizard.observations[0]).toMatchObject({
      value: 5,
      path: "src/lib.ts",
    });

    const radon = radonAdapter.parse(
      JSON.stringify({
        "/repo/service.py": [
          {
            type: "class",
            name: "Service",
            methods: [{ type: "method", name: "run", complexity: 6 }],
          },
          { type: "function", name: "main", complexity: 2 },
        ],
      }),
      "/repo",
    );
    expect(radon.observations.map((item) => item.subject.id)).toEqual([
      "service.py",
    ]);
    expect(radon.observations[0].distribution).toMatchObject({ count: 2 });
  });

  it("deduplicates Radon's repeated class-method projection but keeps same-named methods in different classes", () => {
    const method = (classname: string, complexity: number) => ({
      type: "method",
      name: "delete",
      classname,
      lineno: classname === "First" ? 10 : 30,
      complexity,
    });
    const first = method("First", 3);
    const second = method("Second", 8);
    const result = radonAdapter.parse(
      JSON.stringify({
        "/repo/service.py": [
          { type: "class", name: "First", methods: [first] },
          first,
          { type: "class", name: "Second", methods: [second] },
          second,
        ],
      }),
      "/repo",
    );
    expect(result.observations[0]).toMatchObject({
      value: 8,
      distribution: { count: 2, minimum: 3, maximum: 8, mean: 5.5 },
    });
  });
});

describe("TC-291 churn dimensions remain separate", () => {
  it("does not multiply churn and age into a score", () => {
    const raw = "5,12,src/a.ts\n2,40,src/b.ts\n";
    expect(gitHotChurnAdapter.parse(raw, "/repo").observations).toMatchObject([
      { value: 5, unit: "live-line-change-count" },
      { value: 2, unit: "live-line-change-count" },
    ]);
    expect(gitHotAgeAdapter.parse(raw, "/repo").observations).toMatchObject([
      { value: 12, unit: "days" },
      { value: 40, unit: "days" },
    ]);
  });
});

describe("TC-292 clone pairs keep pair and fragment identity", () => {
  it("emits one observation per pair rather than a repository grade", () => {
    const raw = JSON.stringify({
      duplicates: [
        {
          firstFile: { name: "src/b.ts" },
          secondFile: { name: "src/a.ts" },
          fragment: "const shared = true;",
          lines: 8,
        },
      ],
    });
    const observation = jscpdAdapter.parse(raw, "/repo").observations[0];
    expect(observation).toMatchObject({
      subject: { kind: "clone-pair" },
      value: 8,
      unit: "duplicated-lines",
    });
    expect(observation.subject.id).toMatch(
      /^src\/a\.ts\|src\/b\.ts#[a-f0-9]{16}$/,
    );
  });
});

describe("TC-293 collection uncertainty fails closed", () => {
  it("preserves normalized limitations for the command to reject", () => {
    const result = normalizedMeasurementAdapter.parse(
      JSON.stringify({
        observations: [],
        limitations: ["parser skipped macros.rs"],
      }),
      "/repo",
    );
    expect(result.limitations).toEqual(["parser skipped macros.rs"]);
  });

  it("aggregates repeated fallback names and rejects paths outside the repository", () => {
    expect(() =>
      rustCodeAnalysisAdapter.parse(rca("/outside"), "/repo"),
    ).toThrow(AdapterError);
    const duplicate =
      '1,1,1,0,1,"a@1@src/a.ts","src/a.ts","a","a()",1,1\n' +
      '1,1,1,0,1,"a@2@src/a.ts","src/a.ts","a","a()",2,2\n';
    expect(
      lizardAdapter.parse(duplicate, "/repo").observations[0],
    ).toMatchObject({
      distribution: { count: 2 },
    });
    expect(() =>
      normalizedMeasurementAdapter.parse(
        JSON.stringify({
          observations: [
            { subject: { kind: "file", id: "src/a" }, value: 1, unit: "x" },
            { subject: { kind: "file", id: "src/a" }, value: 2, unit: "x" },
          ],
        }),
        "/repo",
      ),
    ).toThrow(/duplicate stable subject/);
  });
});

describe("TC-294 dependency scans cannot be vacuously clean", () => {
  const ruleSetUsed = {
    forbidden: [{ name: "no-cycles", severity: "error", from: {}, to: {} }],
  };

  it("maps a project-owned rule edge and records evaluated rule identity", () => {
    const result = parseDependencyCruiser(
      JSON.stringify({
        summary: {
          totalCruised: 4,
          ruleSetUsed,
          environment: { version: "18.2.0" },
          violations: [
            {
              type: "cycle",
              from: "src/a.ts",
              to: "src/b.ts",
              rule: { name: "no-cycles", severity: "error" },
            },
          ],
        },
      }),
    );
    expect(result.tool).toBe("dependency-cruiser 18.2.0");
    expect(result.rulesEvaluated).toBe(1);
    expect(result.findings[0]).toMatchObject({
      ruleId: "no-cycles",
      path: "src/a.ts",
    });
  });

  it("rejects zero traversed modules and zero rules", () => {
    expect(() =>
      parseDependencyCruiser(
        JSON.stringify({
          summary: { totalCruised: 0, ruleSetUsed, violations: [] },
        }),
      ),
    ).toThrow(/zero modules/);
    expect(() =>
      parseDependencyCruiser(
        JSON.stringify({
          summary: { totalCruised: 4, ruleSetUsed: {}, violations: [] },
        }),
      ),
    ).toThrow(/no dependency rules/);
  });
});

describe("TC-295 the command transcribes a complete batch atomically", () => {
  function root(): string {
    return mkdtempSync(join(tmpdir(), "quoin-measure-adapter-"));
  }

  function args(repo: string, resultPath: string): string[] {
    return [
      "--repo",
      repo,
      "--plan",
      "MP-001",
      "--definition",
      "rust-structure-v1",
      "--repository",
      "agent-ix/service",
      "--revision",
      "a".repeat(40),
      "--tool",
      "rust-code-analysis",
      "--tool-version",
      "0.0.25",
      "--configuration-digest",
      `sha256:${"b".repeat(64)}`,
      "--environment",
      "linux-x64",
      "--attribute",
      "language=rust",
      "--adapter",
      "rust-code-analysis-cyclomatic-file-distribution",
      "--expected-count",
      "1",
      "--timestamp",
      "2026-08-21T12:00:00Z",
      "--results",
      resultPath,
    ];
  }

  it("writes canonical records only after every observation validates", async () => {
    const repo = root();
    const input = join(repo, "metrics.jsonl");
    writeFileSync(input, rca(repo));
    await EvidenceMeasure.run(args(repo, input), config);

    const paths = listMeasurementCollectionPaths(repo, "MP-001");
    expect(paths).toHaveLength(1);
    const stored = readMeasurementCollection(paths[0]);
    const records = measurementRecordsFromCollection(stored!);
    expect(records.map((record) => record.subject.id)).toEqual(["src/lib.rs"]);
    expect(records[0].distribution).toMatchObject({ count: 2, maximum: 7 });
    expect(records[0].environment.attributes).toEqual({ language: "rust" });
    expect(records[0].rawEvidence.digest).toMatch(/^sha256:[a-f0-9]{64}$/);
    expect(readFileSync(paths[0], "utf8")).toMatch(/\n$/);
  });

  it("writes nothing when expected population disagrees", async () => {
    const repo = root();
    const input = join(repo, "metrics.jsonl");
    writeFileSync(input, rca(repo));
    const mismatched = args(repo, input);
    mismatched[mismatched.indexOf("1")] = "3";
    await expect(EvidenceMeasure.run(mismatched, config)).rejects.toThrow(
      /expected 3 observations, parsed 1/,
    );
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  it("writes no file or temporary residue when a late observation is invalid", async () => {
    const repo = root();
    const input = join(repo, "normalized.json");
    writeFileSync(
      input,
      JSON.stringify({
        observations: [
          { subject: { kind: "file", id: "src/a" }, value: 1, unit: "count" },
          {
            subject: { kind: "file", id: "src/b" },
            value: 2,
            unit: "count",
            distribution: { count: 0, mean: 2 },
          },
        ],
      }),
    );
    const normalized = args(repo, input);
    normalized[
      normalized.indexOf("rust-code-analysis-cyclomatic-file-distribution")
    ] = "observations";
    normalized[normalized.indexOf("1")] = "2";
    await expect(EvidenceMeasure.run(normalized, config)).rejects.toThrow(
      /measurement-collection-v1/,
    );
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });
});

describe("TC-296 normalized input stays an open extension seam", () => {
  it("accepts a producer unknown to Quoin without adding engine logic", () => {
    const result = normalizedMeasurementAdapter.parse(
      JSON.stringify({
        observations: [
          {
            subject: { kind: "module", id: "src/core" },
            value: 4,
            unit: "edges",
          },
        ],
      }),
      "/repo",
    );
    expect(result.observations[0]).toEqual({
      subject: { kind: "module", id: "src/core" },
      value: 4,
      unit: "edges",
    });
  });
});
