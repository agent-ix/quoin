/** Pure, versioned adapters for retained graph evidence (FR-066). */

import { createHash } from "node:crypto";

import { z } from "zod";

import { canonicalJson } from "../evidence/store.js";
import type {
  MeasurementCollection,
  MeasurementObservation,
  MeasurementPlan,
  VerificationStackAttestation,
} from "./types.js";
import { MEASUREMENT_SCHEMA_VERSION } from "./types.js";
import { validateMeasurementCollection } from "./validate.js";

export const GRAPH_ADAPTER_NAMES = [
  "quire-assurance-v1",
  "quire-code-graph-quality-v1",
] as const;

export type GraphAdapterName = (typeof GRAPH_ADAPTER_NAMES)[number];
export type GraphAdapterErrorCode =
  | "unknown_adapter"
  | "invalid_premise"
  | "invalid_observation"
  | "invalid_attestation"
  | "attachment_missing"
  | "attachment_digest_mismatch"
  | "inactive_plan"
  | "duplicate_partition";

export class GraphAdapterError extends Error {
  constructor(
    readonly code: GraphAdapterErrorCode,
    message: string,
  ) {
    super(`${code}: ${message}`);
    this.name = "GraphAdapterError";
  }
}

const bareDigest = z.string().regex(/^[0-9a-f]{64}$/);
const digest = z.string().regex(/^sha256:[0-9a-f]{64}$/);
const fullRevision = z.string().regex(/^[0-9a-f]{40}$/);
const revisionIdentity = z
  .string()
  .regex(
    /^(?:[0-9a-f]{40}|v?[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?|sha256:[0-9a-f]{64})$/,
  );

const locatorSchema = z
  .object({
    path: z
      .string()
      .min(1)
      .regex(/^(?!\/)(?!.*(?:^|\/)\.\.(?:\/|$)).+$/),
    line: z.number().int().min(1),
    digest: bareDigest,
  })
  .strict();

const sourcePremiseSchema = z
  .object({ repository: z.string().min(1), revision: fullRevision })
  .strict();
const schemaPremiseSchema = z
  .object({ archetype: z.string().min(1), schema_digest: bareDigest })
  .strict();
const modulePremiseSchema = z
  .object({
    name: z.string().min(1),
    version: z.string().min(1),
    schemas: z.array(schemaPremiseSchema),
  })
  .strict();

const artifactSchema = z
  .object({
    id: z.string().min(1),
    uuid: z.string().uuid().optional(),
    artifact_type: z.string().min(1),
    locator: locatorSchema,
  })
  .strict();
const obligationSchema = z
  .object({
    source: z.string().min(1),
    id: z.string().min(1),
    document: z.string().min(1),
    statement: z.string().min(1),
    statement_hash: bareDigest,
    method: z.string().min(1).optional(),
    parameters: z.record(z.string(), z.string()).optional(),
    criticality: z.string().min(1).optional(),
    target_ids: z.array(z.string().min(1)),
    locator: locatorSchema,
  })
  .strict();
const symbolSchema = z
  .object({
    id: bareDigest,
    language: z.enum(["rust", "python", "typescript"]),
    kind: z.enum([
      "function",
      "test_function",
      "container",
      "benchmark",
      "fuzz_target",
    ]),
    qualified_name: z.string().min(1),
    container: z.string().nullable(),
    capabilities: z.array(z.enum(["verifies", "implements"])),
    locator: locatorSchema,
  })
  .strict();
const freshnessSchema = z.enum([
  "current",
  "suspect",
  "unknown",
  "not_applicable",
]);
const relationKindSchema = z
  .object({
    kind: z.string().min(1),
    availability: z.literal("available"),
    sources: z
      .array(
        z.enum([
          "module_vocabulary",
          "required_relation",
          "trace_binding",
          "observed",
        ]),
      )
      .min(1),
  })
  .strict();
const corpusRelationSchema = z
  .object({
    kind: z.literal("corpus"),
    source: z.string().min(1),
    target: z.string().min(1),
    edge_type: z.string().min(1),
    resolution: z.enum(["resolved", "dangling"]),
    locator: locatorSchema,
    freshness: freshnessSchema,
  })
  .strict();
const verifiesRelationSchema = z
  .object({
    kind: z.literal("verifies"),
    source: bareDigest,
    target: z.string().min(1),
    form: z.string().min(1),
    provenance: z.enum(["canonical", "legacy"]),
    locator: locatorSchema,
    freshness: freshnessSchema,
  })
  .strict();
const implementsRelationSchema = z
  .object({
    kind: z.literal("implements"),
    source: bareDigest,
    target: z.string().min(1),
    form: z.string().min(1),
    locator: locatorSchema,
    freshness: freshnessSchema,
  })
  .strict();
const relationObservationSchema = z
  .object({
    declaration: z.string().min(1),
    subject: z.string().min(1).optional(),
    availability: z.enum(["available", "missing", "not_applicable", "unknown"]),
    freshness: freshnessSchema,
    reason: z.string().min(1).optional(),
  })
  .strict();

const quireAssuranceSchema = z
  .object({
    format: z.literal("quire-assurance"),
    format_version: z.literal(1),
    source: sourcePremiseSchema,
    modules: z.array(modulePremiseSchema),
    artifacts: z.array(artifactSchema),
    obligations: z.array(obligationSchema),
    symbols: z.array(symbolSchema),
    relation_kinds: z.array(relationKindSchema),
    relations: z.array(
      z.discriminatedUnion("kind", [
        corpusRelationSchema,
        verifiesRelationSchema,
        implementsRelationSchema,
      ]),
    ),
    relation_observations: z.array(relationObservationSchema),
  })
  .strict();

export type QuireAssuranceV1 = z.infer<typeof quireAssuranceSchema>;

export interface AcceptedQuirePremises {
  format: "quire-assurance";
  formatVersion: 1;
  source: QuireAssuranceV1["source"];
  modules: QuireAssuranceV1["modules"];
}

const censusItemSchema = z
  .object({ key: z.string().min(1), count: z.number().int().min(0) })
  .strict();
const dimensionSchema = z.enum([
  "overall",
  "language",
  "node_kind",
  "relation_kind",
  "resolver_tier",
]);
const dimensionCountSchema = z
  .object({
    dimension: dimensionSchema,
    key: z.string().min(1),
    count: z.number().int().min(0),
  })
  .strict();
const confusionMatrixSchema = z
  .object({
    dimension: dimensionSchema,
    key: z.string().min(1),
    true_positive: z.number().int().min(0),
    false_positive: z.number().int().min(0),
    false_negative: z.number().int().min(0),
    true_negative: z.number().int().min(0),
  })
  .strict();
const recallSchema = z
  .object({
    dimension: dimensionSchema,
    key: z.string().min(1),
    recovered: z.number().int().min(0),
    expected: z.number().int().min(1),
    ratio: z.number().min(0).max(1),
  })
  .strict();
const populationSchema = z
  .object({
    state: z.enum(["measured", "empty", "unreadable", "unsupported"]),
    files_seen: z.number().int().min(0),
    supported_files: z.number().int().min(0),
    unreadable_files: z.number().int().min(0),
    unsupported_files: z.number().int().min(0),
    census: z
      .object({
        languages: z.array(censusItemSchema),
        node_kinds: z.array(censusItemSchema),
        relation_kinds: z.array(censusItemSchema),
        resolver_tiers: z.array(censusItemSchema),
      })
      .strict(),
  })
  .strict();
const resultsSchema = z
  .object({
    confusion_matrices: z.array(confusionMatrixSchema).min(4),
    unresolved: z.array(dimensionCountSchema),
    ambiguous: z.array(dimensionCountSchema),
    recall: z.array(recallSchema).min(1),
  })
  .strict();
const graphQualityObservationSchema = z
  .object({
    schema_version: z.literal(1),
    record_type: z.literal("graph_quality_observation"),
    observation_id: digest,
    producer: z
      .object({
        extractor_revision: revisionIdentity,
        producer_contract_version: z.number().int().min(1),
        parser_grammars: z
          .array(
            z
              .object({
                language: z.enum(["rust", "typescript", "tsx", "python"]),
                grammar: z.string().min(1),
                revision: revisionIdentity,
              })
              .strict(),
          )
          .min(1),
        configuration_digest: digest,
        source_revision: revisionIdentity,
        corpus_revision: revisionIdentity,
        scorer_version: revisionIdentity,
      })
      .strict(),
    measurement_plan: z
      .object({
        ref: z.literal("ix://agent-ix/quire-code-rs/MP-001"),
        definition_version: z.literal("quire-code.graph-quality-v1"),
      })
      .strict(),
    population: populationSchema,
    results: resultsSchema.optional(),
    raw_scorer_output: z
      .object({
        path: z
          .string()
          .min(1)
          .regex(/^(?!\/)(?![A-Za-z]:).+$/),
        digest,
      })
      .strict(),
  })
  .strict()
  .superRefine((record, context) => {
    const { population, results } = record;
    if (population.state === "measured") {
      if (!results)
        context.addIssue({
          code: "custom",
          path: ["results"],
          message: "measured population requires results",
        });
      if (population.supported_files < 1 || population.unreadable_files !== 0)
        context.addIssue({
          code: "custom",
          path: ["population"],
          message:
            "measured population requires supported files and no unreadable files",
        });
    } else if (results) {
      context.addIssue({
        code: "custom",
        path: ["results"],
        message: `${population.state} population must omit results`,
      });
    }
    if (
      population.state === "empty" &&
      (population.files_seen !== 0 ||
        population.supported_files !== 0 ||
        population.unreadable_files !== 0 ||
        population.unsupported_files !== 0)
    )
      context.addIssue({
        code: "custom",
        path: ["population"],
        message: "empty population requires all file counts to be zero",
      });
    if (population.state === "unreadable" && population.unreadable_files < 1)
      context.addIssue({
        code: "custom",
        path: ["population", "unreadable_files"],
        message: "unreadable population requires at least one unreadable file",
      });
    if (
      population.state === "unsupported" &&
      (population.files_seen < 1 ||
        population.supported_files !== 0 ||
        population.unreadable_files !== 0 ||
        population.unsupported_files < 1)
    )
      context.addIssue({
        code: "custom",
        path: ["population"],
        message: "unsupported population has inconsistent file counts",
      });
  });

export type GraphQualityObservationV1 = z.infer<
  typeof graphQualityObservationSchema
>;

const verificationStackSchema = z
  .object({
    schemaVersion: z.literal("verification-stack-attestation-v1"),
    lockDigest: digest,
    executableDigest: digest,
    buildProfile: z.literal("release"),
    toolchains: z
      .object({
        node: z.string().min(1),
        rust: z.string().min(1),
        python: z.string().min(1),
      })
      .strict(),
    sources: z.record(
      z.string(),
      z
        .object({
          revision: fullRevision,
          sourceState: z.literal("clean"),
          remote: z.string().min(1),
        })
        .strict(),
    ),
    capabilities: z.array(z.string().min(1)).min(1),
    artifacts: z.record(z.string(), digest),
  })
  .strict();
const invocationAttestationSchema = z
  .object({
    subject: z.string().min(1),
    scope: z.unknown(),
    timestamp: z.string().min(1),
    environment: z.record(z.string(), z.string()),
    verificationStack: verificationStackSchema,
  })
  .strict()
  .superRefine((value, context) => {
    if (!Number.isFinite(Date.parse(value.timestamp)))
      context.addIssue({
        code: "custom",
        path: ["timestamp"],
        message: "timestamp is not a valid instant",
      });
  });

export interface InvocationAttestation {
  subject: string;
  scope: unknown;
  timestamp: string;
  environment: Record<string, string>;
  verificationStack: VerificationStackAttestation;
}

export function selectGraphAdapter(name: string): GraphAdapterName {
  if ((GRAPH_ADAPTER_NAMES as readonly string[]).includes(name))
    return name as GraphAdapterName;
  throw new GraphAdapterError(
    "unknown_adapter",
    `unknown adapter \`${name}\`; available: ${GRAPH_ADAPTER_NAMES.join(", ")}`,
  );
}

/** Validate and expose Quire's authoritative export without translating it. */
export function adaptQuireAssurance(
  value: unknown,
  accepted: AcceptedQuirePremises,
): QuireAssuranceV1 {
  const parsed = parseOrThrow(quireAssuranceSchema, value, "invalid_premise");
  if (parsed.format !== accepted.format)
    premiseFailure("format", accepted.format, parsed.format);
  if (parsed.format_version !== accepted.formatVersion)
    premiseFailure(
      "format_version",
      accepted.formatVersion,
      parsed.format_version,
    );
  requireEqualPremise("source", accepted.source, parsed.source);
  requireEqualPremise("modules", accepted.modules, parsed.modules);
  return parsed;
}

/** SHA-256 identity of canonical producer content with observation_id omitted. */
export function graphQualityObservationId(value: unknown): string {
  const record = isRecord(value) ? { ...value } : value;
  if (isRecord(record)) delete record.observation_id;
  const bytes = canonicalJson(record).trimEnd();
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

export interface AdaptGraphQualityInput {
  record: unknown;
  scorerBytes: Uint8Array;
  scorerMediaType: string;
  attestation: unknown;
  plans: MeasurementPlan[];
}

/** Transcribe a retained producer observation into one governed collection. */
export function adaptGraphQualityObservation(
  input: AdaptGraphQualityInput,
): MeasurementCollection {
  const record = parseOrThrow(
    graphQualityObservationSchema,
    input.record,
    "invalid_observation",
  );
  const expectedId = graphQualityObservationId(record);
  if (record.observation_id !== expectedId)
    throw new GraphAdapterError(
      "invalid_observation",
      `observation_id expected ${expectedId}; observed ${record.observation_id}`,
    );
  if (!(input.scorerBytes instanceof Uint8Array))
    throw new GraphAdapterError(
      "attachment_missing",
      "scorer attachment bytes are required",
    );
  if (!input.scorerMediaType)
    throw new GraphAdapterError(
      "attachment_missing",
      "scorer attachment media type is required",
    );
  const scorerDigest = `sha256:${createHash("sha256")
    .update(input.scorerBytes)
    .digest("hex")}`;
  if (scorerDigest !== record.raw_scorer_output.digest)
    throw new GraphAdapterError(
      "attachment_digest_mismatch",
      `raw_scorer_output.digest expected ${record.raw_scorer_output.digest}; observed ${scorerDigest}`,
    );
  const attestation = parseOrThrow(
    invocationAttestationSchema,
    input.attestation,
    "invalid_attestation",
  ) as InvocationAttestation;
  const planId = record.measurement_plan.ref.split("/").at(-1) as string;
  const plan = input.plans.find(
    (candidate) => candidate.metric === "graph_quality",
  );
  if (
    !plan ||
    plan.status !== "active" ||
    plan.id !== planId ||
    plan.definitionVersion !== record.measurement_plan.definition_version
  )
    throw new GraphAdapterError(
      "inactive_plan",
      `expected active ${planId}/${record.measurement_plan.definition_version}; observed ${
        plan
          ? `${plan.id}/${plan.definitionVersion}/${plan.status}`
          : "no graph_quality plan"
      }`,
    );

  const observations = normalizeGraphQuality(record, planId);
  const collection: MeasurementCollection = {
    schemaVersion: MEASUREMENT_SCHEMA_VERSION,
    collectionId: `graph-quality-${record.observation_id.slice("sha256:".length)}`,
    subject: attestation.subject,
    scope: attestation.scope,
    toolIdentity: "agent-ix/quire-code-rs",
    toolVersion: record.producer.extractor_revision,
    configDigest: record.producer.configuration_digest,
    timestamp: attestation.timestamp,
    sourceRevision: record.producer.source_revision,
    corpusRevision: record.producer.corpus_revision,
    environment: attestation.environment,
    verificationStack: attestation.verificationStack,
    observations,
    rawEvidence: {
      producer: record,
      scorer: {
        path: record.raw_scorer_output.path,
        digest: record.raw_scorer_output.digest,
        mediaType: input.scorerMediaType,
        bytesBase64: Buffer.from(input.scorerBytes).toString("base64"),
      },
    },
  };
  validateMeasurementCollection(collection, input.plans);
  return collection;
}

function normalizeGraphQuality(
  record: GraphQualityObservationV1,
  planId: string,
): MeasurementObservation[] {
  const out: MeasurementObservation[] = [];
  const population = {
    examined: record.population.files_seen,
    matched: record.population.supported_files,
    complete: record.population.state === "measured",
    identity: record.population,
  };
  const count = (
    measure: string,
    dimension: string,
    key: string,
    value: number,
  ): void => {
    out.push({
      metric: "graph_quality",
      planId,
      definitionVersion: record.measurement_plan.definition_version,
      state: "measured",
      value,
      unit: "count",
      shape: "count",
      population,
      dimensions: { measure, dimension, key },
    });
  };

  for (const key of [
    "files_seen",
    "supported_files",
    "unreadable_files",
    "unsupported_files",
  ] as const)
    count("population", "overall", key, record.population[key]);
  for (const [dimension, rows] of Object.entries(record.population.census))
    for (const row of rows) count("census", dimension, row.key, row.count);

  if (record.population.state !== "measured") {
    out.push({
      metric: "graph_quality",
      planId,
      definitionVersion: record.measurement_plan.definition_version,
      state: "not_computed",
      value: null,
      unit: "state",
      shape: "scalar",
      population,
      reason: record.population.state,
      dimensions: {
        measure: "quality_state",
        dimension: "overall",
        key: record.population.state,
      },
    });
    return uniqueAndSort(out);
  }

  for (const row of record.results?.confusion_matrices ?? [])
    for (const component of [
      "true_positive",
      "false_positive",
      "false_negative",
      "true_negative",
    ] as const)
      count(
        "confusion_matrix",
        row.dimension,
        `${row.key}.${component}`,
        row[component],
      );
  for (const row of record.results?.unresolved ?? [])
    count("unresolved", row.dimension, row.key, row.count);
  for (const row of record.results?.ambiguous ?? [])
    count("ambiguous", row.dimension, row.key, row.count);
  for (const row of record.results?.recall ?? [])
    out.push({
      metric: "graph_quality",
      planId,
      definitionVersion: record.measurement_plan.definition_version,
      state: "measured",
      value: row.ratio,
      unit: "ratio",
      shape: "ratio",
      population: {
        examined: row.expected,
        matched: row.recovered,
        complete: true,
        identity: population.identity,
      },
      dimensions: {
        measure: "recall",
        dimension: row.dimension,
        key: row.key,
      },
    });
  return uniqueAndSort(out);
}

function uniqueAndSort(
  observations: MeasurementObservation[],
): MeasurementObservation[] {
  const seen = new Set<string>();
  for (const observation of observations) {
    const key = canonicalJson(observation.dimensions ?? {}).trimEnd();
    if (seen.has(key))
      throw new GraphAdapterError(
        "duplicate_partition",
        `more than one producer fact maps to ${key}`,
      );
    seen.add(key);
  }
  return observations.sort((a, b) =>
    compare(
      canonicalJson(a.dimensions ?? {}),
      canonicalJson(b.dimensions ?? {}),
    ),
  );
}

function parseOrThrow<T>(
  schema: z.ZodType<T>,
  value: unknown,
  code: GraphAdapterErrorCode,
): T {
  const result = schema.safeParse(value);
  if (result.success) return result.data;
  const detail = result.error.issues
    .map((issue) => `${issue.path.join(".") || "<root>"}: ${issue.message}`)
    .join("; ");
  throw new GraphAdapterError(code, detail);
}

function requireEqualPremise(
  name: string,
  expected: unknown,
  observed: unknown,
): void {
  if (canonicalJson(expected) !== canonicalJson(observed))
    premiseFailure(name, expected, observed);
}

function premiseFailure(
  name: string,
  expected: unknown,
  observed: unknown,
): never {
  throw new GraphAdapterError(
    "invalid_premise",
    `${name} expected ${canonicalJson(expected).trimEnd()}; observed ${canonicalJson(observed).trimEnd()}`,
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
