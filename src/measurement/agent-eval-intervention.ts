import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parseAgentEval } from "../evidence/adapters/agent-eval.js";
import { rawEvidenceFor, writeInterventionRecord } from "./intervention.js";
import type {
  AgentEvalInterventionDefinition,
  InterventionExperimentRecord,
} from "./intervention-types.js";

const IMMUTABLE_VERSION =
  /^(?:v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|(?:sha256|blake3):[a-f0-9]{64})$/;

export function produceAgentEvalIntervention(
  repo: string,
  definition: AgentEvalInterventionDefinition,
): { path: string; record: InterventionExperimentRecord } {
  validateDefinition(definition);
  const baselineRaw = readRetained(repo, definition.baseline_evidence_path);
  const treatmentRaw = readRetained(repo, definition.treatment_evidence_path);
  // Reuse the established FR-042 parser/refusal boundary before interpreting
  // scenario rates for the intervention-specific projection.
  parseAgentEval(baselineRaw);
  parseAgentEval(treatmentRaw);
  const baseline = reportFrom(baselineRaw, "baseline");
  const treatment = reportFrom(treatmentRaw, "treatment");
  const baselineIds = [...baseline.keys()].sort(compare);
  const treatmentIds = [...treatment.keys()].sort(compare);
  if (baselineIds.join("\0") !== treatmentIds.join("\0")) {
    throw new Error(
      `agent-eval scenario mismatch: baseline=[${baselineIds.join(", ")}], treatment=[${treatmentIds.join(", ")}]`,
    );
  }

  const qualifiers = [...definition.interactions, ...definition.confounders];
  const gaps = [...definition.gaps];
  if (
    [...baseline.values(), ...treatment.values()].some((item) => item.total < 2)
  ) {
    gaps.push(
      "agent-eval reports do not contain repeated samples for every scenario",
    );
  }
  if (
    qualifiers.some(
      (item) =>
        item.disposition === "uncontrolled" || item.disposition === "unknown",
    )
  ) {
    gaps.push(
      "one or more declared interactions or confounders are uncontrolled or unknown",
    );
  }
  if (!definition.attribution_method) {
    gaps.push("no justified attribution method was supplied");
  }

  const measuredEffects = baselineIds.map((id) => {
    const before = baseline.get(id) as ScenarioRate;
    const after = treatment.get(id) as ScenarioRate;
    return {
      treatment_id: definition.treatment.id,
      metric: scenarioMetric(id),
      baseline_value: before.rate,
      treatment_value: after.rate,
      effect: after.rate - before.rate,
      unit: "fraction",
    };
  });
  const observedDifference = measuredEffects.some((item) => item.effect !== 0);

  const record: InterventionExperimentRecord = {
    schema_version: 1,
    record_type: "intervention_experiment",
    record_id: definition.record_id,
    observed_at: definition.observed_at,
    subject: definition.subject,
    producer: {
      ...definition.producer,
      environment: {
        ...definition.producer.environment,
        cli_agent_evals_version: definition.cli_agent_evals_version,
        report_schema_version: definition.report_schema_version,
      },
    },
    design: definition.design,
    baseline: {
      ...definition.baseline,
      sample_size: sum([...baseline.values()].map((item) => item.total)),
    },
    treatments: [
      {
        ...definition.treatment,
        sample_size: sum([...treatment.values()].map((item) => item.total)),
      },
    ],
    changed_variables: definition.changed_variables,
    held_constant: definition.held_constant,
    measured_effects: measuredEffects,
    interactions: definition.interactions,
    confounders: definition.confounders,
    status: "completed",
    conclusion: {
      kind: "cause_not_established",
      statement: observedDifference
        ? "The retained agent-evaluation runs show observed pass-rate differences; this adapter does not establish causality."
        : "The retained agent-evaluation runs show no observed pass-rate difference; this adapter does not establish causality.",
      attribution_confidence: "none",
    },
    gaps: [...new Set(gaps)],
    owner: definition.owner,
    actions: definition.actions,
    raw_evidence: [
      rawEvidenceFor(
        repo,
        definition.baseline_evidence_path,
        "application/json",
      ),
      rawEvidenceFor(
        repo,
        definition.treatment_evidence_path,
        "application/json",
      ),
    ],
  };
  return { path: writeInterventionRecord(repo, record), record };
}

interface ScenarioRate {
  passed: number;
  total: number;
  rate: number;
}

function reportFrom(raw: string, label: string): Map<string, ScenarioRate> {
  const root = JSON.parse(raw) as { repeats?: unknown; results?: unknown };
  if (!Number.isInteger(root.repeats) || Number(root.repeats) < 1) {
    throw new Error(
      `${label} agent-eval report is unversioned or lacks positive repeats`,
    );
  }
  const out = new Map<string, ScenarioRate>();
  for (const [index, result] of (root.results as unknown[]).entries()) {
    if (!isRecord(result) || typeof result.id !== "string" || !result.id) {
      throw new Error(`${label} result ${index} lacks a scenario id`);
    }
    if (out.has(result.id)) {
      throw new Error(`${label} report repeats scenario id ${result.id}`);
    }
    const match = /^(\d+)\/(\d+)$/.exec(String(result.passRate ?? ""));
    if (!match)
      throw new Error(`${label} scenario ${result.id} has invalid passRate`);
    const passed = Number(match[1]);
    const total = Number(match[2]);
    if (total < 1 || passed > total || total !== root.repeats) {
      throw new Error(
        `${label} scenario ${result.id} has inconsistent sample count`,
      );
    }
    out.set(result.id, { passed, total, rate: passed / total });
  }
  if (out.size === 0) throw new Error(`${label} report contains no scenarios`);
  return out;
}

function validateDefinition(value: AgentEvalInterventionDefinition): void {
  if (!isRecord(value))
    throw new Error("producer definition must be an object");
  if (value.report_schema_version !== "cli-agent-evals-report-v1") {
    throw new Error(
      "producer definition requires report_schema_version cli-agent-evals-report-v1",
    );
  }
  if (!IMMUTABLE_VERSION.test(String(value.cli_agent_evals_version ?? ""))) {
    throw new Error(
      "producer definition requires an immutable cli_agent_evals_version",
    );
  }
  if (
    !value.baseline_evidence_path ||
    !value.treatment_evidence_path ||
    value.baseline_evidence_path === value.treatment_evidence_path
  ) {
    throw new Error(
      "producer definition requires distinct baseline and treatment evidence paths",
    );
  }
  if (!value.treatment?.id)
    throw new Error("producer definition requires a treatment id");
}

function readRetained(repo: string, path: string): string {
  // rawEvidenceFor performs normalized/root-confined/symlink-safe resolution.
  rawEvidenceFor(repo, path, "application/json");
  return readFileSync(join(repo, "spec", "evidence", path), "utf8");
}

function scenarioMetric(id: string): string {
  const safe = id.toLowerCase().replace(/[^a-z0-9._-]+/g, "-");
  return `agent-eval.${safe}.pass-rate`;
}

function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
