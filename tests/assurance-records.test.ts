/** FR-048 — append-only assurance evidence records (TC-1137..TC-1142). */

import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";

import {
  experimentRecordPath,
  listExperimentRecords,
  listOperationalEvidenceRecords,
  readExperimentRecord,
  readOperationalEvidenceRecord,
  writeExperimentRecord,
  writeOperationalEvidenceRecord,
  type ExperimentRecordInput,
  type OperationalEvidenceRecordInput,
  type ProducerProvenance,
} from "../src/evidence/index.js";

const digest = (character: string): string => `sha256:${character.repeat(64)}`;

const provenance: ProducerProvenance = {
  schemaVersion: "producer-provenance-v1",
  identity: "example.invalid/synthetic-runner",
  version: "1.2.3",
  sourceRevision: "0123456789abcdef",
  sourceState: "clean",
  executableDigest: digest("a"),
  configurationDigest: digest("b"),
  capabilities: ["synthetic-experiment-v1"],
  artifacts: [{ name: "raw-result.json", digest: digest("c") }],
};

const experiment: ExperimentRecordInput = {
  schemaVersion: "experiment-record-v1",
  subject: {
    kind: "synthetic-widget",
    id: "widget-900",
    sourceRevision: "0123456789abcdef",
  },
  recordedAt: "2026-08-15T00:00:00.000Z",
  hypothesis: "The bounded recovery path completes within the test window.",
  design: {
    timeBox: "PT30M",
    corpusRefs: ["corpus://synthetic/recovery-v1"],
    comparisonMethod: "Compare observed completion with the declared bound.",
    decisionRule: "Support only when every synthetic case completes.",
  },
  result: {
    status: "supported",
    summary: "Every bounded synthetic case completed.",
    evidenceRefs: ["artifact://raw-result.json"],
  },
  producerProvenance: provenance,
};

const operational: OperationalEvidenceRecordInput = {
  schemaVersion: "operational-evidence-record-v1",
  subject: experiment.subject,
  recordedAt: "2026-08-16T00:00:00.000Z",
  window: {
    startedAt: "2026-08-15T00:00:00.000Z",
    endedAt: "2026-08-16T00:00:00.000Z",
  },
  environment: "synthetic-staging",
  observations: [
    {
      signal: "recovery-duration",
      value: "19",
      unit: "seconds",
      interpretation: "Within the authored 30-second bound.",
      evidenceRefs: ["artifact://telemetry-window.json"],
    },
  ],
  outcome: "within_bounds",
  producerProvenance: {
    ...provenance,
    capabilities: ["synthetic-operational-observation-v1"],
  },
};

let repo: string;

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-assurance-records-"));
});

describe("append-only assurance evidence", () => {
  // Trace: FR-048-AC-1
  // TC-1137
  it("binds a complete producer provenance tuple to each record", () => {
    const stored = writeExperimentRecord(repo, experiment);
    expect(stored.record.producerProvenance).toEqual(provenance);
    expect(stored.record.recordId).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(readExperimentRecord(repo, stored.record.recordId)).toEqual(
      stored.record,
    );
  });

  // Trace: FR-048-AC-2
  // TC-1138
  it("derives identity from canonical content and writes identical input idempotently", () => {
    const first = writeExperimentRecord(repo, experiment);
    const second = writeExperimentRecord(repo, {
      ...experiment,
      design: { ...experiment.design },
    });
    expect(second.record.recordId).toBe(first.record.recordId);
    expect(first.created).toBe(true);
    expect(second.created).toBe(false);
    expect(readFileSync(first.path, "utf8")).toBe(
      readFileSync(second.path, "utf8"),
    );
  });

  // Trace: FR-048-AC-3
  // TC-1139
  it("refuses invalid provenance before publishing any record", () => {
    expect(() =>
      writeExperimentRecord(repo, {
        ...experiment,
        producerProvenance: { ...provenance, executableDigest: "mutable" },
      }),
    ).toThrow("executableDigest must be sha256");
    expect(listExperimentRecords(repo)).toEqual([]);
  });

  // Trace: FR-048-AC-4
  // TC-1140
  it("never overwrites different bytes at an immutable record path", () => {
    const first = writeExperimentRecord(repo, experiment);
    writeFileSync(first.path, "corrupt\n", "utf8");
    expect(() => writeExperimentRecord(repo, experiment)).toThrow(
      "existing bytes were not overwritten",
    );
    expect(readFileSync(first.path, "utf8")).toBe("corrupt\n");
  });

  // Trace: FR-048-AC-5
  // TC-1141
  it("stores operational evidence by content and validates its time window", () => {
    const stored = writeOperationalEvidenceRecord(repo, operational);
    expect(listOperationalEvidenceRecords(repo)).toEqual([
      stored.record.recordId,
    ]);
    expect(readOperationalEvidenceRecord(repo, stored.record.recordId)).toEqual(
      stored.record,
    );
    expect(() =>
      writeOperationalEvidenceRecord(repo, {
        ...operational,
        window: {
          startedAt: operational.window.endedAt,
          endedAt: operational.window.startedAt,
        },
      }),
    ).toThrow("window.endedAt must be after");
  });

  // Trace: FR-048-AC-6
  // TC-1142
  it("detects content tampering on read and keeps run-record storage separate", () => {
    const stored = writeExperimentRecord(repo, experiment);
    const changed = JSON.parse(readFileSync(stored.path, "utf8")) as {
      hypothesis: string;
    };
    changed.hypothesis = "Changed after publication.";
    writeFileSync(stored.path, JSON.stringify(changed), "utf8");
    expect(() => readExperimentRecord(repo, stored.record.recordId)).toThrow(
      "record digest does not match content",
    );
    expect(stored.path).toBe(
      experimentRecordPath(repo, stored.record.recordId),
    );
    expect(stored.path).not.toContain(`${join("runs", "")}`);
    expect(existsSync(join(repo, "spec", "evidence", "runs"))).toBe(false);
  });
});
