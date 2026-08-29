import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

import {
  findingEnvelopeDigest,
  validateFindingEnvelope,
} from "../../evals/lib/finding-envelope.mjs";
import { ADVISORY_PRECISION_VERSION } from "../../evals/lib/quality.mjs";

export const ADVISORY_ADJUDICATION_SCHEMA = "tier1-advisory-adjudication-v1";
export const ADVISORY_METRIC_VERSION = ADVISORY_PRECISION_VERSION;
export const ADVISORY_RUBRIC_VERSION = "advisory-finding-rubric-v1";

const DISPOSITIONS = new Set([
  "correct",
  "incorrect",
  "ambiguous",
  "unresolved",
]);
const DEFECT_OWNERS = new Set(["product", "corpus", "none", "unresolved"]);

/** Read and fail closed on the retained per-finding adjudication record. */
export function loadAdvisoryAdjudication(
  path,
  expectedMetricVersion = ADVISORY_METRIC_VERSION,
) {
  const value = JSON.parse(readFileSync(path, "utf8"));
  return validateAdvisoryAdjudication(value, expectedMetricVersion);
}

export function validateAdvisoryAdjudication(
  value,
  expectedMetricVersion = ADVISORY_METRIC_VERSION,
) {
  if (value?.schema_version !== ADVISORY_ADJUDICATION_SCHEMA) {
    throw new Error(
      `advisory adjudication: unsupported schema ${JSON.stringify(value?.schema_version)}`,
    );
  }
  if (value.metric_version !== expectedMetricVersion) {
    throw new Error(
      `advisory adjudication: metric version ${JSON.stringify(value.metric_version)} is incompatible with ${JSON.stringify(expectedMetricVersion)}`,
    );
  }
  if (value.rubric?.version !== ADVISORY_RUBRIC_VERSION) {
    throw new Error(
      `advisory adjudication: unsupported rubric ${JSON.stringify(value.rubric?.version)}`,
    );
  }
  if (!nonBlank(value.population?.source?.revision)) {
    throw new Error("advisory adjudication: source revision is required");
  }
  if (!isDigest(value.population?.source?.report_digest)) {
    throw new Error("advisory adjudication: source report digest is required");
  }
  if (!Array.isArray(value.findings) || !Array.isArray(value.rulings)) {
    throw new Error("advisory adjudication: findings and rulings are required");
  }

  const findings = new Map();
  for (const entry of value.findings) {
    validateFindingEnvelope(entry?.finding);
    const digest = findingEnvelopeDigest(entry.finding);
    if (entry.id !== digest) {
      throw new Error(
        `advisory adjudication: finding id ${JSON.stringify(entry?.id)} does not match ${digest}`,
      );
    }
    if (findings.has(entry.id)) {
      throw new Error(`advisory adjudication: duplicate finding ${entry.id}`);
    }
    findings.set(entry.id, entry.finding);
  }
  const populationDigest = digestIds([...findings.keys()]);
  if (value.population.digest !== populationDigest) {
    throw new Error(
      `advisory adjudication: population digest ${JSON.stringify(value.population.digest)} does not match ${populationDigest}`,
    );
  }
  if (value.population.count !== findings.size) {
    throw new Error(
      `advisory adjudication: population count ${value.population.count} does not match ${findings.size}`,
    );
  }

  const rulings = new Map();
  for (const ruling of value.rulings) {
    if (!findings.has(ruling?.finding_id)) {
      throw new Error(
        `advisory adjudication: ruling names unknown finding ${JSON.stringify(ruling?.finding_id)}`,
      );
    }
    if (rulings.has(ruling.finding_id)) {
      throw new Error(
        `advisory adjudication: duplicate ruling ${ruling.finding_id}`,
      );
    }
    if (ruling.metric_version !== expectedMetricVersion) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} has incompatible metric version`,
      );
    }
    if (ruling.rubric_version !== value.rubric.version) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} has incompatible rubric version`,
      );
    }
    if (!DISPOSITIONS.has(ruling.disposition)) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} has invalid disposition`,
      );
    }
    if (!DEFECT_OWNERS.has(ruling.defect_owner)) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} has invalid defect owner`,
      );
    }
    if (
      (ruling.disposition === "incorrect" &&
        ruling.defect_owner !== "product") ||
      (["ambiguous", "unresolved"].includes(ruling.disposition) &&
        ruling.defect_owner !== "unresolved")
    ) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} has an incompatible disposition and defect owner`,
      );
    }
    if (
      ["product", "corpus"].includes(ruling.defect_owner) &&
      !nonBlank(ruling.follow_up)
    ) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} requires a defect follow-up`,
      );
    }
    if (!nonBlank(ruling.rationale) || !nonBlank(ruling.reviewer)) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} requires rationale and reviewer`,
      );
    }
    if (!Array.isArray(ruling.disagreements)) {
      throw new Error(
        `advisory adjudication: ruling ${ruling.finding_id} must retain disagreements`,
      );
    }
    rulings.set(ruling.finding_id, structuredClone(ruling));
  }
  if (rulings.size !== findings.size) {
    const missing = [...findings.keys()].filter((id) => !rulings.has(id));
    throw new Error(
      `advisory adjudication: ${missing.length} finding(s) have no retained ruling`,
    );
  }

  const byFamily = {};
  for (const [id, ruling] of rulings) {
    const finding = findings.get(id);
    const family = finding.identity?.family;
    if (!nonBlank(family)) {
      throw new Error(`advisory adjudication: finding ${id} has no family`);
    }
    byFamily[family] ??= [];
    byFamily[family].push({ id, ...ruling });
  }
  for (const rows of Object.values(byFamily)) {
    rows.sort((a, b) => a.id.localeCompare(b.id));
  }

  return {
    metricVersion: value.metric_version,
    rubricVersion: value.rubric.version,
    population: structuredClone(value.population),
    byFamily,
    counts: dispositionCounts([...rulings.values()]),
  };
}

/** Bind a validated ruling artifact to the exact retained source report. */
export function verifyAdvisoryAdjudicationSource(value, sourceBytes) {
  const loaded = validateAdvisoryAdjudication(value);
  const bytes = Buffer.isBuffer(sourceBytes)
    ? sourceBytes
    : Buffer.from(sourceBytes);
  const reportDigest = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
  if (loaded.population.source.report_digest !== reportDigest) {
    throw new Error("advisory adjudication: source report digest changed");
  }
  const report = JSON.parse(bytes.toString("utf8"));
  const sourceIds = new Set(
    (report.finding_records ?? []).map(findingEnvelopeDigest),
  );
  const missing = value.findings.filter((entry) => !sourceIds.has(entry.id));
  if (missing.length) {
    throw new Error(
      `advisory adjudication: ${missing.length} retained finding(s) are absent from the source report`,
    );
  }
  const declaredUnadjudicated = (report.families ?? [])
    .filter((family) => family.shape === "advisory")
    .reduce((total, family) => total + family.precision_basis.unadjudicated, 0);
  if (declaredUnadjudicated !== loaded.population.count) {
    throw new Error(
      `advisory adjudication: source report declares ${declaredUnadjudicated} unadjudicated finding(s), artifact retains ${loaded.population.count}`,
    );
  }
  return loaded;
}

export function digestIds(ids) {
  const hash = createHash("sha256");
  for (const id of [...ids].sort()) hash.update(`${id}\n`);
  return `sha256:${hash.digest("hex")}`;
}

function dispositionCounts(rulings) {
  const counts = {
    correct: 0,
    incorrect: 0,
    ambiguous: 0,
    unresolved: 0,
  };
  for (const ruling of rulings) counts[ruling.disposition] += 1;
  return counts;
}

function isDigest(value) {
  return typeof value === "string" && /^sha256:[0-9a-f]{64}$/.test(value);
}

function nonBlank(value) {
  return typeof value === "string" && value.trim().length > 0;
}
