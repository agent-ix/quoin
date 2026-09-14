import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  readMeasurementCollectionResults,
  type MeasurementCollection,
} from "../src/measurement/index.js";

// Split out of tests/graph-portfolio.test.ts (quoin#480). That file was deleted
// with src/measurement/graph-portfolio.ts, but it held this repository's only
// assertion of readMeasurementCollectionResults — which lives in the retained
// src/measurement/store.ts and is not part of the FR-101 delete set. The
// assertion moves here rather than going out with the module it did not cover.

function collection(id: string, timestamp: string): MeasurementCollection {
  return {
    schemaVersion: 1,
    collectionId: id,
    subject: "fixture",
    scope: { roots: ["src"] },
    toolIdentity: "agent-ix/quire-code-rs",
    toolVersion: "1.0.0",
    configDigest: `sha256:${"c".repeat(64)}`,
    timestamp,
    sourceRevision: "d".repeat(40),
    corpusRevision: "e".repeat(40),
    environment: { runner: "test" },
    observations: [
      {
        metric: "graph_quality",
        planId: "MP-001",
        definitionVersion: "quire-code.graph-quality-v1",
        state: "measured",
        value: 2,
        unit: "count",
        shape: "count",
        population: {
          examined: 2,
          matched: 2,
          complete: true,
          identity: { files: ["a.rs", "b.rs"] },
        },
        dimensions: { measure: "census", dimension: "language", key: "rust" },
      },
    ],
    rawEvidence: {
      producer: { observation_id: `sha256:${id.padEnd(64, "0").slice(0, 64)}` },
      scorer: { digest: `sha256:${"f".repeat(64)}` },
    },
  } as MeasurementCollection;
}

describe("the measurement store's failure-isolating reader", () => {
  // Trace: FR-067-AC-8
  test("reads each collection independently, so one corrupt record hides no sibling", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-store-results-"));
    try {
      const store = join(root, "spec", "evidence", "measurements");
      mkdirSync(store, { recursive: true });
      writeFileSync(
        join(store, "a-new.json"),
        JSON.stringify(collection("a-new", "2026-08-30T23:00:00Z")),
      );
      writeFileSync(
        join(store, "z-old.json"),
        JSON.stringify(collection("z-old", "2026-08-31T00:30:00+02:00")),
      );
      writeFileSync(join(store, "broken.json"), "{not json");

      const reads = readMeasurementCollectionResults(root);
      expect(reads.map((read) => read.path.split("/").at(-1))).toEqual([
        "a-new.json",
        "broken.json",
        "z-old.json",
      ]);
      expect(
        reads.find((read) => read.path.endsWith("broken.json")),
      ).toMatchObject({ error: expect.any(String) });
      expect(
        reads
          .filter((read) => read.collection)
          .map((read) => read.collection?.collectionId),
      ).toEqual(["a-new", "z-old"]);
      expect(
        reads.every((read) => Boolean(read.collection) !== Boolean(read.error)),
      ).toBe(true);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
