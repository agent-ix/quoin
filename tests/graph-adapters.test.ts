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

function producerObservationId(value: unknown): string {
  const record = { ...(value as Record<string, unknown>) };
  delete record.observation_id;
  const sort = (item: unknown): unknown => {
    if (Array.isArray(item)) return item.map(sort);
    if (item === null || typeof item !== "object") return item;
    const keys = Object.keys(item as Record<string, unknown>).sort((a, b) => {
      const left = Array.from(a, (part) => part.codePointAt(0) as number);
      const right = Array.from(b, (part) => part.codePointAt(0) as number);
      for (
        let index = 0;
        index < Math.min(left.length, right.length);
        index += 1
      )
        if (left[index] !== right[index]) return left[index] - right[index];
      return left.length - right.length;
    });
    return Object.fromEntries(
      keys.map((key) => [key, sort((item as Record<string, unknown>)[key])]),
    );
  };
  return `sha256:${createHash("sha256")
    .update(JSON.stringify(sort(record)))
    .digest("hex")}`;
}

function deleteAt(value: unknown, path: string[]): void {
  const parent = path
    .slice(0, -1)
    .reduce(
      (record, key) => record[key] as Record<string, unknown>,
      value as Record<string, unknown>,
    );
  delete parent[path.at(-1) as string];
}

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
    observation_id: producerObservationId(withoutId),
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
  patched.observation_id = producerObservationId({
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
      adaptQuireAssurance({ ...value, format: "other" }, accepted),
    ).toThrow(/invalid_premise.*format/);
    expect(() =>
      adaptQuireAssurance(
        { ...value, modules: [{ ...value.modules[0], version: "9.9.9" }] },
        accepted,
      ),
    ).toThrow(/invalid_premise.*modules/);
    expect(() =>
      adaptQuireAssurance(
        {
          ...value,
          modules: [
            {
              ...value.modules[0],
              schemas: [
                {
                  ...value.modules[0].schemas[0],
                  schema_digest: "d".repeat(64),
                },
              ],
            },
          ],
        },
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
    const canonicalBytes = '{"a":[{"a":1,"z":2}],"":"bmp","𐀀":"astral"}';
    const expected = `sha256:${createHash("sha256")
      .update(canonicalBytes)
      .digest("hex")}`;
    expect(
      graphQualityObservationId({
        "𐀀": "astral",
        "": "bmp",
        a: [{ z: 2, a: 1 }],
        observation_id: SHA_A,
      }),
    ).toBe(expected);
    expect(canonicalBytes).not.toMatch(/\n|: |, /);

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
    record.observation_id = producerObservationId({
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
    for (const path of [
      ["subject"],
      ["scope"],
      ["timestamp"],
      ["environment"],
      ["verificationStack"],
      ["verificationStack", "schemaVersion"],
      ["verificationStack", "lockDigest"],
      ["verificationStack", "executableDigest"],
      ["verificationStack", "buildProfile"],
      ["verificationStack", "toolchains"],
      ["verificationStack", "toolchains", "node"],
      ["verificationStack", "toolchains", "rust"],
      ["verificationStack", "toolchains", "python"],
      ["verificationStack", "sources"],
      ["verificationStack", "sources", "producer", "revision"],
      ["verificationStack", "sources", "producer", "sourceState"],
      ["verificationStack", "sources", "producer", "remote"],
      ["verificationStack", "capabilities"],
      ["verificationStack", "artifacts"],
    ]) {
      const value = structuredClone(attestation());
      deleteAt(value, path);
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

    for (const path of [
      ["producer", "extractor_revision"],
      ["producer", "producer_contract_version"],
      ["producer", "parser_grammars"],
      ["producer", "parser_grammars", "0", "language"],
      ["producer", "parser_grammars", "0", "grammar"],
      ["producer", "parser_grammars", "0", "revision"],
      ["producer", "configuration_digest"],
      ["producer", "source_revision"],
      ["producer", "corpus_revision"],
      ["producer", "scorer_version"],
    ]) {
      const record = structuredClone(graphRecord());
      deleteAt(record, path);
      record.observation_id = producerObservationId(record);
      expect(() =>
        adaptGraphQualityObservation({
          record,
          scorerBytes: Buffer.from("scorer"),
          scorerMediaType: "application/json",
          attestation: attestation(),
          plans: [plan()],
        }),
      ).toThrow(/invalid_observation/);
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
      ).toThrow(
        /inactive_plan.*expected exactly one active MP-001\/quire-code\.graph-quality-v1.*observed/,
      );
    }

    const historical = plan({
      id: "MP-RETIRED",
      status: "retired",
      definitionVersion: "quire-code.graph-quality-v0",
    });
    expect(
      adaptGraphQualityObservation({
        record: graphRecord(),
        scorerBytes: Buffer.from("scorer"),
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [historical, plan()],
      }).observations,
    ).not.toHaveLength(0);
    expect(
      adaptGraphQualityObservation({
        record: graphRecord(),
        scorerBytes: Buffer.from("scorer"),
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [plan(), historical],
      }).observations,
    ).not.toHaveLength(0);
    expect(() =>
      adaptGraphQualityObservation({
        record: graphRecord(),
        scorerBytes: Buffer.from("scorer"),
        scorerMediaType: "application/json",
        attestation: attestation(),
        plans: [plan(), plan({ id: "MP-OTHER" })],
      }),
    ).toThrow(/expected exactly one active/);
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
          record.observation_id = producerObservationId({
            ...record,
            observation_id: undefined,
          });
          const collection = adapt(record);
          expect(collection.observations[0].population?.identity).toEqual(
            record.population,
          );
          const observations = collection.observations.filter((item) =>
            ["census", "population"].includes(item.dimensions?.measure ?? ""),
          );
          expect(observations.map((item) => item.dimensions)).toEqual(
            [...observations.map((item) => item.dimensions)].sort((a, b) =>
              JSON.stringify({
                dimension: a?.dimension,
                key: a?.key,
                measure: a?.measure,
              }) <
              JSON.stringify({
                dimension: b?.dimension,
                key: b?.key,
                measure: b?.measure,
              })
                ? -1
                : 1,
            ),
          );
          expect(
            Object.fromEntries(
              observations.map((item) => [
                `${item.dimensions?.measure}/${item.dimensions?.dimension}/${item.dimensions?.key}`,
                item.value,
              ]),
            ),
          ).toEqual({
            "census/languages/typescript": 2,
            "census/node_kinds/function": 3,
            "census/relation_kinds/calls": 1,
            "census/resolver_tiers/local": 1,
            "population/overall/files_seen": 2,
            "population/overall/supported_files": 2,
            "population/overall/unreadable_files": 0,
            "population/overall/unsupported_files": 0,
          });
        },
      ),
    );
  });

  // Trace: FR-066-AC-9
  test("TC-1301 maps result facts bijectively and refuses duplicate keys", () => {
    const collection = adapt();
    const facts = Object.fromEntries(
      collection.observations
        .filter((row) =>
          ["ambiguous", "confusion_matrix", "recall", "unresolved"].includes(
            row.dimensions?.measure ?? "",
          ),
        )
        .map((row) => [
          `${row.dimensions?.measure}/${row.dimensions?.dimension}/${row.dimensions?.key}`,
          {
            value: row.value,
            examined: row.population?.examined,
            matched: row.population?.matched,
          },
        ]),
    );
    const expectedConfusion = Object.fromEntries(
      [
        ["overall", "all"],
        ["language", "typescript"],
        ["node_kind", "function"],
        ["relation_kind", "calls"],
        ["resolver_tier", "local"],
      ].flatMap(([dimension, key]) =>
        [
          ["false_negative", 1],
          ["false_positive", 0],
          ["true_negative", 1],
          ["true_positive", 1],
        ].map(([component, value]) => [
          `confusion_matrix/${dimension}/${key}.${component}`,
          { value, examined: 2, matched: 2 },
        ]),
      ),
    );
    expect(facts).toEqual({
      ...expectedConfusion,
      "ambiguous/overall/all": { value: 0, examined: 2, matched: 2 },
      "recall/overall/all": { value: 0.5, examined: 2, matched: 1 },
      "unresolved/overall/all": { value: 1, examined: 2, matched: 2 },
    });
    const record = graphRecord();
    record.results?.confusion_matrices.push(
      record.results.confusion_matrices[0],
    );
    record.observation_id = producerObservationId({
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
