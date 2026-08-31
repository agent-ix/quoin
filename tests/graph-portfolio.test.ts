import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import fc from "fast-check";

import {
  adaptGraphQualityObservation,
  buildGovernedGraphPortfolio,
  buildGovernedGraphPortfolioFrom,
  canonicalGraphPortfolioJson,
  compareGraphQualityCollections,
  graphQualityObservationId,
  GraphPortfolioMappingError,
  parseGraphPortfolioMappings,
  readMeasurementCollectionResults,
  renderGovernedGraphPortfolio,
  type GraphPortfolioRepositoryInput,
  type MeasurementCollection,
  type MeasurementPlan,
  type PortfolioRepositoryReport,
} from "../src/measurement/index.js";

const PLAN: MeasurementPlan = {
  id: "MP-001",
  title: "Graph quality",
  status: "active",
  stage: "branch-comparison",
  metric: "graph_quality",
  definitionVersion: "quire-code.graph-quality-v1",
  path: "spec/assurance/MP-001.md",
  owner: "assurance-team",
  action: "repair retained evidence",
};
const RETAINED_SCORER_BYTES = Buffer.from("retained scorer");
const RETAINED_SCORER_DIGEST = `sha256:${createHash("sha256")
  .update(RETAINED_SCORER_BYTES)
  .digest("hex")}`;

function collection(
  id: string,
  timestamp: string,
  overrides: Partial<MeasurementCollection> = {},
): MeasurementCollection {
  const producer = {
    extractor_revision: "a".repeat(40),
    producer_contract_version: 1,
    parser_grammars: [
      {
        language: "rust",
        grammar: "tree-sitter-rust",
        revision: "b".repeat(40),
      },
    ],
    configuration_digest: `sha256:${"c".repeat(64)}`,
    source_revision: "d".repeat(40),
    corpus_revision: "e".repeat(40),
    scorer_version: "1.0.0",
  };
  return {
    schemaVersion: 2,
    collectionId: id,
    subject: "fixture",
    scope: { roots: ["src"] },
    toolIdentity: "agent-ix/quire-code-rs",
    toolVersion: producer.extractor_revision,
    configDigest: producer.configuration_digest,
    timestamp,
    sourceRevision: producer.source_revision,
    corpusRevision: producer.corpus_revision,
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
      {
        metric: "graph_quality",
        planId: "MP-001",
        definitionVersion: "quire-code.graph-quality-v1",
        state: "measured",
        value: 1,
        unit: "count",
        shape: "count",
        population: {
          examined: 2,
          matched: 2,
          complete: true,
          identity: { files: ["a.rs", "b.rs"] },
        },
        dimensions: {
          measure: "census",
          dimension: "node_kind",
          key: "function",
        },
      },
    ],
    rawEvidence: {
      producer: {
        observation_id: `sha256:${id.padEnd(64, "0").slice(0, 64)}`,
        producer,
        population: { state: "measured" },
        raw_scorer_output: { digest: RETAINED_SCORER_DIGEST },
      },
      scorer: {
        digest: RETAINED_SCORER_DIGEST,
        bytesBase64: RETAINED_SCORER_BYTES.toString("base64"),
      },
    },
    ...overrides,
  };
}

function base(root: string): PortfolioRepositoryReport {
  return {
    name: root.split("/").at(-1) ?? root,
    root,
    status: "readable",
    error: null,
    store: "present",
    profiles: [],
    plans: [PLAN],
    measurements: null,
    latestCollection: null,
    comparison: null,
    staleness: {
      status: "not_computed",
      ageDays: null,
      relativeTo: null,
      thresholdDays: 30,
    },
  };
}

function repository(
  root: string,
  collections: MeasurementCollection[],
  overrides: Partial<GraphPortfolioRepositoryInput> = {},
): GraphPortfolioRepositoryInput {
  return {
    portfolio: base(root),
    plans: [PLAN],
    collections: collections.map((value) => ({
      path: `${root}/spec/evidence/measurements/${value.collectionId}.json`,
      collection: value,
    })),
    graph: { availability: "missing", reason: "no graph export" },
    ...overrides,
  };
}

describe("governed graph portfolio", () => {
  // Trace: FR-067-AC-1
  test("TC-1305 current carries the active plan, producer tuple, revisions, population, and raw digests", () => {
    const current = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [collection("new", "2026-08-31T00:00:00Z")]),
    ]).repositories[0].graphQuality.current;

    expect(current).toMatchObject({
      availability: "available",
      plan: { id: "MP-001", definitionVersion: "quire-code.graph-quality-v1" },
      toolIdentity: "agent-ix/quire-code-rs",
      sourceRevision: "d".repeat(40),
      corpusRevision: "e".repeat(40),
      producerRecordDigest: expect.stringMatching(/^sha256:/),
      scorerDigest: RETAINED_SCORER_DIGEST,
      populationIdentity: { files: ["a.rs", "b.rs"] },
    });
    expect(current?.producer).toMatchObject({ producer_contract_version: 1 });
  });

  // Trace: FR-067-AC-2
  test("TC-1306 history retains readable collections in timestamp/id order and marks retired-plan evidence incompatible", () => {
    const old = collection("z-old", "2026-01-01T00:00:00Z", {
      observations: collection("x", "2026-01-01T00:00:00Z").observations.map(
        (row) => ({ ...row, planId: "MP-RETIRED" }),
      ),
    });
    const report = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [
        collection("a-new", "2026-02-01T00:00:00Z"),
        old,
      ]),
    ]);
    expect(
      report.repositories[0].graphQuality.history.map((row) => [
        row.id,
        row.availability,
      ]),
    ).toEqual([
      ["z-old", "incompatible"],
      ["a-new", "available"],
    ]);
  });

  // Trace: FR-067-AC-3
  test("TC-1307 partitions and repositories remain separate under permutations", () => {
    fc.assert(
      fc.property(
        fc.shuffledSubarray(["a", "b"], { minLength: 2, maxLength: 2 }),
        fc.boolean(),
        (order, reversePartitions) => {
          const inputs = order.map((name) => {
            const values = [
              collection(`${name}-1`, "2026-08-30T00:00:00Z"),
              collection(`${name}-2`, "2026-08-31T00:00:00Z"),
            ];
            for (const value of values) {
              if (name === "a")
                value.observations[0].dimensions = {
                  measure: "census",
                  dimension: "language",
                  key: "rust|tsx\nline",
                };
              if (reversePartitions) value.observations.reverse();
            }
            return repository(`/repos/${name}`, values);
          });
          const report = buildGovernedGraphPortfolioFrom(inputs);
          expect(report.repositories.map((row) => row.name)).toEqual([
            "a",
            "b",
          ]);
          expect(
            report.repositories.flatMap(
              (row) => row.graphQuality.current?.partitions ?? [],
            ),
          ).toHaveLength(4);
          expect(
            report.repositories[0].graphQuality.current?.partitions.map(
              ({ key }) => key,
            ),
          ).toContain("rust|tsx\nline");
          const human = renderGovernedGraphPortfolio(report);
          expect(human).toContain("| rust\\|tsx<br>line |");
          expect(human.split("\n")).not.toContain("line");
        },
      ),
    );
  });

  // Trace: FR-067-AC-4
  test("TC-1308 availability is separate from measurement state and numeric zero", () => {
    const unavailable = [
      "missing",
      "unreadable",
      "incompatible",
      "unknown",
      "not_applicable",
    ] as const;
    for (const availability of unavailable) {
      const report = buildGovernedGraphPortfolioFrom([
        repository("/repos/a", [], {
          graph: { availability, reason: availability },
        }),
      ]);
      expect(report.repositories[0].graph.availability).toBe(availability);
      expect(JSON.stringify(report.repositories[0].graph)).not.toContain(
        '"value":0',
      );
    }
    const notComputed = collection("state", "2026-08-31T00:00:00Z", {
      observations: [
        {
          ...collection("x", "2026-08-31T00:00:00Z").observations[0],
          state: "not_computed",
          value: null,
          reason: "unsupported",
        },
      ],
    });
    expect(
      buildGovernedGraphPortfolioFrom([repository("/repos/a", [notComputed])])
        .repositories[0].graphQuality.current?.partitions[0],
    ).toMatchObject({ state: "not_computed", value: null });
  });

  // Trace: FR-067-AC-5
  test("TC-1309 every graph compatibility premise blocks a delta independently", () => {
    const before = collection("before", "2026-01-01T00:00:00Z");
    expect(
      compareGraphQualityCollections(
        before,
        collection("after", "2026-02-01T00:00:00Z"),
      )[0],
    ).toMatchObject({ status: "comparable", delta: 0 });
    const mutations: Array<[string, (value: MeasurementCollection) => void]> = [
      [
        "plan_changed",
        (value) => {
          value.observations[0].planId = "MP-002";
        },
      ],
      [
        "definition_changed",
        (value) => {
          value.observations[0].definitionVersion = "v2";
        },
      ],
      [
        "configuration_changed",
        (value) => {
          value.configDigest = "sha256:changed";
        },
      ],
      [
        "tool_changed",
        (value) => {
          value.toolVersion = "changed";
        },
      ],
      [
        "corpus_changed",
        (value) => {
          value.corpusRevision = "changed";
        },
      ],
      [
        "population_changed",
        (value) => {
          value.observations[0].population = {
            ...value.observations[0].population,
            identity: { files: ["other.rs"] },
          };
        },
      ],
      [
        "population_incomplete",
        (value) => {
          value.observations[0].population = {
            ...value.observations[0].population,
            complete: false,
          };
        },
      ],
    ];
    for (const [code, mutate] of mutations) {
      const after = structuredClone(
        collection(`after-${code}`, "2026-02-01T00:00:00Z"),
      );
      mutate(after);
      const row = compareGraphQualityCollections(before, after)[0];
      expect(row).toMatchObject({ status: "incomparable", delta: null });
      expect(row.reasons).toContainEqual(
        expect.objectContaining({ code, blocking: true }),
      );
    }

    const retiredBefore = collection("retired-before", "2026-03-01T00:00:00Z", {
      observations: before.observations.map((row) => ({
        ...row,
        planId: "MP-RETIRED",
      })),
    });
    const retiredAfter = collection("retired-after", "2026-04-01T00:00:00Z", {
      observations: before.observations.map((row) => ({
        ...row,
        planId: "MP-RETIRED",
      })),
    });
    const incompatible = buildGovernedGraphPortfolioFrom([
      repository("/repos/retired", [retiredAfter, retiredBefore]),
    ]).repositories[0].graphQuality.comparison;
    expect(incompatible?.observations).not.toHaveLength(0);
    expect(incompatible?.observations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          status: "incomparable",
          delta: null,
          reasons: expect.arrayContaining([
            expect.objectContaining({ code: "collection_incompatible" }),
          ]),
        }),
      ]),
    );
    expect(
      incompatible?.observations.some(
        ({ status, delta }) => status === "comparable" || delta !== null,
      ),
    ).toBe(false);

    const retiredNotComputed = ["before", "after"].map((suffix, index) =>
      collection(
        `retired-not-computed-${suffix}`,
        `2026-05-0${index + 1}T00:00:00Z`,
        {
          observations: [
            {
              ...before.observations[0],
              planId: "MP-RETIRED",
              state: "not_computed",
              value: null,
              reason: "unsupported",
            },
          ],
        },
      ),
    );
    const notComputedComparison = buildGovernedGraphPortfolioFrom([
      repository("/repos/retired-not-computed", retiredNotComputed),
    ]).repositories[0].graphQuality.comparison;
    expect(notComputedComparison?.observations).toEqual([
      expect.objectContaining({
        status: "not_computed",
        delta: null,
        reasons: expect.arrayContaining([
          expect.objectContaining({ code: "collection_incompatible" }),
        ]),
      }),
    ]);

    const inherited = base("/repos/inherited");
    inherited.measurements = {
      plans: [PLAN],
      corpusGaps: null,
      current: [
        {
          metric: "graph_quality",
          planId: PLAN.id,
          planPath: PLAN.path,
          planDefinitionVersion: PLAN.definitionVersion,
          stage: PLAN.stage,
          observation: retiredAfter.observations[0],
          collection: {
            collectionId: retiredAfter.collectionId,
            timestamp: retiredAfter.timestamp,
            toolIdentity: retiredAfter.toolIdentity,
            toolVersion: retiredAfter.toolVersion,
            configDigest: retiredAfter.configDigest,
            sourceRevision: retiredAfter.sourceRevision,
            corpusRevision: retiredAfter.corpusRevision,
            path: "/repos/inherited/spec/evidence/measurements/retired-after.json",
          },
        },
      ],
    };
    const sanitized = buildGovernedGraphPortfolioFrom([
      repository("/repos/inherited", [retiredAfter], {
        portfolio: inherited,
      }),
    ]).repositories[0];
    expect(sanitized.graphQuality.current).toBeNull();
    expect(sanitized.measurements?.current).toEqual([
      expect.objectContaining({ observation: null, collection: null }),
    ]);
  });

  // Trace: FR-067-AC-6
  test("TC-1310 raw identities are the retained producer and scorer digests in every view", () => {
    const result = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [
        collection("one", "2026-01-01T00:00:00Z"),
        collection("two", "2026-02-01T00:00:00Z"),
      ]),
    ]).repositories[0].graphQuality;
    expect(result.history.map((row) => row.scorerDigest)).toEqual([
      RETAINED_SCORER_DIGEST,
      RETAINED_SCORER_DIGEST,
    ]);
    expect(result.comparison).toMatchObject({
      before: { scorerDigest: expect.stringMatching(/^sha256:/) },
      after: { producerRecordDigest: expect.stringMatching(/^sha256:/) },
    });
  });

  // Trace: FR-067-AC-7
  test("TC-1311 structural report objects remain byte-identical and absent inputs are explicit", () => {
    const fanOut = Object.freeze({
      type: "fan-out",
      rows: [{ id: "FR-001", count: 2 }],
    });
    const churn = Object.freeze({ type: "churn", availability: "unknown" });
    const impact = Object.freeze({
      type: "change-impact",
      seed: "FR-001",
      affected: ["TC-001"],
    });
    const report = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [], {
        graph: {
          availability: "available",
          premises: { revision: "a" },
          fanOut,
          churn,
          changeImpact: [impact],
        },
      }),
    ]).repositories[0].graph;
    expect(report.fanOut).toBe(fanOut);
    expect(report.churn).toBe(churn);
    expect(report.changeImpact?.[0]).toBe(impact);
    const human = renderGovernedGraphPortfolio(
      buildGovernedGraphPortfolioFrom([
        repository("/repos/a", [], {
          graph: {
            availability: "available",
            premises: { revision: "a" },
            fanOut,
            churn,
            changeImpact: [impact],
          },
        }),
      ]),
    );
    expect(human).toContain("# QA portfolio report");
    expect(human).toContain('Fan-out: {\n  "rows"');
    expect(
      buildGovernedGraphPortfolioFrom([repository("/repos/b", [])])
        .repositories[0].graph,
    ).toMatchObject({
      availability: "missing",
      changeImpact: { availability: "not_applicable" },
    });

    const partial = parseGraphPortfolioMappings(["/repos/a"], {
      graphExports: ["/repos/a=/inputs/export.json"],
    });
    expect(partial[0]).toMatchObject({
      status: "incompatible",
      reason: expect.stringContaining("export, premises, and audit"),
    });
  });

  // Trace: FR-067-AC-8
  test("TC-1312 corrupt collections and graph inputs become local gaps without hiding siblings", () => {
    const result = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [collection("good", "2026-08-31T00:00:00Z")], {
        collections: [
          {
            path: "/repos/a/broken.json",
            availability: "unreadable",
            error: "invalid JSON",
          },
          {
            path: "/repos/a/good.json",
            collection: collection("good", "2026-08-31T00:00:00Z"),
          },
        ],
        graph: {
          availability: "unreadable",
          reason: "bad export",
          path: "/repos/a/graph.json",
        },
      }),
      repository("/repos/b", [collection("other", "2026-08-31T00:00:00Z")]),
    ]);
    expect(result.repositories[0].graphQuality.current?.id).toBe("good");
    expect(result.repositories[0].gaps).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          availability: "unreadable",
          path: "/repos/a/broken.json",
          owner: "assurance-team",
          action: "repair retained evidence",
        }),
        expect.objectContaining({
          availability: "unreadable",
          path: "/repos/a/graph.json",
        }),
      ]),
    );
    expect(result.repositories[1].graphQuality.current?.id).toBe("other");

    const corruptAttachment = structuredClone(
      collection("bad-attachment", "2026-08-31T00:00:00Z"),
    );
    (
      corruptAttachment.rawEvidence as {
        scorer: { bytesBase64: string };
      }
    ).scorer.bytesBase64 = Buffer.from("changed").toString("base64");
    const attachmentReport = buildGovernedGraphPortfolioFrom([
      repository("/repos/c", [corruptAttachment]),
    ]).repositories[0];
    expect(attachmentReport.graphQuality.current).toBeNull();
    expect(attachmentReport.gaps).toContainEqual(
      expect.objectContaining({
        availability: "unreadable",
        path: expect.stringContaining("bad-attachment"),
      }),
    );

    const root = mkdtempSync(join(tmpdir(), "quoin-graph-read-"));
    try {
      const assurance = join(root, "spec", "assurance");
      const store = join(root, "spec", "evidence", "measurements");
      mkdirSync(assurance, { recursive: true });
      mkdirSync(store, { recursive: true });
      writeFileSync(
        join(assurance, "MP-001.md"),
        `---
id: MP-001
title: "Graph quality"
type: MeasurementPlan
status: active
stage: branch-comparison
metric: graph_quality
definition_version: quire-code.graph-quality-v1
owner: assurance-team
action: repair retained evidence
---

# Graph quality
`,
      );
      writeFileSync(
        join(store, "z-old.json"),
        JSON.stringify(
          collection("z-old", "2026-08-31T00:30:00+02:00", {
            schemaVersion: 1,
          }),
        ),
      );
      writeFileSync(
        join(store, "a-new.json"),
        JSON.stringify(
          collection("a-new", "2026-08-30T23:00:00Z", {
            schemaVersion: 1,
          }),
        ),
      );
      writeFileSync(
        join(store, "bad-time.json"),
        JSON.stringify(
          collection("bad-time", "not-a-time", {
            schemaVersion: 1,
          }),
        ),
      );
      writeFileSync(join(store, "broken.json"), "{not json");
      const reads = readMeasurementCollectionResults(root);
      expect(reads).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            path: expect.stringMatching(/broken\.json$/),
            error: expect.any(String),
          }),
          expect.objectContaining({
            path: expect.stringMatching(/a-new\.json$/),
            collection: expect.objectContaining({ collectionId: "a-new" }),
          }),
        ]),
      );

      const loaded = buildGovernedGraphPortfolio([root]).repositories[0];
      expect(loaded.status, loaded.error ?? undefined).toBe("readable");
      expect(loaded.latestCollection?.id).toBe("a-new");
      expect(loaded.graphQuality.current?.id).toBe("a-new");
      expect(loaded.gaps).toContainEqual(
        expect.objectContaining({
          availability: "unreadable",
          path: expect.stringMatching(/broken\.json$/),
        }),
      );
      expect(loaded.gaps).toContainEqual(
        expect.objectContaining({
          availability: "unreadable",
          path: expect.stringMatching(/bad-time\.json$/),
          reason: expect.stringContaining("timestamp"),
        }),
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  // Trace: FR-067-AC-9
  test("TC-1313 permutations have canonical JSON and human output consumes the report object", () => {
    const a = repository("/repos/a", [collection("a", "2026-08-31T00:00:00Z")]);
    const b = repository("/repos/b", [collection("b", "2026-08-31T00:00:00Z")]);
    const first = buildGovernedGraphPortfolioFrom([b, a]);
    const second = buildGovernedGraphPortfolioFrom([a, b]);
    expect(canonicalGraphPortfolioJson(first)).toBe(
      canonicalGraphPortfolioJson(second),
    );
    expect(renderGovernedGraphPortfolio(first)).toContain("/repos/a");
    expect(renderGovernedGraphPortfolio(first)).toContain("History:");

    const expectedPermutation = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [
        collection("a-old", "2026-01-01T00:00:00Z"),
        collection("a-new", "2026-02-01T00:00:00Z"),
      ]),
      repository("/repos/b", [
        collection("b-old", "2026-01-01T00:00:00Z"),
        collection("b-new", "2026-02-01T00:00:00Z"),
      ]),
    ]);
    fc.assert(
      fc.property(
        fc.shuffledSubarray(["a-old", "a-new", "b-old", "b-new"], {
          minLength: 4,
          maxLength: 4,
        }),
        fc.shuffledSubarray(["a", "b"], { minLength: 2, maxLength: 2 }),
        (collectionOrder, repositoryOrder) => {
          const collections = Object.fromEntries(
            collectionOrder.map((id) => [
              id,
              collection(
                id,
                id.endsWith("old")
                  ? "2026-01-01T00:00:00Z"
                  : "2026-02-01T00:00:00Z",
              ),
            ]),
          );
          const permuted = buildGovernedGraphPortfolioFrom(
            repositoryOrder.map((name) =>
              repository(
                `/repos/${name}`,
                collectionOrder
                  .filter((id) => id.startsWith(`${name}-`))
                  .map((id) => collections[id]),
              ),
            ),
          );
          expect(canonicalGraphPortfolioJson(permuted)).toBe(
            canonicalGraphPortfolioJson(expectedPermutation),
          );
          expect(renderGovernedGraphPortfolio(permuted)).toBe(
            renderGovernedGraphPortfolio(expectedPermutation),
          );
        },
      ),
    );

    const mapping = parseGraphPortfolioMappings(["/repos/a", "/repos/./a"], {
      graphExports: ["/repos/a=/inputs/export.json"],
      graphPremises: ["/repos/./a=/inputs/premises.json"],
      graphAudits: ["/repos/a=/inputs/audit.json"],
      changed: ["/repos/a=FR-002", "/repos/./a=FR-001", "/repos/a=FR-001"],
    });
    expect(mapping).toEqual([
      {
        root: "/repos/a",
        status: "ready",
        exportPath: "/inputs/export.json",
        premisesPath: "/inputs/premises.json",
        auditPath: "/inputs/audit.json",
        changed: ["FR-001", "FR-002"],
      },
    ]);
    for (const [field, code] of [
      ["graphExports", "duplicate_graph_export"],
      ["graphPremises", "duplicate_graph_premises"],
      ["graphAudits", "duplicate_graph_audit"],
    ] as const) {
      expect(() =>
        parseGraphPortfolioMappings(["/repos/a"], {
          [field]: ["/repos/a=/inputs/one.json", "/repos/a=/inputs/two.json"],
        }),
      ).toThrow(
        expect.objectContaining<Partial<GraphPortfolioMappingError>>({ code }),
      );
    }
  });

  // Trace: FR-067-AC-10
  test("TC-1314 old collections stay historical and output has no aggregate verdict", () => {
    const old = collection("old", "2026-01-01T00:00:00Z", {
      schemaVersion: 1,
      verificationStack: undefined,
    });
    const report = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [old]),
    ]);
    expect(report.repositories[0].graphQuality.history).toHaveLength(1);
    expect(canonicalGraphPortfolioJson(report)).not.toMatch(
      /aggregate|trustScore|releaseVerdict/,
    );
  });

  // Trace: FR-067-AC-11
  test("TC-1315 static boundary consumes injected objects and has no execution, graph traversal, or write dependency", () => {
    const root = dirname(fileURLToPath(import.meta.url));
    const source = readFileSync(
      join(root, "../src/measurement/graph-portfolio.ts"),
      "utf8",
    );
    expect(source).not.toMatch(
      /child_process|spawnSync|execFile|writeFile|renameSync|fetch\(|from \"\.\.\/graph-analysis/,
    );
    expect(source).not.toMatch(/relations\.map|traverse|adjacency/);
  });

  // Trace: StR-007-VC-1
  test("TC-1316 retained adapter evidence reaches the portfolio without losing boundaries or identity", () => {
    const scorerBytes = Buffer.from('{"score":1}\n');
    const scorerDigest = `sha256:${createHash("sha256").update(scorerBytes).digest("hex")}`;
    const record: Record<string, unknown> = {
      schema_version: 1,
      record_type: "graph_quality_observation",
      observation_id: `sha256:${"0".repeat(64)}`,
      producer: {
        extractor_revision: "a".repeat(40),
        producer_contract_version: 1,
        parser_grammars: [
          {
            language: "rust",
            grammar: "tree-sitter-rust",
            revision: "b".repeat(40),
          },
        ],
        configuration_digest: `sha256:${"c".repeat(64)}`,
        source_revision: "d".repeat(40),
        corpus_revision: "e".repeat(40),
        scorer_version: "1.0.0",
      },
      measurement_plan: {
        ref: "ix://agent-ix/quire-code-rs/MP-001",
        definition_version: "quire-code.graph-quality-v1",
      },
      population: {
        state: "measured",
        files_seen: 1,
        supported_files: 1,
        unreadable_files: 0,
        unsupported_files: 0,
        census: {
          languages: [{ key: "rust", count: 1 }],
          node_kinds: [{ key: "function", count: 1 }],
          relation_kinds: [{ key: "verifies", count: 1 }],
          resolver_tiers: [{ key: "exact", count: 1 }],
        },
      },
      results: {
        confusion_matrices: [
          "overall",
          "language",
          "node_kind",
          "relation_kind",
        ].map((dimension) => ({
          dimension,
          key: dimension === "overall" ? "all" : "fixture",
          true_positive: 1,
          false_positive: 0,
          false_negative: 0,
          true_negative: 1,
        })),
        unresolved: [],
        ambiguous: [],
        recall: [
          {
            dimension: "overall",
            key: "all",
            recovered: 1,
            expected: 1,
            ratio: 1,
          },
        ],
      },
      raw_scorer_output: { path: "evidence/score.json", digest: scorerDigest },
    };
    record.observation_id = graphQualityObservationId(record);
    const adapted = adaptGraphQualityObservation({
      record,
      scorerBytes,
      scorerMediaType: "application/json",
      plans: [PLAN],
      attestation: {
        subject: "fixture",
        scope: { roots: ["src"] },
        timestamp: "2026-08-31T00:00:00Z",
        environment: { runner: "test" },
        verificationStack: {
          schemaVersion: "verification-stack-attestation-v1",
          lockDigest: `sha256:${"1".repeat(64)}`,
          executableDigest: `sha256:${"2".repeat(64)}`,
          buildProfile: "release",
          toolchains: { node: "22", rust: "1.94", python: "3.10" },
          sources: {
            quire_code: {
              revision: "f".repeat(40),
              sourceState: "clean",
              remote: "https://example.invalid/quire-code-rs",
            },
          },
          capabilities: ["graph-quality"],
          artifacts: { scorer: scorerDigest },
        },
      },
    });
    const report = buildGovernedGraphPortfolioFrom([
      repository("/repos/a", [adapted]),
    ]);
    const current = report.repositories[0].graphQuality.current;
    expect(current?.producerRecordDigest).toBe(record.observation_id);
    expect(current?.scorerDigest).toBe(scorerDigest);
    expect(current?.partitions.length).toBeGreaterThan(1);
    expect(report.repositories[0].graph.availability).toBe("missing");
    expect(canonicalGraphPortfolioJson(report)).not.toMatch(
      /aggregateScore|qualityVerdict|child_process/,
    );
  });
});
