/** Command boundary for FR-048-AC-7 (TC-1143). */

import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { expect, it } from "vitest";

const root = resolve(import.meta.dirname, "..");
const digest = (character: string): string => `sha256:${character.repeat(64)}`;
const provenance = {
  schemaVersion: "producer-provenance-v1",
  identity: "example.invalid/command-fixture",
  version: "1.0.0",
  sourceRevision: "0123456789abcdef",
  sourceState: "clean",
  executableDigest: digest("a"),
  configurationDigest: digest("b"),
  capabilities: ["fixture-v1"],
  artifacts: [],
};
const subject = {
  kind: "synthetic-widget",
  id: "widget-command",
  sourceRevision: "0123456789abcdef",
};

function run(args: string[], input?: string): string {
  return execFileSync("node", [join(root, "bin", "quoin.js"), ...args], {
    cwd: root,
    encoding: "utf8",
    input,
    env: { ...process.env, CI: "1", NODE_ENV: "production" },
  });
}

// Trace: FR-048-AC-7
// TC-1143
it("records experiments from stdin and operational evidence from a file", () => {
  const repo = mkdtempSync(join(tmpdir(), "quoin-assurance-command-"));
  const experiment = {
    schemaVersion: "experiment-record-v1",
    subject,
    recordedAt: "2026-08-15T00:00:00.000Z",
    hypothesis: "The bounded synthetic path completes.",
    design: {
      timeBox: "PT5M",
      corpusRefs: ["corpus://synthetic/command"],
      comparisonMethod: "Compare to the declared bound.",
      decisionRule: "Support only when every case completes.",
    },
    result: {
      status: "supported",
      summary: "Every case completed.",
      evidenceRefs: ["artifact://experiment.json"],
    },
    producerProvenance: provenance,
  };
  const experimentResult = JSON.parse(
    run(
      [
        "evidence",
        "record-experiment",
        "--repo",
        repo,
        "--input",
        "-",
        "--json",
      ],
      JSON.stringify(experiment),
    ),
  ) as { created: boolean; path: string; record: { recordId: string } };
  expect(experimentResult.created).toBe(true);
  expect(experimentResult.record.recordId).toMatch(/^sha256:/);

  const operational = {
    schemaVersion: "operational-evidence-record-v1",
    subject,
    recordedAt: "2026-08-16T00:00:00.000Z",
    window: {
      startedAt: "2026-08-15T00:00:00.000Z",
      endedAt: "2026-08-16T00:00:00.000Z",
    },
    environment: "synthetic-staging",
    observations: [
      {
        signal: "completion",
        value: "complete",
        interpretation: "The bounded path completed.",
        evidenceRefs: ["artifact://operational.json"],
      },
    ],
    outcome: "within_bounds",
    producerProvenance: provenance,
  };
  const inputPath = join(repo, "operational.json");
  writeFileSync(inputPath, JSON.stringify(operational), "utf8");
  const operationalResult = JSON.parse(
    run([
      "evidence",
      "record-operational",
      "--repo",
      repo,
      "--input",
      inputPath,
      "--json",
    ]),
  ) as { created: boolean; path: string; record: { recordId: string } };
  expect(operationalResult.created).toBe(true);
  expect(operationalResult.path).toContain("/operational/sha256-");
});
