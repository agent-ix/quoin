import { canonicalJson } from "../evidence/store.js";
import { compareMeasurementCollections } from "./compare.js";
import { loadMeasurementPlans } from "./plans.js";
import { measurementPath, readMeasurementCollections } from "./store.js";
import type {
  MeasurementCollection,
  MeasurementComparison,
  MeasurementObservation,
  MeasurementPlan,
} from "./types.js";

export interface MeasurementReport {
  plans: ReturnType<typeof loadMeasurementPlans>;
  current: Array<{
    metric: string;
    planId: string;
    planPath: string;
    planDefinitionVersion: string;
    stage: string;
    observation: MeasurementObservation | null;
    collection:
      | (Pick<
          MeasurementCollection,
          | "collectionId"
          | "timestamp"
          | "toolIdentity"
          | "toolVersion"
          | "configDigest"
          | "sourceRevision"
          | "corpusRevision"
        > & { path: string })
      | null;
  }>;
  corpusGaps: number | null;
}

export function buildMeasurementReport(repo: string): MeasurementReport {
  return buildMeasurementReportFrom(
    repo,
    loadMeasurementPlans(repo),
    readMeasurementCollections(repo),
  );
}

/** Build every view from one already-loaded plan and collection snapshot. */
export function buildMeasurementReportFrom(
  repo: string,
  allPlans: MeasurementPlan[],
  collections: MeasurementCollection[],
): MeasurementReport {
  const plans = allPlans.filter((plan) => plan.status === "active");
  const current = plans.flatMap((plan) => {
    const collection = [...collections]
      .reverse()
      .find((candidate) =>
        candidate.observations.some(
          (observation) => observation.metric === plan.metric,
        ),
      );
    const observations =
      collection?.observations.filter(
        (observation) => observation.metric === plan.metric,
      ) ?? [];
    const rows = observations.length > 0 ? observations : [null];
    return rows.map((observation) => ({
      metric: plan.metric,
      planId: plan.id,
      planPath: plan.path,
      planDefinitionVersion: plan.definitionVersion,
      stage: plan.stage,
      observation,
      collection: collection
        ? {
            collectionId: collection.collectionId,
            timestamp: collection.timestamp,
            toolIdentity: collection.toolIdentity,
            toolVersion: collection.toolVersion,
            configDigest: collection.configDigest,
            sourceRevision: collection.sourceRevision,
            corpusRevision: collection.corpusRevision,
            path: measurementPath(repo, collection.collectionId),
          }
        : null,
    }));
  });
  return { plans, current, corpusGaps: latestGapCount(collections) };
}

export function renderMeasurementReport(report: MeasurementReport): string {
  const lines = [
    "# QA measurement report",
    "",
    "## What this repository measures",
    "",
  ];
  if (report.plans.length === 0)
    lines.push("No MeasurementPlan is authored.", "");
  else {
    lines.push(
      "| Metric | Plan | Stage | Current |",
      "| --- | --- | --- | --- |",
    );
    for (const row of report.current) {
      const dimensions = Object.entries(row.observation?.dimensions ?? {})
        .map(([key, value]) => `${key}=${value}`)
        .join(", ");
      const metric = dimensions ? `${row.metric} [${dimensions}]` : row.metric;
      const value = row.observation
        ? row.observation.definitionVersion !== row.planDefinitionVersion
          ? `incomparable: definition ${row.observation.definitionVersion}; active plan ${row.planDefinitionVersion}`
          : row.observation.state === "measured"
            ? `${row.observation.value} ${row.observation.unit}`
            : `not_computed: ${row.observation.reason ?? "producer did not compute it"}`
        : "not_computed: no record";
      lines.push(
        `| ${metric} | ${row.planId} (${row.planPath}) | ${row.stage} | ${value} |`,
      );
    }
    lines.push("");
  }
  lines.push("## Current evidence", "");
  lines.push(
    report.corpusGaps === null
      ? "Corpus gaps: not_computed"
      : `Corpus gaps: ${report.corpusGaps}`,
    "",
  );
  const collections = new Map(
    report.current
      .filter((row) => row.collection)
      .map((row) => [row.collection?.collectionId, row.collection]),
  );
  for (const collection of collections.values()) {
    if (!collection) continue;
    lines.push(
      `- ${collection.timestamp} — ${collection.toolIdentity} ${collection.toolVersion}; ` +
        `source ${collection.sourceRevision}; corpus ${collection.corpusRevision ?? "n/a"}; ` +
        `config ${collection.configDigest}`,
    );
  }
  if (collections.size === 0) lines.push("No measurement collection recorded.");
  lines.push("");
  lines.push("## Attention", "");
  const attention: string[] = [];
  if ((report.corpusGaps ?? 0) > 0) {
    attention.push(
      `${report.corpusGaps} mode-language cells are GAP; run the corpus bounds view to see each named cell.`,
    );
  }
  for (const row of report.current) {
    const observation = row.observation;
    if (!observation) {
      attention.push(
        `${row.metric}: no collection has computed this authored plan.`,
      );
      continue;
    }
    const dimensions = Object.entries(observation.dimensions ?? {})
      .map(([key, value]) => `${key}=${value}`)
      .join(", ");
    const name = dimensions ? `${row.metric} [${dimensions}]` : row.metric;
    if (observation.state === "not_computed") {
      attention.push(
        `${name}: not computed — ${observation.reason ?? "no reason supplied"}.`,
      );
    } else if (observation.definitionVersion !== row.planDefinitionVersion) {
      attention.push(
        `${name}: stored definition ${observation.definitionVersion} does not match active plan ${row.planDefinitionVersion}; do not compare the value.`,
      );
    } else if (row.metric === "finding_recall" && observation.value === 0) {
      attention.push(
        `${name}: zero seeded defects detected; add or connect the detector named by the family.`,
      );
    } else if (
      row.metric === "finding_precision.unadjudicated" &&
      (observation.value ?? 0) > 0
    ) {
      attention.push(
        `${name}: ${observation.value} firings have no corpus ruling; adjudicate their cases.`,
      );
    } else if (
      row.metric === "actionability_rate" &&
      observation.population?.examined !== undefined
    ) {
      attention.push(
        `${name}: ${observation.population.matched ?? 0} of ${observation.population.examined} findings name a row or line; inspect emitted findings without a locus.`,
      );
    }
  }
  lines.push(
    ...(attention.length > 0
      ? attention.map((item) => `- ${item}`)
      : ["No factual attention items."]),
    "",
  );
  return lines.join("\n");
}

export function renderMeasurementReportJson(report: MeasurementReport): string {
  return canonicalJson(report);
}

export function seriesFor(repo: string, metric: string): unknown {
  return readMeasurementCollections(repo).flatMap((collection) =>
    collection.observations
      .filter((observation) => observation.metric === metric)
      .map((observation) => ({
        timestamp: collection.timestamp,
        sourceRevision: collection.sourceRevision,
        toolVersion: collection.toolVersion,
        toolIdentity: collection.toolIdentity,
        configDigest: collection.configDigest,
        corpusRevision: collection.corpusRevision ?? null,
        corpusGaps: gapCount(collection),
        observation,
      })),
  );
}

export interface MeasurementCollectionReference {
  collectionId: string;
  timestamp: string;
  toolIdentity: string;
  toolVersion: string;
  configDigest: string;
  sourceRevision: string;
  corpusRevision: string | null;
  corpusGaps: number | null;
  path: string;
}

export interface MeasurementComparisonReport {
  status: "compared" | "not_computed";
  reason: string | null;
  before: MeasurementCollectionReference | null;
  after: MeasurementCollectionReference | null;
  comparisons: MeasurementComparison[];
}

export function comparisonFor(
  repo: string,
  beforeRevision: string,
): MeasurementComparisonReport {
  const collections = readMeasurementCollections(repo);
  const before = [...collections]
    .reverse()
    .find((c) => c.sourceRevision.startsWith(beforeRevision));
  const after = collections.at(-1);
  if (!after) {
    return {
      status: "not_computed",
      reason: "no current measurement collection is recorded",
      before: null,
      after: null,
      comparisons: [],
    };
  }
  if (!before) {
    return {
      status: "not_computed",
      reason: `no baseline measurement collection for source revision ${beforeRevision}`,
      before: null,
      after: collectionReference(repo, after),
      comparisons: [],
    };
  }
  return {
    status: "compared",
    reason: null,
    before: collectionReference(repo, before),
    after: collectionReference(repo, after),
    comparisons: compareMeasurementCollections(before, after),
  };
}

export function renderMeasurementComparison(
  report: MeasurementComparisonReport,
): string {
  const lines = [
    "# QA measurement comparison",
    "",
    `Status: ${report.status}`,
    "",
  ];
  if (report.before) lines.push(`Before: ${renderReference(report.before)}`);
  else lines.push("Before: not_computed — no baseline");
  if (report.after) lines.push(`After: ${renderReference(report.after)}`);
  else lines.push("After: not_computed — no current record");
  lines.push("");
  if (report.reason) {
    lines.push(`not_computed: ${report.reason}`, "");
    return lines.join("\n");
  }
  lines.push(
    "| Metric | Status | Before | After | Delta |",
    "| --- | --- | ---: | ---: | ---: |",
  );
  for (const comparison of report.comparisons) {
    const dimensions = Object.entries(comparison.dimensions)
      .map(([key, value]) => `${key}=${value}`)
      .join(", ");
    const metric = dimensions
      ? `${comparison.metric} [${dimensions}]`
      : comparison.metric;
    lines.push(
      `| ${metric} | ${comparison.status} | ${comparison.before ?? "n/a"} | ` +
        `${comparison.after ?? "n/a"} | ${comparison.delta ?? "n/a"} |`,
    );
    for (const reason of comparison.reasons) {
      lines.push(
        `| ↳ ${reason.code} | ${reason.blocking ? "blocks delta" : "context"} |  |  | ${reason.message} |`,
      );
    }
  }
  lines.push("");
  return lines.join("\n");
}

function collectionReference(
  repo: string,
  collection: MeasurementCollection,
): MeasurementCollectionReference {
  return {
    collectionId: collection.collectionId,
    timestamp: collection.timestamp,
    toolIdentity: collection.toolIdentity,
    toolVersion: collection.toolVersion,
    configDigest: collection.configDigest,
    sourceRevision: collection.sourceRevision,
    corpusRevision: collection.corpusRevision ?? null,
    corpusGaps: gapCount(collection),
    path: measurementPath(repo, collection.collectionId),
  };
}

function renderReference(reference: MeasurementCollectionReference): string {
  return (
    `${reference.collectionId} — ${reference.toolIdentity} ${reference.toolVersion}; ` +
    `source ${reference.sourceRevision}; corpus ${reference.corpusRevision ?? "n/a"}; ` +
    `gaps ${reference.corpusGaps ?? "not_computed"}; config ${reference.configDigest}; ` +
    `record ${reference.path}`
  );
}

function gapCount(collection: MeasurementCollection): number | null {
  const raw = collection.rawEvidence as {
    bounds?: { gap_count?: unknown };
  } | null;
  return typeof raw?.bounds?.gap_count === "number"
    ? raw.bounds.gap_count
    : null;
}

function latestGapCount(collections: MeasurementCollection[]): number | null {
  for (const collection of [...collections].reverse()) {
    const gaps = gapCount(collection);
    if (gaps !== null) return gaps;
  }
  return null;
}
