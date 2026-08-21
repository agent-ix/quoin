/** FR-043 — a generic, policy-free measurement record (TC-277..TC-282). */

import {
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { beforeEach, describe, expect, it } from "vitest";

import {
  measurementCollectionPath,
  measurementRecordsFromCollection,
  readMeasurementCollectionSchema,
  readMeasurementCollection,
  readMeasurementSchema,
  validateMeasurementRecord,
  writeMeasurementCollection,
  type MeasurementCollection,
  type MeasurementRecord,
} from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const fixtures = join(here, "fixtures", "evidence");
const latency = JSON.parse(
  readFileSync(join(fixtures, "measurement-cli-latency.json"), "utf8"),
) as MeasurementRecord;
const complexity = JSON.parse(
  readFileSync(join(fixtures, "measurement-function-complexity.json"), "utf8"),
) as MeasurementRecord;

let repo: string;

function collection(...records: MeasurementRecord[]): MeasurementCollection {
  const [first] = records;
  return {
    schemaVersion: 1,
    plan: first.plan,
    repository: first.scope.repository,
    sourceRevision: first.sourceRevision,
    tool: first.tool,
    environment: first.environment,
    ...(first.sampling === undefined ? {} : { sampling: first.sampling }),
    collectedAt: first.collectedAt,
    rawEvidence: first.rawEvidence,
    observations: records.map((record) => ({
      subject: record.subject,
      ...(record.scope.path === undefined ? {} : { path: record.scope.path }),
      value: record.value,
      unit: record.unit,
      ...(record.distribution === undefined
        ? {}
        : { distribution: record.distribution }),
    })),
  };
}

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-measurement-"));
});

describe("TC-277 one measure-neutral record shape covers unlike observations", () => {
  // Trace: FR-043-AC-1
  it("accepts a CLI latency distribution and per-function complexity", () => {
    expect(validateMeasurementRecord(latency)).toBe(latency);
    expect(validateMeasurementRecord(complexity)).toBe(complexity);
    expect(latency.distribution?.quantiles?.[1]).toEqual({
      probability: 0.95,
      value: 43.9,
    });
    expect(complexity.subject.kind).toBe("function");
  });
});

describe("TC-278 definition, unit, subject and scope identity are mandatory", () => {
  // Trace: FR-043-AC-2
  it.each([
    ["unit", { ...complexity, unit: "" }],
    ["plan", { ...complexity, plan: { ...complexity.plan, id: "" } }],
    [
      "definitionVersion",
      {
        ...complexity,
        plan: { ...complexity.plan, definitionVersion: "" },
      },
    ],
    ["subject", { ...complexity, subject: { ...complexity.subject, id: "" } }],
    [
      "scope",
      { ...complexity, scope: { ...complexity.scope, repository: "" } },
    ],
  ])("rejects an ambiguous or missing %s before writing", (_name, record) => {
    expect(() =>
      writeMeasurementCollection(repo, collection(record as MeasurementRecord)),
    ).toThrow();
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });
});

describe("TC-279 non-finite observations never reach JSON serialization", () => {
  // Trace: FR-043-AC-3
  it.each([
    { ...latency, value: Number.NaN },
    {
      ...latency,
      distribution: {
        ...latency.distribution!,
        mean: Number.POSITIVE_INFINITY,
      },
    },
    {
      ...latency,
      distribution: {
        ...latency.distribution!,
        quantiles: [{ probability: 0.95, value: Number.NEGATIVE_INFINITY }],
      },
    },
  ])("rejects $value and non-finite distribution members", (record) => {
    expect(() =>
      writeMeasurementCollection(repo, collection(record)),
    ).toThrow();
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });
});

describe("TC-280 measurement identity and bytes are deterministic", () => {
  // Trace: FR-043-AC-4
  it("is byte-identical on repeat and keys paths by collection identity", () => {
    const batch = collection(latency);
    const path = writeMeasurementCollection(repo, batch);
    const first = readFileSync(path, "utf8");
    expect(writeMeasurementCollection(repo, batch)).toBe(path);
    expect(readFileSync(path, "utf8")).toBe(first);
    expect(JSON.parse(first).schemaVersion).toBe(1);

    for (const changed of [
      { ...latency, plan: { ...latency.plan, definitionVersion: "3" } },
      { ...latency, sourceRevision: "f".repeat(40) },
      {
        ...latency,
        tool: {
          ...latency.tool,
          configurationDigest: `sha256:${"f".repeat(64)}`,
        },
      },
    ]) {
      expect(measurementCollectionPath(repo, collection(changed))).not.toBe(
        path,
      );
    }
    expect(
      measurementCollectionPath(
        repo,
        collection({
          ...latency,
          subject: { ...latency.subject, id: "quoin --help" },
        }),
      ),
    ).toBe(path);
  });

  it("stores a thousand logical observations in one physical file", () => {
    const records = Array.from({ length: 1_000 }, (_, index) => ({
      ...latency,
      subject: { kind: "function", id: `src/lib.rs#f${index}` },
      scope: { ...latency.scope, path: "src/lib.rs" },
      value: index,
    }));
    writeMeasurementCollection(repo, collection(...records));
    const directory = join(
      repo,
      "spec",
      "evidence",
      "measurements",
      latency.plan.id,
    );
    expect(
      readFileSync(join(directory, readdirSync(directory)[0]), "utf8"),
    ).toContain('"src/lib.rs#f999"');
    expect(readdirSync(directory)).toHaveLength(1);
  });
});

describe("TC-281 provenance survives a validated write/read round trip", () => {
  // Trace: FR-043-AC-5
  it("preserves tool, environment, sampling, time and raw evidence", () => {
    const stored = readMeasurementCollection(
      writeMeasurementCollection(repo, collection(latency)),
    );
    const [record] = measurementRecordsFromCollection(stored!);
    expect(record.tool).toEqual(latency.tool);
    expect(record.environment).toEqual(latency.environment);
    expect(record.sampling).toEqual(latency.sampling);
    expect(record.collectedAt).toBe(latency.collectedAt);
    expect(record.rawEvidence).toEqual(latency.rawEvidence);
  });

  it("returns null for an absent record and validates a record read from disk", () => {
    expect(readMeasurementCollection(join(repo, "missing.json"))).toBeNull();
    const path = join(repo, "invalid.json");
    writeFileSync(
      path,
      JSON.stringify({ ...collection(latency), threshold: 50 }),
    );
    expect(() => readMeasurementCollection(path)).toThrow(
      /additional properties/,
    );
  });
});

describe("TC-282 the schema is closed, policy-free and measure-neutral", () => {
  // Trace: FR-043-AC-6
  // Trace: FR-043-CON-1
  // Trace: FR-043-CON-4
  it("rejects policy and contains no named standard or hard-coded measure", () => {
    expect(() =>
      validateMeasurementRecord({ ...latency, threshold: 50 }),
    ).toThrow(/additional properties/);

    const schema = JSON.stringify([
      readMeasurementSchema(),
      readMeasurementCollectionSchema(),
    ]).toLowerCase();
    for (const forbidden of [
      "threshold",
      "verdict",
      "compliance",
      "iso/iec",
      "latency-ms",
      "cyclomatic",
    ]) {
      expect(schema).not.toContain(forbidden);
    }
  });
});
