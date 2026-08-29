export const MEASUREMENT_SCHEMA_VERSION = 2;
export const HISTORICAL_MEASUREMENT_SCHEMA_VERSIONS = [1] as const;

export type MeasurementState = "measured" | "not_computed";
export type MeasurementShape = "scalar" | "ratio" | "count";

export interface MeasurementPopulation {
  examined?: number;
  matched?: number;
  complete?: boolean;
  identity?: unknown;
}

export interface MeasurementObservation {
  metric: string;
  planId: string;
  definitionVersion: string;
  state: MeasurementState;
  value: number | null;
  unit: string;
  shape: MeasurementShape;
  population?: MeasurementPopulation;
  dimensions?: Record<string, string>;
  reason?: string;
}

/** One producer invocation. All observations land atomically as this unit. */
export interface MeasurementCollection {
  schemaVersion: number;
  collectionId: string;
  subject: string;
  scope: unknown;
  toolIdentity: string;
  toolVersion: string;
  configDigest: string;
  timestamp: string;
  sourceRevision: string;
  corpusRevision?: string;
  environment: Record<string, string>;
  /** Required in v2: immutable identity of every input that produced evidence. */
  verificationStack?: VerificationStackAttestation;
  observations: MeasurementObservation[];
  /** Complete producer output; report views derive rather than transcribe. */
  rawEvidence: unknown;
}

export interface VerificationStackAttestation {
  schemaVersion: "verification-stack-attestation-v1";
  lockDigest: string;
  executableDigest: string;
  buildProfile: "release";
  toolchains: { node: string; rust: string; python: string };
  sources: Record<
    string,
    { revision: string; sourceState: "clean"; remote: string }
  >;
  capabilities: string[];
  artifacts: Record<string, string>;
}

export interface MeasurementPlan {
  id: string;
  title: string;
  status: "proposed" | "active" | "retired";
  stage:
    | "observe"
    | "baseline"
    | "branch-comparison"
    | "trend"
    | "ratchet"
    | "target"
    | "gate";
  metric: string;
  definitionVersion: string;
  path: string;
}

export interface ComparisonReason {
  code:
    | "definition_changed"
    | "configuration_changed"
    | "incomplete_population"
    | "population_changed"
    | "tool_changed";
  message: string;
  blocking: boolean;
}

export interface MeasurementComparison {
  metric: string;
  dimensions: Record<string, string>;
  before: number | null;
  after: number | null;
  delta: number | null;
  status: "comparable" | "incomparable" | "not_computed";
  reasons: ComparisonReason[];
}
