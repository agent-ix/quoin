import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { parse as parseYaml } from "yaml";

function loadPlan(path) {
  const text = readFileSync(path, "utf8");
  const match = /^---\r?\n([\s\S]*?)\r?\n---/.exec(text);
  if (!match) throw new Error(`${path}: missing MeasurementPlan frontmatter`);
  const plan = parseYaml(match[1]);
  if (plan.type !== "MeasurementPlan") {
    throw new Error(`${path}: mapped metric does not name a MeasurementPlan`);
  }
  return plan;
}

export function createMeasurementRecord(report, at, options) {
  const {
    root,
    metricsPath,
    corpusMetricsPath,
    corpusPlanOverrides = {},
    sectionHitRate,
    execute,
    scorerDigest,
  } = options;
  const ownDictionary = JSON.parse(readFileSync(metricsPath, "utf8"));
  const corpusDictionary = corpusMetricsPath
    ? JSON.parse(readFileSync(corpusMetricsPath, "utf8"))
    : { metrics: {} };
  const dictionary = {
    metrics: { ...ownDictionary.metrics, ...corpusDictionary.metrics },
  };
  const plans = {
    ...Object.fromEntries(
      Object.entries(ownDictionary.metrics).map(([metric, declaration]) => [
        metric,
        loadPlan(join(root, declaration.measurement_plan)),
      ]),
    ),
    ...Object.fromEntries(
      Object.entries(corpusDictionary.metrics).map(([metric, declaration]) => [
        metric,
        loadPlan(
          join(
            root,
            corpusPlanOverrides[metric] ??
              join("corpus", declaration.measurement_plan),
          ),
        ),
      ]),
    ),
  };
  const revision = execute("git", ["-C", root, "rev-parse", "HEAD"]);
  const status = execute("git", ["-C", root, "status", "--porcelain"]);
  const sourceRevision = !revision.ok
    ? "unknown-source-revision"
    : !status.ok || status.stdout.trim() === ""
      ? revision.stdout.trim()
      : `${revision.stdout.trim()}+working-tree:${scorerDigest().slice(7, 23)}`;

  const observation = (metric, value, observationOptions = {}) => {
    const plan = plans[metric];
    if (!plan) throw new Error(`no MeasurementPlan mapped for ${metric}`);
    return {
      metric,
      planId: plan.id,
      definitionVersion: plan.definition_version,
      state: value === null ? "not_computed" : "measured",
      value,
      unit: observationOptions.unit ?? dictionary.metrics[metric].unit,
      shape: observationOptions.shape ?? "ratio",
      ...(observationOptions.population
        ? { population: observationOptions.population }
        : {}),
      ...(observationOptions.dimensions
        ? { dimensions: observationOptions.dimensions }
        : {}),
      ...(value === null
        ? {
            reason:
              observationOptions.reason ??
              "producer did not compute this metric",
          }
        : {}),
    };
  };

  const observations = [];
  for (const family of report.families) {
    const precisionPopulation = family.precision_basis ?? family;
    observations.push(
      observation("finding_precision", family.precision, {
        dimensions: { family: family.family },
        population: {
          examined:
            (precisionPopulation.truePositives ?? 0) +
            (precisionPopulation.falsePositives ?? 0),
          matched: precisionPopulation.truePositives ?? 0,
          complete: true,
          identity: {
            family: family.family,
            ...(family.precision_basis
              ? {
                  metricVersion: family.precision_basis.metricVersion,
                  retainedPopulation:
                    report.advisory_adjudication?.population?.digest,
                  ambiguous: family.precision_basis.ambiguous,
                  unresolved: family.precision_basis.unresolved,
                }
              : {}),
          },
        },
      }),
      observation("finding_recall", family.recall, {
        dimensions: { family: family.family },
        population: {
          examined: family.truePositives + family.misses,
          matched: family.truePositives,
          complete: true,
          identity: { family: family.family },
        },
      }),
      observation(
        "finding_precision.unadjudicated",
        family.precision_basis?.unadjudicated ?? 0,
        {
          shape: "count",
          dimensions: { family: family.family },
          population: {
            identity: {
              family: family.family,
              ...(family.precision_basis
                ? {
                    metricVersion: family.precision_basis.metricVersion,
                    retainedPopulation:
                      report.advisory_adjudication?.population?.digest,
                    unadjudicatedFindingIds:
                      family.precision_basis.unadjudicatedFindingIds,
                  }
                : {}),
            },
          },
        },
      ),
    );
  }

  observations.push(
    observation(
      "span_grounding_rate",
      report.span_grounding?.rate == null
        ? null
        : Number((report.span_grounding.rate * 100).toFixed(3)),
      {
        population: {
          examined: report.span_grounding?.denominator ?? 0,
          matched: report.span_grounding?.numerator ?? 0,
          complete: (report.span_grounding?.malformed?.length ?? 0) === 0,
          identity: {
            spanStates: report.span_grounding?.spanStates ?? {},
            exclusions: report.span_grounding?.exclusions ?? [],
            namedMisses: report.span_grounding?.namedMisses ?? [],
          },
        },
        dimensions: {
          producerVersions: (
            report.span_grounding?.producerVersions ?? []
          ).join(", "),
        },
        reason:
          report.span_grounding?.malformed?.length > 0
            ? "one or more properties payloads were malformed"
            : undefined,
      },
    ),
    observation(
      "span_correctness_rate",
      report.grounding_quality?.correctness?.rate == null
        ? null
        : Number((report.grounding_quality.correctness.rate * 100).toFixed(3)),
      {
        population: {
          examined: report.grounding_quality?.correctness?.denominator ?? 0,
          matched: report.grounding_quality?.correctness?.numerator ?? 0,
          complete: (report.grounding_quality?.malformed?.length ?? 0) === 0,
          identity: {
            namedMisses:
              report.grounding_quality?.correctness?.namedMisses ?? [],
            tradeoff: report.grounding_quality?.tradeoff ?? {},
          },
        },
        dimensions: {
          producerVersions: (
            report.grounding_quality?.producerVersions ?? []
          ).join(", "),
        },
      },
    ),
    observation(
      "span_safe_refusal_rate",
      report.grounding_quality?.safeRefusal?.rate == null
        ? null
        : Number((report.grounding_quality.safeRefusal.rate * 100).toFixed(3)),
      {
        population: {
          examined: report.grounding_quality?.safeRefusal?.denominator ?? 0,
          matched: report.grounding_quality?.safeRefusal?.numerator ?? 0,
          complete: (report.grounding_quality?.malformed?.length ?? 0) === 0,
          identity: {
            namedMisses:
              report.grounding_quality?.safeRefusal?.namedMisses ?? [],
            tradeoff: report.grounding_quality?.tradeoff ?? {},
          },
        },
        dimensions: {
          producerVersions: (
            report.grounding_quality?.producerVersions ?? []
          ).join(", "),
        },
      },
    ),
    observation(
      "actionability_rate",
      (report.actionability_v1 ?? report.actionability)?.rate == null
        ? null
        : Number(
            (
              (report.actionability_v1 ?? report.actionability).rate * 100
            ).toFixed(3),
          ),
      {
        population: {
          examined:
            (report.actionability_v1 ?? report.actionability)?.total ?? 0,
          matched:
            (report.actionability_v1 ?? report.actionability)?.actionable ?? 0,
          complete: true,
        },
      },
    ),
    observation(
      "actionability_v2_rate",
      report.actionability_v2?.rate == null
        ? null
        : Number((report.actionability_v2.rate * 100).toFixed(3)),
      {
        population: {
          examined: report.actionability_v2?.denominator ?? 0,
          matched: report.actionability_v2?.numerator ?? 0,
          complete: true,
          exclusions: report.actionability_v2?.exclusions ?? [],
          namedMisses: report.actionability_v2?.namedMisses ?? [],
        },
      },
    ),
    observation(
      "cost_per_confirmed_insight",
      report.cost_per_confirmed_insight?.toolCallsPer ?? null,
      {
        shape: "scalar",
        unit: "tool calls per confirmed insight",
        dimensions: { component: "tool_calls" },
      },
    ),
    observation(
      "cost_per_confirmed_insight",
      report.cost_per_confirmed_insight?.tokensPer ?? null,
      {
        shape: "scalar",
        unit: "tokens per confirmed insight",
        dimensions: { component: "tokens" },
        reason: "tier-1 calls no model",
      },
    ),
    observation(
      "sentinel.silent_zero",
      report["sentinel.silent_zero"]?.count ?? null,
      {
        shape: "count",
      },
    ),
    observation(
      "minting.section_hit_rate",
      report[sectionHitRate]?.rate ?? null,
      {
        population: report[sectionHitRate]
          ? {
              examined: report[sectionHitRate].examined,
              matched: report[sectionHitRate].matched,
              complete: true,
            }
          : undefined,
        reason: "engine emitted no section-hit metric",
      },
    ),
    observation("finding_localisation_rate", report.finding_localisation_rate, {
      population: {
        examined: report.families.reduce(
          (total, family) => total + family.truePositives,
          0,
        ),
        matched: report.positional,
        complete: true,
      },
    }),
  );
  for (const [axis, metric] of [
    ["correctness", "span_correctness_rate"],
    ["safeRefusal", "span_safe_refusal_rate"],
  ]) {
    for (const family of report.grounding_quality?.[axis]?.families ?? []) {
      observations.push(
        observation(
          metric,
          family.rate == null ? null : Number((family.rate * 100).toFixed(3)),
          {
            dimensions: { family: family.family },
            population: {
              examined: family.denominator,
              matched: family.numerator,
              complete: true,
              identity: { namedMisses: family.namedMisses },
            },
          },
        ),
      );
    }
  }
  for (const row of report.detection_recall ?? []) {
    observations.push(
      observation("detection.recall", row.rate, {
        dimensions: {
          level: row.level,
          mode: row.mode,
          language: row.language,
          family: row.family,
        },
        population: {
          examined: row.population,
          matched: row.reached,
          complete: true,
          identity: {
            level: row.level,
            mode: row.mode,
            language: row.language,
            family: row.family,
            misses: row.misses,
            exclusions: row.exclusions ?? [],
          },
        },
      }),
      observation("bounds.gap_count", row.gap_count, {
        shape: "count",
        dimensions: {
          beside: "detection.recall",
          level: row.level,
          mode: row.mode,
          language: row.language,
          family: row.family,
        },
      }),
    );
  }

  const collectionId = `tier1-${at.replace(/[^0-9]/g, "").slice(0, 17)}-${createHash(
    "sha256",
  )
    .update(`${sourceRevision}\0${report.provenance.corpus}\0${at}`)
    .digest("hex")
    .slice(0, 12)}`;

  return {
    schemaVersion: 1,
    collectionId,
    subject: "tier-1 controlled corpus",
    scope: {
      corpora: report.corpora,
      by_language: Object.fromEntries(
        (report.by_language ?? []).map((row) => [row.language, row.corpora]),
      ),
      findings: report.findings,
    },
    toolIdentity: "quoin bench-tier1",
    toolVersion: report.provenance.engine,
    configDigest:
      report.provenance.declaration?.digest ?? "missing-declaration-digest",
    corpusRevision: report.provenance.corpus,
    sourceRevision,
    timestamp: at,
    environment: {
      runner: "tier-1",
      scorerDigest: scorerDigest(),
      languages: (report.by_language ?? [])
        .map((row) => row.language)
        .sort()
        .join(","),
    },
    observations,
    rawEvidence: report,
  };
}

export function persistMeasurement(collection, root) {
  // A file avoids a large-stdin/Oclif diagnostic pipe deadlock (quoin#247).
  const scratch = mkdtempSync(join(tmpdir(), "quoin-measurement-"));
  const input = join(scratch, "collection.json");
  try {
    writeFileSync(input, JSON.stringify(collection));
    const output = execFileSync(
      process.execPath,
      [
        join(root, "bin", "quoin.js"),
        "measurement",
        "record",
        "--repo",
        root,
        "--input",
        input,
      ],
      {
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    return (
      output.trim() ||
      join(
        root,
        "spec",
        "evidence",
        "measurements",
        `${collection.collectionId}.json`,
      )
    );
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}
