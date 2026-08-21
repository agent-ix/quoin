/** Generic measurement query, comparison, trend and rendering (FR-044). */

import {
  canonicalJson,
  listMeasurementPaths,
  readMeasurement,
} from "./store.js";
import type { MeasurementRecord } from "./types.js";

export interface MeasurementQuery {
  planId?: string;
  definitionVersion?: string;
  subject?: MeasurementRecord["subject"];
  scope?: MeasurementRecord["scope"];
  sourceRevision?: string;
}

export interface MeasurementPolicy {
  /** Which movement is adverse. */
  direction: "increase-is-regression" | "decrease-is-regression";
  /** Caller-authored tolerance; Quoin supplies no default. */
  tolerance: { kind: "absolute" | "relative"; value: number };
}

export interface MeasurementDelta {
  key: string;
  baseline: number;
  candidate: number;
  delta: number;
  /** `null` when the baseline is zero, where a ratio has no meaning. */
  relativeDelta: number | null;
  unit: string;
  regression: boolean | null;
}

export interface MeasurementIncompatibility {
  key: string;
  reason:
    | "plan-id"
    | "definition-version"
    | "unit"
    | "environment"
    | "sampling"
    | "mixed-revision"
    | "mixed-population"
    | "duplicate-subject"
    | "zero-baseline-for-relative-policy";
  baseline?: string;
  candidate?: string;
}

export interface SamplingCountMismatch {
  key: string;
  baseline: number;
  candidate: number;
}

interface RatchetState {
  requested: boolean;
  baselineApplied: boolean;
}

export type MeasurementComparison =
  | {
      state: "empty-population" | "missing-baseline";
      ratchet: RatchetState;
    }
  | {
      state: "partial-collection";
      missingFromBaseline: string[];
      missingFromCandidate: string[];
      samplingCountMismatches: SamplingCountMismatch[];
      ratchet: RatchetState;
    }
  | {
      state: "incompatible";
      incompatibilities: MeasurementIncompatibility[];
      ratchet: RatchetState;
    }
  | {
      state: "comparable";
      deltas: MeasurementDelta[];
      regression: boolean | null;
      ratchet: RatchetState;
    };

export interface MeasurementTrendPoint {
  collectedAt: string;
  sourceRevision: string;
  value: number;
  unit: string;
  continuity: "start" | "continuous" | "discontinuity";
  discontinuities: MeasurementIncompatibility["reason"][];
}

export type MeasurementTrend =
  | { state: "empty-population"; points: [] }
  | {
      state: "incompatible";
      incompatibilities: MeasurementIncompatibility[];
      points: [];
    }
  | { state: "trend"; points: MeasurementTrendPoint[] };

/** Stable population identity: subject plus its repository/path scope. */
export function measurementKey(record: MeasurementRecord): string {
  return JSON.stringify({
    scope: {
      repository: record.scope.repository,
      path: record.scope.path ?? null,
    },
    subject: { kind: record.subject.kind, id: record.subject.id },
  });
}

/** Read records matching exact authored identities, ordered deterministically. */
export function queryMeasurements(
  repo: string,
  query: MeasurementQuery = {},
): MeasurementRecord[] {
  return listMeasurementPaths(repo, query.planId)
    .map((path) => readMeasurement(path))
    .filter((record): record is MeasurementRecord => record !== null)
    .filter((record) => matches(record, query))
    .sort(byTimeRevisionKey);
}

/**
 * Compare two revision populations without knowing what their measure means.
 *
 * Compatibility precedes arithmetic. A changed definition, unit, environment
 * or sampling identity produces an explicit non-comparison; it can never be
 * mistaken for a zero delta or a clean result.
 */
export function compareMeasurementSets(
  baseline: MeasurementRecord[],
  candidate: MeasurementRecord[],
  policy?: MeasurementPolicy,
): MeasurementComparison {
  const requested = policy !== undefined;
  validatePolicy(policy);
  if (candidate.length === 0) {
    return {
      state: "empty-population",
      ratchet: { requested, baselineApplied: false },
    };
  }
  if (baseline.length === 0) {
    return {
      state: "missing-baseline",
      ratchet: { requested, baselineApplied: false },
    };
  }

  const structural = [
    ...collectionIncompatibilities(baseline, "baseline"),
    ...collectionIncompatibilities(candidate, "candidate"),
  ];
  if (structural.length > 0) return incompatible(structural, requested);

  const before = indexPopulation(baseline);
  const after = indexPopulation(candidate);
  const beforeKeys = [...before.keys()].sort(compareStrings);
  const afterKeys = [...after.keys()].sort(compareStrings);
  const missingFromBaseline = afterKeys.filter((key) => !before.has(key));
  const missingFromCandidate = beforeKeys.filter((key) => !after.has(key));
  if (missingFromBaseline.length > 0 || missingFromCandidate.length > 0) {
    return {
      state: "partial-collection",
      missingFromBaseline,
      missingFromCandidate,
      samplingCountMismatches: [],
      ratchet: { requested, baselineApplied: false },
    };
  }

  const incompatibilities: MeasurementIncompatibility[] = [];
  for (const key of beforeKeys) {
    incompatibilities.push(
      ...pairIncompatibilities(before.get(key)!, after.get(key)!, key),
    );
  }
  if (policy?.tolerance.kind === "relative") {
    for (const key of beforeKeys) {
      const value = before.get(key)!.value;
      if (value === 0) {
        incompatibilities.push({
          key,
          reason: "zero-baseline-for-relative-policy",
          baseline: "0",
        });
      }
    }
  }
  if (incompatibilities.length > 0) {
    return incompatible(incompatibilities, requested);
  }

  const samplingCountMismatches = beforeKeys
    .map((key): SamplingCountMismatch | null => {
      const baselineCount = before.get(key)!.sampling?.sampleCount;
      const candidateCount = after.get(key)!.sampling?.sampleCount;
      if (
        baselineCount === undefined ||
        candidateCount === undefined ||
        baselineCount === candidateCount
      ) {
        return null;
      }
      return { key, baseline: baselineCount, candidate: candidateCount };
    })
    .filter((item): item is SamplingCountMismatch => item !== null);
  if (samplingCountMismatches.length > 0) {
    return {
      state: "partial-collection",
      missingFromBaseline: [],
      missingFromCandidate: [],
      samplingCountMismatches,
      ratchet: { requested, baselineApplied: false },
    };
  }

  const deltas = beforeKeys.map((key) =>
    deltaFor(before.get(key)!, after.get(key)!, key, policy),
  );
  return {
    state: "comparable",
    deltas,
    regression: policy ? deltas.some((delta) => delta.regression) : null,
    ratchet: { requested, baselineApplied: requested },
  };
}

/** Build a time-ordered single-subject trend with visible discontinuities. */
export function measurementTrend(
  records: MeasurementRecord[],
): MeasurementTrend {
  if (records.length === 0) return { state: "empty-population", points: [] };
  const keys = new Set(records.map(measurementKey));
  if (keys.size !== 1) {
    return {
      state: "incompatible",
      incompatibilities: [{ key: "<population>", reason: "mixed-population" }],
      points: [],
    };
  }
  const ordered = [...records].sort(byTimeRevisionKey);
  return {
    state: "trend",
    points: ordered.map((record, index) => {
      const prior = ordered[index - 1];
      const discontinuities = prior
        ? pairIncompatibilities(prior, record, measurementKey(record)).map(
            (issue) => issue.reason,
          )
        : [];
      return {
        collectedAt: record.collectedAt,
        sourceRevision: record.sourceRevision,
        value: record.value,
        unit: record.unit,
        continuity:
          index === 0
            ? "start"
            : discontinuities.length === 0
              ? "continuous"
              : "discontinuity",
        discontinuities,
      };
    }),
  };
}

/** Canonical machine output. */
export function renderMeasurementComparisonJson(
  result: MeasurementComparison,
): string {
  return canonicalJson(result);
}

/** Deterministic review-oriented output. */
export function renderMeasurementComparisonMarkdown(
  result: MeasurementComparison,
): string {
  const lines = [
    "# Measurement comparison",
    "",
    `- State: \`${result.state}\``,
    `- Ratchet requested: ${result.ratchet.requested ? "yes" : "no"}`,
    `- Baseline applied: ${result.ratchet.baselineApplied ? "yes" : "no"}`,
  ];
  if (result.state === "comparable") {
    lines.push(
      `- Regression: ${result.regression === null ? "not evaluated" : result.regression ? "yes" : "no"}`,
      "",
      "| Subject/scope key | Baseline | Candidate | Delta | Relative delta | Unit | Regression |",
      "|---|---:|---:|---:|---:|---|---|",
      ...result.deltas.map(
        (delta) =>
          `| \`${delta.key}\` | ${delta.baseline} | ${delta.candidate} | ${delta.delta} | ${delta.relativeDelta ?? "n/a"} | ${delta.unit} | ${delta.regression === null ? "n/a" : delta.regression ? "yes" : "no"} |`,
      ),
    );
  } else if (result.state === "partial-collection") {
    lines.push(
      "",
      `- Missing from baseline: ${result.missingFromBaseline.join(", ") || "none"}`,
      `- Missing from candidate: ${result.missingFromCandidate.join(", ") || "none"}`,
      `- Sampling-count mismatches: ${result.samplingCountMismatches.map((item) => `${item.key} (${item.baseline} vs ${item.candidate})`).join(", ") || "none"}`,
    );
  } else if (result.state === "incompatible") {
    lines.push(
      "",
      ...result.incompatibilities.map(
        (issue) => `- \`${issue.key}\`: ${issue.reason}`,
      ),
    );
  }
  return `${lines.join("\n")}\n`;
}

function matches(record: MeasurementRecord, query: MeasurementQuery): boolean {
  return (
    (query.planId === undefined || record.plan.id === query.planId) &&
    (query.definitionVersion === undefined ||
      record.plan.definitionVersion === query.definitionVersion) &&
    (query.subject === undefined || equal(record.subject, query.subject)) &&
    (query.scope === undefined || equal(record.scope, query.scope)) &&
    (query.sourceRevision === undefined ||
      record.sourceRevision === query.sourceRevision)
  );
}

function collectionIncompatibilities(
  records: MeasurementRecord[],
  side: "baseline" | "candidate",
): MeasurementIncompatibility[] {
  const issues: MeasurementIncompatibility[] = [];
  const revisions = new Set(records.map((record) => record.sourceRevision));
  if (revisions.size > 1) {
    issues.push({ key: `<${side}>`, reason: "mixed-revision" });
  }
  const seen = new Set<string>();
  for (const record of records) {
    const key = measurementKey(record);
    if (seen.has(key)) {
      issues.push({ key, reason: "duplicate-subject" });
    }
    seen.add(key);
  }
  return issues;
}

function indexPopulation(
  records: MeasurementRecord[],
): Map<string, MeasurementRecord> {
  return new Map(records.map((record) => [measurementKey(record), record]));
}

function pairIncompatibilities(
  baseline: MeasurementRecord,
  candidate: MeasurementRecord,
  key: string,
): MeasurementIncompatibility[] {
  const issues: MeasurementIncompatibility[] = [];
  mismatch(issues, key, "plan-id", baseline.plan.id, candidate.plan.id);
  mismatch(
    issues,
    key,
    "definition-version",
    baseline.plan.definitionVersion,
    candidate.plan.definitionVersion,
  );
  mismatch(issues, key, "unit", baseline.unit, candidate.unit);
  mismatch(
    issues,
    key,
    "environment",
    baseline.environment.id,
    candidate.environment.id,
  );
  mismatch(
    issues,
    key,
    "sampling",
    baseline.sampling?.id,
    candidate.sampling?.id,
  );
  return issues;
}

function mismatch(
  issues: MeasurementIncompatibility[],
  key: string,
  reason: MeasurementIncompatibility["reason"],
  baseline: string | undefined,
  candidate: string | undefined,
): void {
  if (baseline === candidate) return;
  issues.push({ key, reason, baseline, candidate });
}

function deltaFor(
  baseline: MeasurementRecord,
  candidate: MeasurementRecord,
  key: string,
  policy?: MeasurementPolicy,
): MeasurementDelta {
  const delta = candidate.value - baseline.value;
  const relativeDelta = baseline.value === 0 ? null : delta / baseline.value;
  let regression: boolean | null = null;
  if (policy) {
    const adverse =
      policy.direction === "increase-is-regression" ? delta : -delta;
    const observed =
      policy.tolerance.kind === "absolute"
        ? adverse
        : adverse / Math.abs(baseline.value);
    regression = observed > policy.tolerance.value;
  }
  return {
    key,
    baseline: baseline.value,
    candidate: candidate.value,
    delta,
    relativeDelta,
    unit: baseline.unit,
    regression,
  };
}

function incompatible(
  incompatibilities: MeasurementIncompatibility[],
  requested: boolean,
): MeasurementComparison {
  return {
    state: "incompatible",
    incompatibilities: [...incompatibilities].sort(
      (a, b) =>
        compareStrings(a.key, b.key) || compareStrings(a.reason, b.reason),
    ),
    ratchet: { requested, baselineApplied: false },
  };
}

function validatePolicy(policy: MeasurementPolicy | undefined): void {
  if (
    policy &&
    (!Number.isFinite(policy.tolerance.value) || policy.tolerance.value < 0)
  ) {
    throw new Error(
      "measurement policy tolerance must be finite and non-negative",
    );
  }
}

function equal(a: unknown, b: unknown): boolean {
  return canonicalJson(a) === canonicalJson(b);
}

function byTimeRevisionKey(a: MeasurementRecord, b: MeasurementRecord): number {
  return (
    compareStrings(a.collectedAt, b.collectedAt) ||
    compareStrings(a.sourceRevision, b.sourceRevision) ||
    compareStrings(measurementKey(a), measurementKey(b))
  );
}

function compareStrings(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
