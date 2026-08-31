import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import fc from "fast-check";

import {
  adaptGraphQualityObservation,
  adaptQuireAssurance,
  graphQualityObservationId,
  selectGraphAdapter,
  type GraphQualityObservationV1,
  type InvocationAttestation,
  type QuireAssuranceV1,
} from "../src/measurement/graph-adapters.js";
import {
  readMeasurementCollections,
  writeMeasurementCollection,
  type MeasurementPlan,
} from "../src/measurement/index.js";

const SHA_A = `sha256:${"a".repeat(64)}`;
const SHA_B = `sha256:${"b".repeat(64)}`;
const REV_A = "a".repeat(40);
const REV_B = "b".repeat(40);
const SCORER_DIGEST = `sha256:${createHash("sha256")
  .update("scorer")
  .digest("hex")}`;

function quireExport(): QuireAssuranceV1 {
  return {
    format: "quire-assurance",
    format_version: 1,
    source: { repository: "agent-ix/quoin", revision: REV_A },
    modules: [
      {
        name: "spec-artifacts-iso",
        version: "1.2.3",
        schemas: [{ archetype: "FR", schema_digest: "c".repeat(64) }],
      },
    ],
    artifacts: [],
    obligations: [],
    symbols: [],
    relation_kinds: [
      {
        kind: "requires",
        availability: "available",
        sources: ["module_vocabulary"],
      },
    ],
    relations: [],
    relation_observations: [],
  };
}

function graphRecord(
  state: "measured" | "empty" | "unreadable" | "unsupported" = "measured",
): GraphQualityObservationV1 {
  const population = {
    state,
    files_seen: state === "empty" ? 0 : 2,
    supported_files: state === "measured" ? 2 : 0,
    unreadable_files: state === "unreadable" ? 1 : 0,
    unsupported_files: state === "unsupported" ? 2 : 0,
    census: {
      languages: [{ key: "typescript", count: 2 }],
      node_kinds: [{ key: "function", count: 3 }],
      relation_kinds: [{ key: "calls", count: 1 }],
      resolver_tiers: [{ key: "local", count: 1 }],
    },
  } as const;
  const withoutId = {
    schema_version: 1 as const,
    record_type: "graph_quality_observation" as const,
    producer: {
      extractor_revision: REV_A,
      producer_contract_version: 1,
      parser_grammars: [
        {
          language: "typescript" as const,
          grammar: "tree-sitter-typescript",
          revision: REV_B,
        },
      ],
      configuration_digest: SHA_A,
      source_revision: REV_A,
      corpus_revision: REV_B,
      scorer_version: "1.0.0",
    },
    measurement_plan: {
      ref: "ix://agent-ix/quire-code-rs/MP-001" as const,
      definition_version: "quire-code.graph-quality-v1" as const,
    },
    population,
    ...(state === "measured"
      ? {
          results: {
            confusion_matrices: [
              {
                dimension: "overall" as const,
                key: "all",
                true_positive: 1,
                false_positive: 0,
                false_negative: 1,
                true_negative: 1,
              },
              {
                dimension: "language" as const,
                key: "typescript",
                true_positive: 1,
                false_positive: 0,
                false_negative: 1,
                true_negative: 1,
              },
              {
                dimension: "node_kind" as const,
                key: "function",
                true_positive: 1,
                false_positive: 0,
                false_negative: 1,
                true_negative: 1,
              },
              {
                dimension: "relation_kind" as const,
                key: "calls",
                true_positive: 1,
                false_positive: 0,
                false_negative: 1,
                true_negative: 1,
              },
              {
                dimension: "resolver_tier" as const,
                key: "local",
                true_positive: 1,
                false_positive: 0,
                false_negative: 1,
                true_negative: 1,
              },
            ],
            unresolved: [
              { dimension: "overall" as const, key: "all", count: 1 },
            ],
            ambiguous: [
              { dimension: "overall" as const, key: "all", count: 0 },
            ],
            recall: [
              {
                dimension: "overall" as const,
                key: "all",
                recovered: 1,
                expected: 2,
                ratio: 0.5,
              },
            ],
          },
        }
      : {}),
    raw_scorer_output: { path: "score.json", digest: SCORER_DIGEST },
  };
  return {
    ...withoutId,
    observation_id: graphQualityObservationId(withoutId),
  } as GraphQualityObservationV1;
}

function attestation(): InvocationAttestation {
  return {
    subject: "agent-ix/quire-code-rs",
    scope: { corpus: "fixture" },
    timestamp: "2026-08-31T12:00:00.000Z",
    environment: { runner: "test" },
    verificationStack: {
      schemaVersion: "verification-stack-attestation-v1",
      lockDigest: SHA_A,
      executableDigest: SHA_B,
      buildProfile: "release",
      toolchains: { node: "24.0.0", rust: "1.94.0", python: "3.13.0" },
      sources: {
        producer: {
          revision: REV_A,
          sourceState: "clean",
          remote: "https://example.invalid/producer",
        },
      },
      capabilities: ["graph-quality"],
      artifacts: { scorer: SHA_B },
    },
  };
}

function plan(over: Partial<MeasurementPlan> = {}): MeasurementPlan {
  return {
    id: "MP-001",
    title: "Graph quality",
    status: "active",
    stage: "gate",
    metric: "graph_quality",
    definitionVersion: "quire-code.graph-quality-v1",
    path: "spec/assurance/MP-001.md",
    ...over,
  };
}

function adapt(record = graphRecord(), bytes = Buffer.from("scorer")) {
  const patched = {
    ...record,
    raw_scorer_output: {
      ...record.raw_scorer_output,
      digest: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
    },
  };
  patched.observation_id = graphQualityObservationId({
    ...patched,
    observation_id: undefined,
  });
  return adaptGraphQualityObservation({
    record: patched,
    scorerBytes: bytes,
    scorerMediaType: "application/json",
    attestation: attestation(),
    plans: [plan()],
  });
}

describe("FR-066 governed graph producer adapters", () => {
  // Trace: FR-066-AC-1
  test("TC-1293 selects exact versioned names and refuses an unknown adapter", () => {
    expect(selectGraphAdapter("quire-assurance-v1")).toBe("quire-assurance-v1");
    expect(selectGraphAdapter("quire-code-graph-quality-v1")).toBe(
      "quire-code-graph-quality-v1",
    );
    expect(() => selectGraphAdapter("graph-quality")).toThrow(
      /unknown_adapter.*quire-assurance-v1.*quire-code-graph-quality-v1/,
    );
  });

  // Trace: FR-066-AC-2
  test("TC-1294 preserves a valid Quire export and rejects every premise drift", () => {
    const value = quireExport();
    const accepted = {
      format: value.format,
      formatVersion: value.format_version,
      source: value.source,
      modules: value.modules,
    };
    expect(adaptQuireAssurance(value, accepted)).toEqual(value);
    expect(() =>
      adaptQuireAssurance({ ...value, format_version: 2 }, accepted),
    ).toThrow(/invalid_premise.*format_version/);
    expect(() =>
      adaptQuireAssurance(
        { ...value, modules: [{ ...value.modules[0], version: "9.9.9" }] },
        accepted,
      ),
    ).toThrow(/invalid_premise.*modules/);
  });

  // Trace: FR-066-AC-3
  test("TC-1295 hands every Quire graph collection through without translation", () => {
    const value = quireExport();
    value.artifacts.push({
      id: "FR-001",
      artifact_type: "FR",
      locator: { path: "spec/FR-001.md", line: 1, digest: "d".repeat(64) },
    });
    const output = adaptQuireAssurance(value, {
      format: value.format,
      formatVersion: value.format_version,
      source: value.source,
      modules: value.modules,
    });
    expect(output).toEqual(value);
    expect(output.relation_kinds[0]).toEqual(value.relation_kinds[0]);

    value.relation_kinds[0].sources.push("module_vocabulary");
    expect(() =>
      adaptQuireAssurance(value, {
        format: value.format,
        formatVersion: value.format_version,
        source: value.source,
        modules: value.modules,
      }),
    ).toThrow(/invalid_premise.*duplicate/i);
  });

  // Trace: FR-066-AC-4
  test("TC-1296 validates the closed graph-quality schema and canonical id", () => {
    const record = graphRecord();
    expect(() =>
      adaptGraphQualityObservation({
        record: { ...record, observation_id: SHA_A },
        scorerBytes: Buffer.from("scorer"),
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [plan()],
      }),
    ).toThrow(/invalid_observation.*observation_id/);
    expect(() =>
      adaptGraphQualityObservation({
        record: { ...record, surprise: true },
        scorerBytes: Buffer.from("scorer"),
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [plan()],
      }),
    ).toThrow(/invalid_observation/);
  });

  // Trace: FR-066-AC-5
  test("TC-1297 retains exact scorer bytes and refuses a digest mismatch", () => {
    const bytes = Buffer.from("scorer");
    const collection = adapt(graphRecord(), bytes);
    expect(collection.rawEvidence).toMatchObject({
      scorer: {
        mediaType: "application/json",
        bytesBase64: bytes.toString("base64"),
      },
    });
    const record = graphRecord();
    record.raw_scorer_output.digest = SHA_B;
    record.observation_id = graphQualityObservationId({
      ...record,
      observation_id: undefined,
    });
    expect(() =>
      adaptGraphQualityObservation({
        record,
        scorerBytes: bytes,
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [plan()],
      }),
    ).toThrow(/attachment_digest_mismatch/);
  });

  // Trace: FR-066-AC-6
  test("TC-1298 refuses every missing attestation field", () => {
    for (const field of [
      "subject",
      "scope",
      "timestamp",
      "environment",
      "verificationStack",
    ]) {
      const value = { ...attestation() } as Record<string, unknown>;
      delete value[field];
      expect(() =>
        adaptGraphQualityObservation({
          record: graphRecord(),
          scorerBytes: Buffer.from("scorer"),
          scorerMediaType: "application/json",
          attestation: value,
          plans: [plan()],
        }),
      ).toThrow(/invalid_attestation/);
    }
  });

  // Trace: FR-066-AC-7
  test("TC-1299 refuses absent, inactive, or mismatched graph-quality plans", () => {
    for (const plans of [
      [],
      [plan({ status: "retired" })],
      [plan({ id: "MP-999" })],
      [plan({ definitionVersion: "v2" })],
    ] as MeasurementPlan[][]) {
      expect(() =>
        adaptGraphQualityObservation({
          record: graphRecord(),
          scorerBytes: Buffer.from("scorer"),
          scorerMediaType: "application/json",
          attestation: attestation(),
          plans,
        }),
      ).toThrow(/inactive_plan/);
    }
  });

  // Trace: FR-066-AC-8
  test("TC-1300 maps census entries bijectively and deterministically", () => {
    fc.assert(
      fc.property(
        fc.shuffledSubarray(
          ["languages", "node_kinds", "relation_kinds", "resolver_tiers"],
          { minLength: 4, maxLength: 4 },
        ),
        (order) => {
          const record = graphRecord();
          const census = record.population.census as Record<string, unknown>;
          record.population.census = Object.fromEntries(
            order.map((key) => [key, census[key]]),
          ) as typeof record.population.census;
          record.observation_id = graphQualityObservationId({
            ...record,
            observation_id: undefined,
          });
          const collection = adapt(record);
          expect(collection.observations[0].population?.identity).toEqual(
            record.population,
          );
          const observations = collection.observations.filter(
            (item) => item.dimensions?.measure === "census",
          );
          expect(observations.map((item) => item.dimensions)).toEqual(
            [...observations.map((item) => item.dimensions)].sort((a, b) =>
              JSON.stringify(a).localeCompare(JSON.stringify(b)),
            ),
          );
          expect(observations).toHaveLength(4);
        },
      ),
    );
  });

  // Trace: FR-066-AC-9
  test("TC-1301 maps result facts bijectively and refuses duplicate keys", () => {
    const collection = adapt();
    expect(
      collection.observations.filter(
        (row) => row.dimensions?.measure === "confusion_matrix",
      ),
    ).toHaveLength(20);
    expect(
      collection.observations.filter(
        (row) => row.dimensions?.measure === "recall",
      ),
    ).toHaveLength(1);
    const record = graphRecord();
    record.results?.confusion_matrices.push(
      record.results.confusion_matrices[0],
    );
    record.observation_id = graphQualityObservationId({
      ...record,
      observation_id: undefined,
    });
    expect(() => adapt(record)).toThrow(/duplicate_partition/);
  });

  // Trace: FR-066-AC-10
  test("TC-1302 retains census and each distinct not-computed state", () => {
    for (const state of ["empty", "unreadable", "unsupported"] as const) {
      const collection = adapt(graphRecord(state));
      const states = collection.observations.filter(
        (row) => row.state === "not_computed",
      );
      expect(states).toHaveLength(1);
      expect(states[0].dimensions).toMatchObject({
        measure: "quality_state",
        key: state,
      });
      expect(
        collection.observations.some(
          (row) => row.dimensions?.measure === "census",
        ),
      ).toBe(true);
    }
  });

  // Trace: FR-066-AC-11
  test("TC-1303 adapted intake is idempotent and collision-safe", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-graph-adapter-"));
    try {
      mkdirSync(join(root, "spec", "assurance"), { recursive: true });
      writeFileSync(
        join(root, "spec", "assurance", "MP-001.md"),
        `---\nid: MP-001\ntitle: Graph quality\ntype: MeasurementPlan\nstatus: active\nstage: gate\nmetric: graph_quality\ndefinition_version: quire-code.graph-quality-v1\n---\n`,
      );
      const collection = adapt();
      const first = writeMeasurementCollection(root, collection);
      expect(writeMeasurementCollection(root, collection)).toBe(first);
      expect(readMeasurementCollections(root)).toEqual([collection]);
      expect(() =>
        writeMeasurementCollection(root, { ...collection, subject: "changed" }),
      ).toThrow(/same|different content|already exists/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  // Trace: FR-066-AC-12
  test("TC-1304 adapters have no producer, network, Git, or frontmatter dependency", () => {
    const source = readFileSync(
      join(process.cwd(), "src", "measurement", "graph-adapters.ts"),
      "utf8",
    );
    expect(source).not.toMatch(/node:(?:child_process|fs|http|https|net)/);
    expect(source).not.toMatch(
      /runQuire|readBundleFrontmatter|graph-analysis|simple-git/,
    );
  });
});
