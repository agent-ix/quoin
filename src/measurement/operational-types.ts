import type { RawEvidenceReference } from "./intervention-types.js";

export type OperationalControlKind =
  | "release"
  | "feature_flag"
  | "canary_deployment"
  | "shadow_deployment"
  | "rollback"
  | "kill_switch"
  | "human_override"
  | "appeal"
  | "abstention"
  | "safe_fallback"
  | "policy_pin"
  | "prompt_pin"
  | "model_pin"
  | "tool_pin"
  | "data_pin"
  | "reporting";

export interface OperationalBase {
  schema_version: 1;
  record_type: "operational_evidence";
  record_id: string;
  observed_at: string;
  control_kind: OperationalControlKind;
  subject: { id: string; revision: string };
  producer: {
    tool_identity: string;
    tool_version: string;
    configuration_digest: string;
    source_revision: string;
    environment: Record<string, string | number | boolean | null>;
    definition_version: string;
  };
  scope: { service: string; environment: string; population: string };
  configuration: {
    version_pins: Array<{
      kind: "policy" | "prompt" | "model" | "tool" | "data";
      identity: string;
      revision: string;
      digest: string;
    }>;
  };
  owner: string;
  gaps: string[];
  actions: string[];
  raw_evidence: RawEvidenceReference[];
}

export interface StandingCapabilityRecord extends OperationalBase {
  record_shape: "standing_capability";
  capability: {
    control_id: string;
    status: "available" | "unavailable" | "unknown" | "not_applicable";
    surface: string;
    authorized_roles: string[];
    coverage: string;
    limitations: string[];
    supported_transitions: string[];
    clock_support:
      | { supported: false }
      | {
          supported: true;
          start_event: string;
          completion_event: string;
          deadline_seconds: number;
        };
  };
  exercise?: never;
}

export interface OperationalExerciseRecord extends OperationalBase {
  record_shape: "exercise";
  capability?: never;
  exercise: {
    control_id: string;
    capability_record_id?: string;
    mode: "actual" | "drill";
    started_at: string;
    completed_at: string;
    actor: string;
    trigger: string;
    outcome: "succeeded" | "failed" | "partial" | "aborted";
    state_before: Record<string, unknown>;
    state_after: Record<string, unknown>;
    observations: string[];
    clock:
      | { applicability: "not_applicable"; status: "not_applicable" }
      | {
          applicability: "operational_with_clock";
          started_at: string;
          deadline_at: string;
          completed_at?: string;
          status: "open" | "met" | "missed";
        };
  };
}

export type OperationalEvidenceRecord =
  StandingCapabilityRecord | OperationalExerciseRecord;

export interface OperationalObligation {
  control_kind: OperationalControlKind;
  subject: OperationalBase["subject"];
  scope: OperationalBase["scope"];
  accepted_modes: Array<"actual" | "drill">;
  clock: {
    applicability: "operational_with_clock";
    started_at: string;
    deadline_at: string;
  };
}

export interface GitHubReleaseProducerDefinition {
  record_prefix: string;
  workflow_path: string;
  release_job: string;
  accepted_event: string;
  control_id: string;
  subject: OperationalBase["subject"];
  producer: OperationalBase["producer"];
  scope: OperationalBase["scope"];
  configuration: OperationalBase["configuration"];
  supported_transition: string;
  authorized_roles: string[];
  coverage: string;
  limitations: string[];
  owner: string;
  gaps: string[];
  actions: string[];
  clock_deadline_seconds: number;
  workflow_evidence_path: string;
  run_evidence_path: string;
  jobs_evidence_path: string;
}
