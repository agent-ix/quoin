import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";

import { loadConfig } from "@agent-ix/ix-cli-core";

import ReportCommand from "../src/commands/report.js";

import {
  buildPortfolioReport,
  renderPortfolioReport,
  renderPortfolioReportJson,
  writeMeasurementCollection,
  type MeasurementCollection,
} from "../src/measurement/index.js";

const PROFILE = `---
id: AP-001
title: Fixture assurance
type: AssuranceProfile
status: active
---

# Fixture assurance
`;

const plan = (definition: string) => `---
id: MP-001
title: Fixture quality
type: MeasurementPlan
status: active
stage: branch-comparison
metric: quality.fixture
definition_version: ${definition}
---

# Fixture quality
`;

describe("portfolio measurement report", () => {
  const roots: string[] = [];
  const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
  let config: Config;

  beforeAll(async () => {
    config = await loadConfig({ root: projectRoot });
  });

  afterEach(() => {
    for (const root of roots.splice(0))
      rmSync(root, { recursive: true, force: true });
  });

  function repo(name: string, withPlan = true): string {
    const parent = mkdtempSync(join(tmpdir(), "quoin-portfolio-"));
    roots.push(parent);
    const root = join(parent, name);
    mkdirSync(join(root, "spec", "assurance"), { recursive: true });
    if (withPlan) {
      writeFileSync(
        join(root, "spec", "assurance", "MP-001.md"),
        plan("quality.fixture-v1"),
      );
    }
    return root;
  }

  function collection(
    id: string,
    timestamp: string,
    definition = "quality.fixture-v1",
    value = 0.5,
  ): MeasurementCollection {
    return {
      schemaVersion: 2,
      collectionId: id,
      subject: "fixture",
      scope: { cases: 2 },
      toolIdentity: "fixture-producer",
      toolVersion: "fixture 1",
      configDigest: "sha256:fixture-config",
      timestamp,
      sourceRevision: `${id}-source`,
      corpusRevision: `${id}-corpus`,
      environment: { runner: "test" },
      verificationStack: {
        schemaVersion: "verification-stack-attestation-v1",
        lockDigest: `sha256:${"1".repeat(64)}`,
        executableDigest: `sha256:${"2".repeat(64)}`,
        toolchains: { node: "22.15.0", rust: "1.94.1", python: "3.10.12" },
        sources: {
          fixture: {
            revision: "a".repeat(40),
            sourceState: "clean",
            remote: "https://example.invalid/fixture",
          },
        },
        capabilities: ["fixture.capability"],
        artifacts: { config: `sha256:${"3".repeat(64)}` },
      },
      observations: [
        {
          metric: "quality.fixture",
          planId: "MP-001",
          definitionVersion: definition,
          state: "measured",
          value,
          unit: "fraction",
          shape: "ratio",
          dimensions: { family: "fixture" },
          population: {
            examined: 4,
            matched: Math.round(value * 4),
            complete: true,
            identity: ["a", "b", "c", "d"],
          },
        },
      ],
      rawEvidence: { bounds: { gap_count: 3 } },
    };
  }

  test("TC-1011 mixed definitions, missing stores, unreadable input and one stale repository stay distinct", () => {
    const recent = repo("recent");
    writeFileSync(join(recent, "spec", "assurance", "AP-001.md"), PROFILE);
    writeMeasurementCollection(
      recent,
      collection("run-old", "2026-06-01T00:00:00.000Z"),
    );
    writeFileSync(
      join(recent, "spec", "assurance", "MP-001.md"),
      plan("quality.fixture-v2"),
    );
    writeMeasurementCollection(
      recent,
      collection(
        "run-new",
        "2026-08-26T00:00:00.000Z",
        "quality.fixture-v2",
        0.75,
      ),
    );

    const stale = repo("stale");
    writeMeasurementCollection(
      stale,
      collection("run-stale", "2026-06-01T00:00:00.000Z"),
    );
    const noStore = repo("no-store");
    const missing = join(repo("missing-parent", false), "does-not-exist");
    const unreadable = repo("unreadable");
    const unreadableRoot = join(unreadable, "spec", "evidence", "measurements");
    mkdirSync(unreadableRoot, { recursive: true });
    writeFileSync(join(unreadableRoot, "broken.json"), "<<<<<<< HEAD\n");

    const report = buildPortfolioReport([
      stale,
      recent,
      noStore,
      missing,
      unreadable,
    ]);
    const byName = Object.fromEntries(
      report.repositories.map((repository) => [repository.name, repository]),
    );

    expect(byName.recent).toMatchObject({
      status: "readable",
      store: "present",
      staleness: { status: "current", ageDays: 0 },
    });
    expect(byName.recent.profiles.map((profile) => profile.id)).toEqual([
      "AP-001",
    ]);
    expect(byName.recent.comparison?.observations[0]).toMatchObject({
      status: "incomparable",
      before: 0.5,
      after: 0.75,
    });
    expect(
      byName.recent.comparison?.observations[0].reasons.map(
        (reason) => reason.code,
      ),
    ).toContain("definition_changed");
    expect(byName.stale.staleness).toMatchObject({
      status: "stale",
      ageDays: 86,
    });
    expect(byName["no-store"]).toMatchObject({
      status: "readable",
      store: "missing",
      latestCollection: null,
      staleness: { status: "not_computed" },
    });
    expect(byName["does-not-exist"]).toMatchObject({
      root: missing,
      status: "missing",
      store: "missing",
    });
    expect(byName.unreadable).toMatchObject({
      status: "unreadable",
      store: "unreadable",
    });
    expect(byName.unreadable.error).toMatch(/broken\.json.*unreadable/);
    const human = renderPortfolioReport(report);
    expect(human).toContain("quality.fixture [family=fixture]");
    expect(human).toContain(
      "definition_changed: quality.fixture: definition moved quality.fixture-v1 -> quality.fixture-v2",
    );
  });

  test("TC-1012 human and JSON views share one report, link values, and invent no aggregate", () => {
    const measured = repo("measured");
    writeMeasurementCollection(
      measured,
      collection("run-001", "2026-08-26T00:00:00.000Z", undefined, 0.75),
    );
    const report = buildPortfolioReport([measured]);
    const human = renderPortfolioReport(report);
    const json = renderPortfolioReportJson(report);

    expect(JSON.parse(json)).toEqual(report);
    expect(renderPortfolioReport(report)).toBe(human);
    expect(human).toContain(
      "| quality.fixture [family=fixture] | 0.75 fraction | MP-001 (spec/assurance/MP-001.md) |",
    );
    expect(human).toMatch(/run-001 \(.+run-001\.json\)/);
    expect(human).toContain("Corpus gaps: 3");
    expect(human).toContain("Provenance: source run-001-source");
    expect(human).toContain("No cross-repository metric is summed, averaged");
    expect(human).not.toMatch(/overall quality|portfolio quality|aggregate %/i);
  });

  test("TC-1013 one report command accepts repeated repository locations and no values", async () => {
    const first = repo("first");
    const second = repo("second");
    const output: string[] = [];
    const log = vi
      .spyOn(console, "log")
      .mockImplementation((value) => output.push(String(value)));
    try {
      await ReportCommand.run(
        ["--portfolio", first, "--portfolio", second, "--format", "json"],
        config,
      );
    } finally {
      log.mockRestore();
    }
    const value = JSON.parse(output.join("\n"));
    expect(
      value.repositories
        .map((repository: { name: string }) => repository.name)
        .sort(),
    ).toEqual(["first", "second"]);
    expect(Object.keys(ReportCommand.flags)).not.toContain("value");
  });

  test("TC-1014 a corpus-oriented root assurance directory is a governed store", () => {
    const root = repo("root-assurance", false);
    const rootAssurance = join(root, "assurance");
    mkdirSync(rootAssurance, { recursive: true });
    writeFileSync(join(rootAssurance, "AP-001.md"), PROFILE);
    writeFileSync(join(rootAssurance, "MP-001.md"), plan("quality.fixture-v1"));
    writeMeasurementCollection(
      root,
      collection("root-run", "2026-08-27T00:00:00.000Z"),
    );

    const repository = buildPortfolioReport([root]).repositories[0];
    expect(repository.profiles).toEqual([
      expect.objectContaining({ id: "AP-001", path: "assurance/AP-001.md" }),
    ]);
    expect(repository.plans).toEqual([
      expect.objectContaining({ id: "MP-001", path: "assurance/MP-001.md" }),
    ]);
    expect(repository.latestCollection?.id).toBe("root-run");
  });
});
