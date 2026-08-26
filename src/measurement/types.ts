export const MEASUREMENT_SCHEMA_VERSION = 1;

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
  observations: MeasurementObservation[];
  /** Complete producer output; report views derive rather than transcribe. */
  rawEvidence: unknown;
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
