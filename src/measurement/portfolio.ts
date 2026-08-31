import { existsSync, statSync } from "node:fs";
import { basename, resolve } from "node:path";

import { canonicalJson } from "../evidence/store.js";
import { compareMeasurementCollections } from "./compare.js";
import { loadMeasurementPlans } from "./plans.js";
import {
  loadActiveAssuranceProfiles,
  type AssuranceProfileSummary,
} from "./profiles.js";
import {
  buildMeasurementReportFrom,
  type MeasurementReport,
} from "./report.js";
import {
  measurementPath,
  measurementsRoot,
  readMeasurementCollections,
} from "./store.js";
import type {
  MeasurementCollection,
  MeasurementComparison,
  MeasurementPlan,
} from "./types.js";

const DAY_MS = 24 * 60 * 60 * 1000;
export const PORTFOLIO_STALE_AFTER_DAYS = 30;

export interface PortfolioCollectionRef {
  id: string;
  path: string;
  timestamp: string;
  sourceRevision: string;
  toolIdentity: string;
  toolVersion: string;
  configDigest: string;
  corpusRevision?: string;
}

export interface PortfolioComparison {
  before: PortfolioCollectionRef;
  after: PortfolioCollectionRef;
  observations: MeasurementComparison[];
}

export interface PortfolioRepositoryReport {
  name: string;
  root: string;
  status: "readable" | "missing" | "unreadable";
  error: string | null;
  store: "present" | "empty" | "missing" | "unreadable";
  profiles: AssuranceProfileSummary[];
  plans: MeasurementPlan[];
  measurements: MeasurementReport | null;
  latestCollection: PortfolioCollectionRef | null;
  comparison: PortfolioComparison | null;
  staleness: {
    status: "current" | "stale" | "not_computed";
    ageDays: number | null;
    relativeTo: string | null;
    thresholdDays: number;
  };
}

export interface PortfolioReport {
  schemaVersion: 1;
  newestCollectionTimestamp: string | null;
  staleAfterDays: number;
  repositories: PortfolioRepositoryReport[];
}

export interface PortfolioCollectionSnapshot {
  root: string;
  collections: MeasurementCollection[];
}

/** Load repository stores once, then derive both human and JSON views. */
export function buildPortfolioReport(locations: string[]): PortfolioReport {
  const roots = [
    ...new Set(locations.map((location) => resolve(location))),
  ].sort(compare);
  return finalizePortfolioReport(roots.map((root) => loadRepository(root)));
}

/** Build the same FR-045 view from a caller's single-read store snapshots. */
export function buildPortfolioReportFromCollections(
  snapshots: readonly PortfolioCollectionSnapshot[],
): PortfolioReport {
  const byRoot = new Map(
    snapshots.map(({ root, collections }) => [
      resolve(root),
      { root: resolve(root), collections },
    ]),
  );
  return finalizePortfolioReport(
    [...byRoot.values()]
      .sort((a, b) => compare(a.root, b.root))
      .map(({ root, collections }) => loadRepository(root, collections)),
  );
}

function finalizePortfolioReport(
  repositories: PortfolioRepositoryReport[],
): PortfolioReport {
  for (const repository of repositories) {
    const latest = repository.latestCollection;
    if (latest && !Number.isFinite(Date.parse(latest.timestamp))) {
      markUnreadable(
        repository,
        `${latest.path}: collection timestamp is not a valid date`,
      );
    }
  }
  const timestamps = repositories
    .map((repository) => repository.latestCollection?.timestamp)
    .filter((timestamp): timestamp is string => timestamp !== undefined)
    .sort((a, b) => Date.parse(a) - Date.parse(b) || compare(a, b));
  const newestCollectionTimestamp = timestamps.at(-1) ?? null;
  const newest = newestCollectionTimestamp
    ? Date.parse(newestCollectionTimestamp)
    : null;
  for (const repository of repositories) {
    const latest = repository.latestCollection;
    if (!latest || newest === null) continue;
    const timestamp = Date.parse(latest.timestamp);
    const ageDays = Math.floor(Math.max(0, newest - timestamp) / DAY_MS);
    repository.staleness = {
      status: ageDays > PORTFOLIO_STALE_AFTER_DAYS ? "stale" : "current",
      ageDays,
      relativeTo: newestCollectionTimestamp,
      thresholdDays: PORTFOLIO_STALE_AFTER_DAYS,
    };
  }
  return {
    schemaVersion: 1,
    newestCollectionTimestamp,
    staleAfterDays: PORTFOLIO_STALE_AFTER_DAYS,
    repositories,
  };
}

export function renderPortfolioReport(report: PortfolioReport): string {
  const lines = [
    "# QA portfolio report",
    "",
    "No cross-repository metric is summed, averaged, or assigned a quality verdict.",
    "",
  ];
  for (const repository of report.repositories) {
    lines.push(`## ${repository.name}`, "", `Root: ${repository.root}`);
    if (repository.status !== "readable") {
      lines.push(
        `Status: ${repository.status} — ${repository.error ?? "unknown read error"}`,
        "",
      );
      continue;
    }
    lines.push(`Status: readable; measurement store: ${repository.store}`);
    lines.push(
      `Profiles: ${
        repository.profiles.length
          ? repository.profiles
              .map((profile) => `${profile.id} (${profile.path})`)
              .join(", ")
          : "none"
      }`,
    );
    const latest = repository.latestCollection;
    lines.push(
      latest
        ? `Latest: ${latest.timestamp} — ${latest.id} (${latest.path}); ` +
            `${repository.staleness.status}, ${repository.staleness.ageDays} days behind portfolio latest`
        : "Latest: not_computed — no measurement collection",
    );
    if (latest) {
      lines.push(
        `Provenance: source ${latest.sourceRevision}; corpus ${latest.corpusRevision ?? "n/a"}; ` +
          `tool ${latest.toolIdentity} ${latest.toolVersion}; config ${latest.configDigest}`,
      );
    }
    const measurements = repository.measurements;
    lines.push(
      !measurements || measurements.corpusGaps === null || !latest
        ? "Corpus gaps: not_computed"
        : `Corpus gaps: ${measurements.corpusGaps} (${latest.path}; raw evidence, no metric plan)`,
      "",
      "| Metric | State/value | Plan | Collection |",
      "| --- | --- | --- | --- |",
    );
    if (!measurements || measurements.current.length === 0) {
      lines.push("| not_computed | no active MeasurementPlan | n/a | n/a |");
    } else {
      for (const row of measurements.current) {
        const dimensions = Object.entries(row.observation?.dimensions ?? {})
          .map(([key, value]) => `${key}=${value}`)
          .join(", ");
        const metric = dimensions
          ? `${row.metric} [${dimensions}]`
          : row.metric;
        const value = row.observation
          ? row.observation.definitionVersion !== row.planDefinitionVersion
            ? `incomparable: definition ${row.observation.definitionVersion}; active plan ${row.planDefinitionVersion}`
            : row.observation.state === "measured"
              ? `${row.observation.value} ${row.observation.unit}`
              : `not_computed: ${row.observation.reason ?? "producer supplied no reason"}`
          : "not_computed: no collection";
        lines.push(
          `| ${metric} | ${value} | ${row.planId} (${row.planPath}) | ` +
            `${row.collection ? `${row.collection.collectionId} (${row.collection.path})` : "n/a"} |`,
        );
      }
    }
    lines.push("");
    renderComparison(lines, repository);
  }
  return lines.join("\n");
}

export function renderPortfolioReportJson(report: PortfolioReport): string {
  return canonicalJson(report);
}

function loadRepository(
  root: string,
  providedCollections?: MeasurementCollection[],
): PortfolioRepositoryReport {
  const base = emptyRepository(root);
  if (!existsSync(root)) {
    return {
      ...base,
      status: "missing",
      error: `${root}: repository does not exist`,
    };
  }
  try {
    if (!statSync(root).isDirectory()) {
      return {
        ...base,
        status: "unreadable",
        store: "unreadable",
        error: `${root}: repository location is not a directory`,
      };
    }
    const profiles = loadActiveAssuranceProfiles(root);
    const plans = loadMeasurementPlans(root).filter(
      (plan) => plan.status === "active",
    );
    const collections = providedCollections ?? readMeasurementCollections(root);
    const latest = collections.at(-1) ?? null;
    const previous = collections.at(-2) ?? null;
    return {
      ...base,
      store: !existsSync(measurementsRoot(root))
        ? "missing"
        : collections.length === 0
          ? "empty"
          : "present",
      profiles,
      plans,
      measurements: buildMeasurementReportFrom(root, plans, collections),
      latestCollection: latest ? collectionRef(root, latest) : null,
      comparison:
        previous && latest
          ? {
              before: collectionRef(root, previous),
              after: collectionRef(root, latest),
              observations: compareMeasurementCollections(previous, latest),
            }
          : null,
    };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    return {
      ...base,
      status: "unreadable",
      store: "unreadable",
      error: detail,
    };
  }
}

function markUnreadable(
  repository: PortfolioRepositoryReport,
  error: string,
): void {
  repository.status = "unreadable";
  repository.store = "unreadable";
  repository.error = error;
  repository.measurements = null;
  repository.comparison = null;
  repository.latestCollection = null;
}

function emptyRepository(root: string): PortfolioRepositoryReport {
  return {
    name: basename(root) || root,
    root,
    status: "readable",
    error: null,
    store: "missing",
    profiles: [],
    plans: [],
    measurements: null,
    latestCollection: null,
    comparison: null,
    staleness: {
      status: "not_computed",
      ageDays: null,
      relativeTo: null,
      thresholdDays: PORTFOLIO_STALE_AFTER_DAYS,
    },
  };
}

function collectionRef(
  repo: string,
  collection: MeasurementCollection,
): PortfolioCollectionRef {
  return {
    id: collection.collectionId,
    path: measurementPath(repo, collection.collectionId),
    timestamp: collection.timestamp,
    sourceRevision: collection.sourceRevision,
    toolIdentity: collection.toolIdentity,
    toolVersion: collection.toolVersion,
    configDigest: collection.configDigest,
    ...(collection.corpusRevision
      ? { corpusRevision: collection.corpusRevision }
      : {}),
  };
}

function renderComparison(
  lines: string[],
  repository: PortfolioRepositoryReport,
): void {
  const comparison = repository.comparison;
  if (!comparison) {
    lines.push("Comparison: not_computed — fewer than two collections", "");
    return;
  }
  lines.push(
    `Comparison: ${comparison.before.id} (${comparison.before.path}) → ` +
      `${comparison.after.id} (${comparison.after.path})`,
    "",
    "| Metric | Status | Before | After | Reasons | Plan |",
    "| --- | --- | ---: | ---: | --- | --- |",
  );
  for (const observation of comparison.observations) {
    const plan = repository.plans.find(
      (candidate) => candidate.metric === observation.metric,
    );
    const dimensions = Object.entries(observation.dimensions)
      .map(([key, value]) => `${key}=${value}`)
      .join(", ");
    const metric = dimensions
      ? `${observation.metric} [${dimensions}]`
      : observation.metric;
    lines.push(
      `| ${metric} | ${observation.status} | ${observation.before ?? "n/a"} | ` +
        `${observation.after ?? "n/a"} | ` +
        `${observation.reasons.map((reason) => `${reason.code}: ${reason.message}`).join("; ") || "none"} | ` +
        `${plan ? `${plan.id} (${plan.path})` : "not_computed: no active plan"} |`,
    );
  }
  lines.push("");
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
