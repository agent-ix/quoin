import { createHash } from "node:crypto";

import {
  normalizeQuireFinding,
  normalizeQuoinFinding,
} from "../../evals/lib/finding-envelope.mjs";

export const TIER2_BASELINE_VERSION = "tier2-finding-quality-v2";

/** Retain raw producer output beside one cross-producer normalized view. */
export function retainTier2Sources(sources) {
  return Object.fromEntries(
    Object.entries(sources)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, source]) => [name, retainSource(name, source)]),
  );
}

export function createTier2Baseline({ provenance, cohorts, score }) {
  const retained = Object.fromEntries(
    Object.entries(cohorts)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([id, cohort]) => {
        const sources = retainTier2Sources(cohort.sources ?? {});
        const record = {
          provenance: cohort.provenance,
          sources,
          source_digest: digest(sources),
        };
        return [id, record];
      }),
  );
  return {
    schema_version: TIER2_BASELINE_VERSION,
    provenance,
    cohorts: retained,
    score,
    cohort_digest: digest(retained),
  };
}

/** Candidate comparison is read-only and keeps unavailable distinct from clean. */
export function compareTier2Baseline(previous, current) {
  if (previous?.schema_version !== TIER2_BASELINE_VERSION) {
    return {
      comparable: false,
      input_mismatches: ["baseline schema"],
      gained: [...(current.score?.detected ?? [])].sort(),
      lost: [],
      source_regressions: [],
      source_changes: [],
    };
  }
  const inputFields = [
    ["answer key", "answer_key_digest"],
    ["cohort manifest", "cohort_manifest_digest"],
    ["Quire binary", "tools.quire.digest"],
    ["Quoin build", "tools.quoin.digest"],
    ["Node", "environment.node"],
    ["platform", "environment.platform"],
    ["architecture", "environment.arch"],
  ];
  const inputMismatches = inputFields
    .filter(
      ([, path]) =>
        at(previous.provenance, path) !== at(current.provenance, path),
    )
    .map(([label]) => label);
  const before = new Set(previous.score?.detected ?? []);
  const after = new Set(current.score?.detected ?? []);
  const cohortIds = new Set([
    ...Object.keys(previous.cohorts ?? {}),
    ...Object.keys(current.cohorts ?? {}),
  ]);
  const sourceRegressions = [];
  const sourceChanges = [];
  for (const cohort of [...cohortIds].sort()) {
    const sourceNames = new Set([
      ...Object.keys(previous.cohorts?.[cohort]?.sources ?? {}),
      ...Object.keys(current.cohorts?.[cohort]?.sources ?? {}),
    ]);
    for (const name of [...sourceNames].sort()) {
      const prior = previous.cohorts?.[cohort]?.sources?.[name];
      const candidate = current.cohorts?.[cohort]?.sources?.[name];
      if (prior?.state === "evaluated" && candidate?.state !== "evaluated") {
        sourceRegressions.push({
          cohort,
          source: name,
          before: prior.state,
          after: candidate?.state ?? "missing",
          reason: candidate?.reason ?? "source missing from candidate",
        });
      }
      if (prior?.digest !== candidate?.digest) {
        sourceChanges.push({
          cohort,
          source: name,
          before: prior?.state ?? "missing",
          after: candidate?.state ?? "missing",
        });
      }
    }
  }
  return {
    comparable: inputMismatches.length === 0,
    input_mismatches: inputMismatches,
    gained: [...after].filter((id) => !before.has(id)).sort(),
    lost: [...before].filter((id) => !after.has(id)).sort(),
    source_regressions: sourceRegressions,
    source_changes: sourceChanges,
  };
}

function retainSource(name, source) {
  const state = source.state ?? (source.ok ? "evaluated" : "failed");
  const record = {
    state,
    command: source.command,
    ...(source.reason ? { reason: source.reason } : {}),
    raw: source.ok ? source.payload : null,
    normalized: source.ok
      ? normalizePayload(name, source.payload)
      : { state, reason: source.reason },
  };
  return { ...record, digest: digest(record) };
}

function normalizePayload(name, payload) {
  if (name === "quire.coverage") {
    const findings = [
      ...(payload.diagnostics ?? []).map((raw) =>
        normalizeQuireFinding(raw, {
          producer: "quire",
          channel: "coverage.diagnostics",
          family: raw.reason,
        }),
      ),
      ...(payload.suspicions ?? []).map((raw) =>
        normalizeQuireFinding(raw, {
          producer: "quire",
          channel: "coverage.suspicions",
          family: raw.kind,
        }),
      ),
    ];
    return {
      findings: sortFindings(findings),
      metrics: [...(payload.metrics ?? [])].sort((a, b) =>
        String(a.name).localeCompare(String(b.name)),
      ),
    };
  }
  const findings = (payload.findings ?? []).map((raw) =>
    normalizeQuoinFinding(raw, {
      producer: "quoin",
      channel:
        name === "quoin.validate" ? "validate.findings" : "evidence.findings",
      family: raw.kind,
    }),
  );
  return { findings: sortFindings(findings) };
}

function sortFindings(findings) {
  return findings.sort((a, b) => canonical(a).localeCompare(canonical(b)));
}

function digest(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function at(value, path) {
  return path.split(".").reduce((current, key) => current?.[key], value);
}
