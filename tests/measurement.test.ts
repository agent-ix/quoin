import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  buildMeasurementReport,
  comparisonFor,
  compareMeasurementCollections,
  readMeasurementCollections,
  renderMeasurementComparison,
  renderMeasurementReport,
  validateMeasurementCollection,
  writeMeasurementCollection,
  type MeasurementCollection,
} from "../src/measurement/index.js";

const PLAN = `---
id: MP-001
title: Example metric
type: MeasurementPlan
status: active
owner: test
stage: branch-comparison
metric: quality.example
definition_version: quality.example-v1
---

# Example metric

## Decision Objective
Test.
## Population and Scope
Test.
## Measure Definition
Test.
## Collection and Provenance
Test.
## Environment and Sampling
Test.
## Interpretation and Limitations
Test.
## Comparison and Enforcement
Test.
`;

describe("generic MeasurementRecords", () => {
  const roots: string[] = [];

  afterEach(() => {
    for (const root of roots.splice(0))
      rmSync(root, { recursive: true, force: true });
  });

  function repo(withPlan = true): string {
    const root = mkdtempSync(join(tmpdir(), "quoin-measurement-"));
    roots.push(root);
    if (withPlan) {
      mkdirSync(join(root, "spec", "assurance"), { recursive: true });
      writeFileSync(join(root, "spec", "assurance", "MP-001.md"), PLAN);
    }
    return root;
  }

  function collection(
    over: Partial<MeasurementCollection> = {},
  ): MeasurementCollection {
    return {
      schemaVersion: 2,
      collectionId: "run-001",
      subject: "fixture",
      scope: { cases: 1 },
      toolIdentity: "fixture producer",
      toolVersion: "fixture 1 (engine a1)",
      configDigest: "sha256:config-a",
      timestamp: "2026-08-26T00:00:00.000Z",
      sourceRevision: "aaaaaaaaaaaaaaaa",
      corpusRevision: "cccccccccccccccc",
      environment: { runner: "test" },
      verificationStack: {
        schemaVersion: "verification-stack-attestation-v1",
        lockDigest: `sha256:${"1".repeat(64)}`,
        executableDigest: `sha256:${"2".repeat(64)}`,
        buildProfile: "release",
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
          metric: "quality.example",
          planId: "MP-001",
          definitionVersion: "quality.example-v1",
          state: "measured",
          value: 0.5,
          unit: "fraction",
          shape: "ratio",
          population: {
            examined: 2,
            matched: 1,
            complete: true,
            identity: ["a", "b"],
          },
        },
      ],
      rawEvidence: { payload: [1, 2] },
      ...over,
    };
  }

  test("TC-1003 refuses an observation with no authored MeasurementPlan", () => {
    expect(() => validateMeasurementCollection(collection(), [])).toThrow(
      /has no MeasurementPlan.*record refused/,
    );
  });

  test("schema-v2 refuses an attestation with no toolchain identities", () => {
    const drifted = collection();
    (
      drifted.verificationStack as unknown as Record<string, unknown>
    ).toolchains = undefined;
    expect(() => validateMeasurementCollection(drifted, [])).toThrow(
      /toolchains must pin node, rust, and python/,
    );
  });

  test("schema-v2 refuses a noncanonical executable build profile", () => {
    const drifted = collection();
    (
      drifted.verificationStack as unknown as Record<string, unknown>
    ).buildProfile = "debug";
    expect(() => validateMeasurementCollection(drifted, [])).toThrow(
      /buildProfile must be release/,
    );
  });

  test("TC-1004 writes one complete collection atomically and reads the same bytes", () => {
    const root = repo();
    const path = writeMeasurementCollection(root, collection());
    expect(readMeasurementCollections(root)).toEqual([collection()]);
    const before = readFileSync(path, "utf8");
    expect(writeMeasurementCollection(root, collection())).toBe(path);
    expect(readFileSync(path, "utf8")).toBe(before);
  });

  test("TC-1005 refuses unlike definitions/configs and names both causes", () => {
    const before = collection();
    const after = collection({
      collectionId: "run-002",
      configDigest: "sha256:config-b",
      observations: [
        {
          ...before.observations[0],
          definitionVersion: "quality.example-v2",
          value: 0.75,
        },
      ],
    });
    const [result] = compareMeasurementCollections(before, after);
    expect(result.status).toBe("incomparable");
    expect(result.delta).toBeNull();
    expect(result.reasons.map((reason) => reason.code)).toEqual([
      "definition_changed",
      "configuration_changed",
    ]);
    expect(result.reasons.map((reason) => reason.message).join(" ")).toMatch(
      /quality\.example.*config/,
    );
  });

  test("TC-1006 reports population movement beside a delta and never assigns severity", () => {
    const before = collection();
    const after = collection({
      collectionId: "run-002",
      observations: [
        {
          ...before.observations[0],
          value: 0.75,
          population: {
            examined: 4,
            matched: 3,
            complete: true,
            identity: ["a", "b", "c", "d"],
          },
        },
      ],
    });
    const [result] = compareMeasurementCollections(before, after);
    expect(result).toMatchObject({ status: "comparable", delta: 0.25 });
    expect(result.reasons.map((reason) => reason.code)).toContain(
      "population_changed",
    );
    expect(result).not.toHaveProperty("verdict");
    expect(result).not.toHaveProperty("severity");
  });

  test("TC-1007 a missing metric is not_computed, not an unchanged zero", () => {
    const before = collection();
    const after = collection({ collectionId: "run-002", observations: [] });
    const [result] = compareMeasurementCollections(before, after);
    expect(result).toMatchObject({
      status: "not_computed",
      before: 0.5,
      after: null,
      delta: null,
    });
  });

  test("TC-1008 report is byte-identical and keeps a plan with no record visible", () => {
    const root = repo();
    const first = renderMeasurementReport(buildMeasurementReport(root));
    expect(renderMeasurementReport(buildMeasurementReport(root))).toBe(first);
    expect(first).toContain("quality.example");
    expect(first).toContain("not_computed: no record");
    expect(first).toContain("Corpus gaps: not_computed");
    const comparison = renderMeasurementComparison({
      status: "compared",
      reason: null,
      before: {
        collectionId: "run-001",
        timestamp: "2026-08-26T00:00:00Z",
        toolIdentity: "fixture",
        toolVersion: "1",
        configDigest: "config-a",
        sourceRevision: "before",
        corpusRevision: "corpus-a",
        corpusGaps: 0,
        path: "/repo/run-001.json",
      },
      after: {
        collectionId: "run-002",
        timestamp: "2026-08-26T01:00:00Z",
        toolIdentity: "fixture",
        toolVersion: "2",
        configDigest: "config-a",
        sourceRevision: "after",
        corpusRevision: "corpus-a",
        corpusGaps: 0,
        path: "/repo/run-002.json",
      },
      comparisons: [
        {
          metric: "quality.example",
          dimensions: {},
          before: 0.5,
          after: 0.75,
          delta: 0.25,
          status: "comparable",
          reasons: [],
        },
      ],
    });
    expect(comparison).toContain("# QA measurement comparison");
    expect(comparison).toContain(
      "| quality.example | comparable | 0.5 | 0.75 | 0.25 |",
    );
    expect(comparison).toContain(
      "fixture 2; source after; corpus corpus-a; gaps 0; config config-a",
    );
  });

  test("TC-1009 a missing baseline is rendered as not_computed with current provenance", () => {
    const root = repo();
    writeMeasurementCollection(root, collection());
    const report = comparisonFor(root, "missing");
    expect(report).toMatchObject({
      status: "not_computed",
      reason: "no baseline measurement collection for source revision missing",
      before: null,
      after: {
        collectionId: "run-001",
        toolIdentity: "fixture producer",
        corpusRevision: "cccccccccccccccc",
      },
      comparisons: [],
    });
    expect(renderMeasurementComparison(report)).toContain(
      "Before: not_computed — no baseline",
    );
  });
});
