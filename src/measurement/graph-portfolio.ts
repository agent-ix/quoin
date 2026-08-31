/** Pure governed graph portfolio projection (FR-067). */

import { canonicalJson } from "../evidence/store.js";
import { resolve } from "node:path";
import type { PortfolioRepositoryReport } from "./portfolio.js";
import type {
  MeasurementCollection,
  MeasurementObservation,
  MeasurementPlan,
} from "./types.js";

export type GraphAvailability =
  | "available"
  | "missing"
  | "unreadable"
  | "unknown"
  | "incompatible"
  | "not_applicable";

export interface GraphPortfolioGap {
  availability: Exclude<GraphAvailability, "available">;
  subject: "repository" | "collection" | "attachment" | "graph_export";
  path: string | null;
  reason: string;
  owner?: string;
  action?: string;
}

export interface GraphCollectionRead {
  path: string;
  collection?: MeasurementCollection;
  availability?: Exclude<GraphAvailability, "available" | "not_applicable">;
  error?: string;
}

/**
 * Opaque FR-062 results. The portfolio retains object identity and never owns
 * or interprets the graph report contract.
 */
export type InjectedStructuralGraph =
  | {
      availability: "available";
      path?: string;
      premises: unknown;
      fanOut: unknown;
      churn: unknown;
      changeImpact?: readonly unknown[];
    }
  | {
      availability: Exclude<GraphAvailability, "available">;
      path?: string;
      reason: string;
    };

export interface GraphPortfolioRepositoryInput {
  portfolio: PortfolioRepositoryReport;
  plans: MeasurementPlan[];
  collections: GraphCollectionRead[];
  graph: InjectedStructuralGraph;
}

export type GraphPortfolioMappingErrorCode =
  | "invalid_repository_mapping"
  | "duplicate_graph_export"
  | "duplicate_graph_premises"
  | "duplicate_graph_audit";

export class GraphPortfolioMappingError extends Error {
  constructor(
    readonly code: GraphPortfolioMappingErrorCode,
    message: string,
  ) {
    super(`${code}: ${message}`);
    this.name = "GraphPortfolioMappingError";
  }
}

export interface GraphPortfolioMappingOptions {
  graphExports?: readonly string[];
  graphPremises?: readonly string[];
  graphAudits?: readonly string[];
  changed?: readonly string[];
  cwd?: string;
}

export type GraphPortfolioResolvedMapping =
  | {
      root: string;
      status: "ready";
      exportPath: string;
      premisesPath: string;
      auditPath: string;
      changed: string[];
    }
  | {
      root: string;
      status: "missing" | "incompatible";
      reason: string;
      changed: string[];
    };

export interface GraphPartitionRow {
  measure: string;
  dimension: string;
  key: string;
  state: MeasurementObservation["state"];
  value: number | null;
  unit: string;
  shape: MeasurementObservation["shape"];
  reason?: string;
}

export interface GraphQualityHistoryRow {
  id: string;
  path: string;
  timestamp: string;
  availability: "available" | "incompatible" | "unknown";
  reason: string | null;
  plan: { id: string; definitionVersion: string } | null;
  toolIdentity: string;
  toolVersion: string;
  configDigest: string;
  sourceRevision: string;
  corpusRevision: string | null;
  populationIdentity: unknown;
  producer: unknown;
  producerRecordDigest: string | null;
  scorerDigest: string | null;
  partitions: GraphPartitionRow[];
}

export type GraphCompatibilityCode =
  | "plan_changed"
  | "definition_changed"
  | "configuration_changed"
  | "tool_changed"
  | "corpus_changed"
  | "population_changed";

export interface GraphCompatibilityReason {
  code: GraphCompatibilityCode;
  blocking: true;
  message: string;
}

export interface GraphQualityComparisonRow {
  measure: string;
  dimension: string;
  key: string;
  before: number | null;
  after: number | null;
  delta: number | null;
  status: "comparable" | "incomparable" | "not_computed";
  reasons: GraphCompatibilityReason[];
}

export interface GraphQualityComparison {
  before: GraphQualityHistoryRow;
  after: GraphQualityHistoryRow;
  observations: GraphQualityComparisonRow[];
}

export interface GovernedGraphRepositoryReport extends PortfolioRepositoryReport {
  graphQuality: {
    plan: MeasurementPlan | null;
    current: GraphQualityHistoryRow | null;
    history: GraphQualityHistoryRow[];
    comparison: GraphQualityComparison | null;
  };
  graph: NormalizedStructuralGraph;
  gaps: GraphPortfolioGap[];
}

export type NormalizedStructuralGraph =
  | {
      availability: "available";
      path?: string;
      premises: unknown;
      fanOut: unknown;
      churn: unknown;
      changeImpact:
        readonly unknown[] | { availability: "not_applicable"; reason: string };
    }
  | {
      availability: Exclude<GraphAvailability, "available">;
      path?: string;
      reason: string;
      changeImpact: { availability: "not_applicable"; reason: string };
    };

export interface GovernedGraphPortfolioReport {
  schemaVersion: 1;
  repositories: GovernedGraphRepositoryReport[];
}

/** Resolve all mappings before any repository or graph input is read. */
export function parseGraphPortfolioMappings(
  locations: readonly string[],
  options: GraphPortfolioMappingOptions = {},
): GraphPortfolioResolvedMapping[] {
  const cwd = options.cwd ?? process.cwd();
  const roots = [
    ...new Set(locations.map((location) => resolve(cwd, location))),
  ].sort(compare);
  const known = new Set(roots);
  const exports = resolvePathMappings(
    options.graphExports ?? [],
    "duplicate_graph_export",
    known,
    cwd,
  );
  const premises = resolvePathMappings(
    options.graphPremises ?? [],
    "duplicate_graph_premises",
    known,
    cwd,
  );
  const audits = resolvePathMappings(
    options.graphAudits ?? [],
    "duplicate_graph_audit",
    known,
    cwd,
  );
  const changed = resolveChangedMappings(options.changed ?? [], known, cwd);
  return roots.map((root) => {
    const exportPath = exports.get(root);
    const premisesPath = premises.get(root);
    const auditPath = audits.get(root);
    const seeds = [...(changed.get(root) ?? new Set<string>())].sort(compare);
    const present = [exportPath, premisesPath, auditPath].filter(
      Boolean,
    ).length;
    if (present === 0)
      return {
        root,
        status: "missing" as const,
        reason: "no graph export, premises, or audit mapping was supplied",
        changed: seeds,
      };
    if (present !== 3)
      return {
        root,
        status: "incompatible" as const,
        reason:
          "graph structural reporting requires export, premises, and audit mappings",
        changed: seeds,
      };
    return {
      root,
      status: "ready" as const,
      exportPath: exportPath as string,
      premisesPath: premisesPath as string,
      auditPath: auditPath as string,
      changed: seeds,
    };
  });
}

export function buildGovernedGraphPortfolioFrom(
  inputs: readonly GraphPortfolioRepositoryInput[],
): GovernedGraphPortfolioReport {
  const repositories = [...inputs]
    .sort((a, b) => compare(a.portfolio.root, b.portfolio.root))
    .map(buildRepository);
  return { schemaVersion: 1, repositories };
}

export function compareGraphQualityCollections(
  before: MeasurementCollection,
  after: MeasurementCollection,
): GraphQualityComparisonRow[] {
  const left = graphObservations(before);
  const right = graphObservations(after);
  const keys = new Set([...left.keys(), ...right.keys()]);
  return [...keys].sort(compare).map((key) => {
    const a = left.get(key);
    const b = right.get(key);
    const sample = a ?? b;
    if (!sample) throw new Error("graph partition index lost its sample");
    if (!a || !b || a.state === "not_computed" || b.state === "not_computed") {
      return {
        ...partitionIdentity(sample),
        before: a?.value ?? null,
        after: b?.value ?? null,
        delta: null,
        status: "not_computed",
        reasons: [],
      };
    }
    const reasons = compatibilityReasons(before, after, a, b);
    return {
      ...partitionIdentity(a),
      before: a.value,
      after: b.value,
      delta:
        reasons.length === 0 && a.value !== null && b.value !== null
          ? b.value - a.value
          : null,
      status: reasons.length === 0 ? "comparable" : "incomparable",
      reasons,
    };
  });
}

export function canonicalGraphPortfolioJson(
  report: GovernedGraphPortfolioReport,
): string {
  return canonicalJson(report);
}

export function renderGovernedGraphPortfolio(
  report: GovernedGraphPortfolioReport,
): string {
  const lines = [
    "# Governed graph portfolio",
    "",
    "No partition or repository is aggregated and no quality verdict is derived.",
    "",
  ];
  for (const repository of report.repositories) {
    lines.push(`## ${repository.name}`, "", `Root: ${repository.root}`);
    lines.push(`Graph export: ${repository.graph.availability}`);
    const current = repository.graphQuality.current;
    if (!current) lines.push("Graph quality: not_computed");
    else {
      lines.push(
        `Graph quality: ${current.id}; ${current.availability}; ` +
          `producer ${current.producerRecordDigest ?? "unknown"}; scorer ${current.scorerDigest ?? "unknown"}`,
        "",
        "| Measure | Dimension | Key | State/value |",
        "| --- | --- | --- | --- |",
      );
      for (const row of current.partitions)
        lines.push(
          `| ${row.measure} | ${row.dimension} | ${row.key} | ` +
            `${row.state === "measured" ? `${row.value} ${row.unit}` : `not_computed: ${row.reason ?? "unknown"}`} |`,
        );
    }
    if (repository.gaps.length > 0) {
      lines.push("", "Gaps:");
      for (const gap of repository.gaps)
        lines.push(
          `- ${gap.availability}: ${gap.path ?? gap.subject} — ${gap.reason}`,
        );
    }
    lines.push("");
  }
  return lines.join("\n");
}

function buildRepository(
  input: GraphPortfolioRepositoryInput,
): GovernedGraphRepositoryReport {
  const activePlan =
    input.plans.find(
      (plan) => plan.status === "active" && plan.metric === "graph_quality",
    ) ?? null;
  const usable = input.collections.filter(
    (
      read,
    ): read is GraphCollectionRead & { collection: MeasurementCollection } =>
      read.collection !== undefined,
  );
  const historyPairs = usable
    .filter(({ collection }) =>
      collection.observations.some((row) => row.metric === "graph_quality"),
    )
    .map(({ path, collection }) => ({
      collection,
      row: historyRow(path, collection, activePlan),
    }))
    .sort(
      (a, b) =>
        compare(a.row.timestamp, b.row.timestamp) ||
        compare(a.row.id, b.row.id),
    );
  const history = historyPairs.map(({ row }) => row);
  const current =
    [...history].reverse().find((row) => row.availability === "available") ??
    null;
  const comparison =
    historyPairs.length < 2
      ? null
      : {
          before: historyPairs.at(-2)?.row as GraphQualityHistoryRow,
          after: historyPairs.at(-1)?.row as GraphQualityHistoryRow,
          observations: compareGraphQualityCollections(
            (historyPairs.at(-2) as { collection: MeasurementCollection })
              .collection,
            (historyPairs.at(-1) as { collection: MeasurementCollection })
              .collection,
          ),
        };
  const graph = normalizeGraph(input.graph);
  const gaps: GraphPortfolioGap[] = input.collections
    .filter((read) => !read.collection)
    .map((read) => ({
      availability: read.availability ?? "unknown",
      subject: "collection",
      path: read.path,
      reason: read.error ?? "collection was not readable",
    }));
  for (const row of history) {
    if (row.availability !== "available")
      gaps.push({
        availability: row.availability,
        subject: "collection",
        path: row.path,
        reason: row.reason ?? "collection is not current-compatible",
      });
  }
  if (graph.availability !== "available")
    gaps.push({
      availability: graph.availability,
      subject: "graph_export",
      path: "path" in graph ? (graph.path ?? null) : null,
      reason: graph.reason,
    });
  return {
    ...input.portfolio,
    graphQuality: { plan: activePlan, current, history, comparison },
    graph,
    gaps,
  };
}

function normalizeGraph(
  graph: InjectedStructuralGraph,
): GovernedGraphRepositoryReport["graph"] {
  if (graph.availability === "available")
    return {
      ...graph,
      changeImpact: graph.changeImpact ?? {
        availability: "not_applicable",
        reason: "no changed requirement seed was requested",
      },
    };
  return {
    ...graph,
    changeImpact: {
      availability: "not_applicable",
      reason: "change-impact requires an accepted graph export",
    },
  };
}

function historyRow(
  path: string,
  collection: MeasurementCollection,
  activePlan: MeasurementPlan | null,
): GraphQualityHistoryRow {
  const observations = collection.observations.filter(
    (row) => row.metric === "graph_quality",
  );
  const first = observations[0];
  const raw = asRecord(collection.rawEvidence);
  const producerRecord = asRecord(raw?.producer);
  const producer = producerRecord?.producer;
  const producerRecordDigest = stringField(producerRecord, "observation_id");
  const scorer = asRecord(raw?.scorer);
  const scorerDigest =
    stringField(scorer, "digest") ??
    stringField(asRecord(producerRecord?.raw_scorer_output), "digest");
  const plan = first
    ? { id: first.planId, definitionVersion: first.definitionVersion }
    : null;
  let availability: GraphQualityHistoryRow["availability"] = "available";
  let reason: string | null = null;
  if (!producerRecordDigest || !scorerDigest || producer === undefined) {
    availability = "unknown";
    reason = "raw producer record or scorer identity is unavailable";
  } else if (
    !activePlan ||
    !plan ||
    plan.id !== activePlan.id ||
    plan.definitionVersion !== activePlan.definitionVersion
  ) {
    availability = "incompatible";
    reason = activePlan
      ? `collection plan ${plan?.id ?? "unknown"}/${plan?.definitionVersion ?? "unknown"} does not match active ${activePlan.id}/${activePlan.definitionVersion}`
      : "no active graph_quality MeasurementPlan";
  }
  return {
    id: collection.collectionId,
    path,
    timestamp: collection.timestamp,
    availability,
    reason,
    plan,
    toolIdentity: collection.toolIdentity,
    toolVersion: collection.toolVersion,
    configDigest: collection.configDigest,
    sourceRevision: collection.sourceRevision,
    corpusRevision: collection.corpusRevision ?? null,
    populationIdentity: first?.population?.identity ?? null,
    producer: producer ?? null,
    producerRecordDigest,
    scorerDigest,
    partitions: observations.map(partitionRow).sort(partitionCompare),
  };
}

function compatibilityReasons(
  before: MeasurementCollection,
  after: MeasurementCollection,
  a: MeasurementObservation,
  b: MeasurementObservation,
): GraphCompatibilityReason[] {
  const reasons: GraphCompatibilityReason[] = [];
  mismatch(reasons, "plan_changed", a.planId, b.planId);
  mismatch(
    reasons,
    "definition_changed",
    a.definitionVersion,
    b.definitionVersion,
  );
  mismatch(
    reasons,
    "configuration_changed",
    before.configDigest,
    after.configDigest,
  );
  mismatch(
    reasons,
    "tool_changed",
    `${before.toolIdentity}@${before.toolVersion}`,
    `${after.toolIdentity}@${after.toolVersion}`,
  );
  mismatch(
    reasons,
    "corpus_changed",
    before.corpusRevision ?? null,
    after.corpusRevision ?? null,
  );
  mismatch(
    reasons,
    "population_changed",
    a.population?.identity ?? null,
    b.population?.identity ?? null,
  );
  return reasons;
}

function mismatch(
  reasons: GraphCompatibilityReason[],
  code: GraphCompatibilityCode,
  before: unknown,
  after: unknown,
): void {
  if (canonicalJson(before) === canonicalJson(after)) return;
  reasons.push({
    code,
    blocking: true,
    message: `${code}: ${canonicalJson(before).trimEnd()} -> ${canonicalJson(after).trimEnd()}`,
  });
}

function graphObservations(
  collection: MeasurementCollection,
): Map<string, MeasurementObservation> {
  return new Map(
    collection.observations
      .filter((row) => row.metric === "graph_quality")
      .map((row) => [partitionKey(row), row]),
  );
}

function partitionRow(row: MeasurementObservation): GraphPartitionRow {
  return {
    ...partitionIdentity(row),
    state: row.state,
    value: row.value,
    unit: row.unit,
    shape: row.shape,
    ...(row.reason ? { reason: row.reason } : {}),
  };
}

function partitionIdentity(
  row: MeasurementObservation,
): Pick<GraphPartitionRow, "measure" | "dimension" | "key"> {
  return {
    measure: row.dimensions?.measure ?? "unknown",
    dimension: row.dimensions?.dimension ?? "unknown",
    key: row.dimensions?.key ?? "unknown",
  };
}

function partitionKey(row: MeasurementObservation): string {
  const identity = partitionIdentity(row);
  return `${identity.measure}\0${identity.dimension}\0${identity.key}`;
}

function partitionCompare(a: GraphPartitionRow, b: GraphPartitionRow): number {
  return (
    compare(a.measure, b.measure) ||
    compare(a.dimension, b.dimension) ||
    compare(a.key, b.key)
  );
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function stringField(
  value: Record<string, unknown> | null,
  key: string,
): string | null {
  return typeof value?.[key] === "string" ? value[key] : null;
}

function resolvePathMappings(
  values: readonly string[],
  conflict: Exclude<
    GraphPortfolioMappingErrorCode,
    "invalid_repository_mapping"
  >,
  known: Set<string>,
  cwd: string,
): Map<string, string> {
  const resolved = new Map<string, string>();
  for (const value of values) {
    const [repository, path] = splitMapping(value);
    const root = resolve(cwd, repository);
    if (!known.has(root))
      throw new GraphPortfolioMappingError(
        "invalid_repository_mapping",
        `repository ${root} is not present in --portfolio`,
      );
    const target = resolve(cwd, path);
    const prior = resolved.get(root);
    if (prior !== undefined && prior !== target)
      throw new GraphPortfolioMappingError(
        conflict,
        `${root} maps to both ${prior} and ${target}`,
      );
    resolved.set(root, target);
  }
  return resolved;
}

function resolveChangedMappings(
  values: readonly string[],
  known: Set<string>,
  cwd: string,
): Map<string, Set<string>> {
  const resolved = new Map<string, Set<string>>();
  for (const value of values) {
    const [repository, seed] = splitMapping(value);
    const root = resolve(cwd, repository);
    if (!known.has(root))
      throw new GraphPortfolioMappingError(
        "invalid_repository_mapping",
        `repository ${root} is not present in --portfolio`,
      );
    const seeds = resolved.get(root) ?? new Set<string>();
    seeds.add(seed);
    resolved.set(root, seeds);
  }
  return resolved;
}

function splitMapping(value: string): [string, string] {
  const separator = value.indexOf("=");
  if (separator < 1 || separator === value.length - 1)
    throw new GraphPortfolioMappingError(
      "invalid_repository_mapping",
      `expected <repository>=<value>; observed ${JSON.stringify(value)}`,
    );
  const repository = value.slice(0, separator).trim();
  const target = value.slice(separator + 1).trim();
  if (!repository || !target)
    throw new GraphPortfolioMappingError(
      "invalid_repository_mapping",
      `expected non-empty repository and value; observed ${JSON.stringify(value)}`,
    );
  return [repository, target];
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
