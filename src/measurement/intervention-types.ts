export type InterventionDisposition =
  "controlled" | "uncontrolled" | "unknown" | "not_applicable";

export interface InterventionArm {
  id: string;
  population: string;
  sample_size: number;
  configuration: Record<string, unknown>;
}

export interface RawEvidenceReference {
  path: string;
  media_type: string;
  size_bytes: number;
  digest: string;
}

export interface InterventionExperimentRecord {
  schema_version: 1;
  record_type: "intervention_experiment";
  record_id: string;
  observed_at: string;
  subject: { id: string; revision: string };
  producer: {
    tool_identity: string;
    tool_version: string;
    configuration_digest: string;
    source_revision: string;
    environment: Record<string, string | number | boolean | null>;
    definition_version: string;
  };
  design: {
    kind: "repeated" | "randomized" | "factorial";
    repetitions: number;
    assignment: {
      method:
        | "not_applicable"
        | "deterministic"
        | "randomized"
        | "blocked_randomized";
      seed?: string;
    };
    sampling_conditions: string[];
  };
  baseline: InterventionArm;
  treatments: InterventionArm[];
  changed_variables: Array<{
    name: string;
    treatment_id: string;
    baseline_value: unknown;
    treatment_value: unknown;
  }>;
  held_constant: Array<{ name: string; value: unknown }>;
  measured_effects: Array<{
    treatment_id: string;
    metric: string;
    baseline_value: number | string | boolean | null;
    treatment_value: number | string | boolean | null;
    effect: number | string | null;
    unit: string;
  }>;
  interactions: Array<{
    description: string;
    disposition: InterventionDisposition;
  }>;
  confounders: Array<{
    description: string;
    disposition: InterventionDisposition;
  }>;
  status: "completed" | "failed" | "inconclusive";
  conclusion: {
    kind:
      | "causal_effect_established"
      | "no_effect_observed"
      | "cause_not_established";
    statement: string;
    attribution_confidence: "none" | "low" | "moderate" | "high";
  };
  gaps: string[];
  owner: string;
  actions: string[];
  raw_evidence: RawEvidenceReference[];
}

export interface AgentEvalInterventionDefinition {
  report_schema_version: string;
  cli_agent_evals_version: string;
  record_id: string;
  observed_at: string;
  subject: InterventionExperimentRecord["subject"];
  producer: InterventionExperimentRecord["producer"];
  design: InterventionExperimentRecord["design"];
  baseline: Omit<InterventionArm, "sample_size">;
  treatment: Omit<InterventionArm, "sample_size">;
  changed_variables: InterventionExperimentRecord["changed_variables"];
  held_constant: InterventionExperimentRecord["held_constant"];
  interactions: InterventionExperimentRecord["interactions"];
  confounders: InterventionExperimentRecord["confounders"];
  attribution_method?: string;
  owner: string;
  gaps: string[];
  actions: string[];
  baseline_evidence_path: string;
  treatment_evidence_path: string;
}

export type InterventionRefusalCode =
  | "invalid_record"
  | "raw_evidence_mismatch"
  | "governing_plan_absent"
  | "definition_mismatch"
  | "record_id_collision";

export class InterventionIntakeError extends Error {
  constructor(
    readonly code: InterventionRefusalCode,
    readonly findings: string[],
  ) {
    super(`${code}: ${findings.join("; ")}`);
    this.name = "InterventionIntakeError";
  }
}
